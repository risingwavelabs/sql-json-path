use serde_json::Number;

use crate::{
    ast::*,
    json::{ArrayRef, Cow, Json, JsonRef, ObjectRef},
};

pub type Result<T> = std::result::Result<T, Error>;

/// The error type returned when evaluating a JSON path.
#[non_exhaustive]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("jsonpath array accessor can only be applied to an array")]
    ArrayAccess,
    #[error("jsonpath member accessor can only be applied to an object")]
    MemberAccess,
    #[error("jsonpath array subscript is out of bounds")]
    ArrayIndexOutOfBounds,
    #[error("JSON object does not contain key \"{0}\"")]
    NoKey(Box<str>),
    #[error("operand of unary jsonpath operator {0} is not a numeric value")]
    UnaryOperandNotNumeric(UnaryOp),
    #[error("left operand of jsonpath operator {0} is not a single numeric value")]
    LeftOperandNotNumeric(BinaryOp),
    #[error("right operand of jsonpath operator {0} is not a single numeric value")]
    RightOperandNotNumeric(BinaryOp),
    #[error("jsonpath array subscript is not a single numeric value")]
    ArraySubscriptNotNumeric,
    #[error("could not find jsonpath variable \"{0}\"")]
    NoVariable(Box<str>),
    #[error("\"vars\" argument is not an object")]
    VarsNotObject,
    #[error("jsonpath item method .double() can only be applied to a string or numeric value")]
    DoubleTypeError,
    #[error("string argument of jsonpath item method .double() is not a valid representation of a double precision number")]
    InvalidDouble,
    #[error("LAST is allowed only in array subscripts")]
    UnexpectedLast,
}

/// Truth value used in SQL/JSON path predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    True,
    False,
    Unknown,
}

impl From<bool> for Truth {
    fn from(b: bool) -> Self {
        if b {
            Truth::True
        } else {
            Truth::False
        }
    }
}

impl Truth {
    /// Returns true if the value is true.
    fn is_true(self) -> bool {
        matches!(self, Truth::True)
    }

    /// Returns true if the value is false.
    #[allow(unused)]
    fn is_false(self) -> bool {
        matches!(self, Truth::False)
    }

    /// Returns true if the value is unknown.
    fn is_unknown(self) -> bool {
        matches!(self, Truth::Unknown)
    }

    /// AND operation.
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Truth::True, Truth::True) => Truth::True,
            (Truth::False, _) | (_, Truth::False) => Truth::False,
            _ => Truth::Unknown,
        }
    }

    /// OR operation.
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Truth::True, _) | (_, Truth::True) => Truth::True,
            (Truth::False, Truth::False) => Truth::False,
            _ => Truth::Unknown,
        }
    }

    /// NOT operation.
    fn not(self) -> Self {
        match self {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        }
    }

    /// Converts to JSON value.
    fn to_json<T: Json>(self) -> T {
        match self {
            Truth::True => T::bool(true),
            Truth::False => T::bool(false),
            Truth::Unknown => T::null(),
        }
    }
}

impl JsonPath {
    /// Evaluate the JSON path against the given JSON value.
    pub fn query<'a, T: Json>(&self, value: T::Borrowed<'a>) -> Result<Vec<Cow<'a, T>>> {
        self.query_with_vars(value, T::Borrowed::null())
    }

    /// Evaluate the JSON path against the given JSON value with variables.
    pub fn query_with_vars<'a, T: Json>(
        &self,
        value: T::Borrowed<'a>,
        vars: T::Borrowed<'a>,
    ) -> Result<Vec<Cow<'a, T>>> {
        Evaluator {
            root: value,
            current: value,
            vars,
            array: T::Borrowed::null(),
            mode: self.mode,
        }
        .eval_expr_or_predicate(&self.expr)
    }
}

/// Evaluation context.
#[derive(Debug, Clone, Copy)]
struct Evaluator<'a, T: Json> {
    /// The current value referenced by `@`.
    current: T::Borrowed<'a>,
    /// The root value referenced by `$`.
    root: T::Borrowed<'a>,
    /// The innermost array value referenced by `last`.
    array: T::Borrowed<'a>,
    /// An object containing the variables referenced by `$var`.
    vars: T::Borrowed<'a>,
    /// The path mode.
    /// If the query is in lax mode, then errors are ignored and the result is empty or unknown.
    mode: Mode,
}

/// Unwrap the result or return an empty result if the evaluator is in lax mode.
macro_rules! lax {
    // for `Option`
    ($self:expr, $expr:expr, $err:expr) => {
        match $expr {
            Some(x) => x,
            None if $self.is_lax() => return Ok(vec![]),
            None => return Err($err),
        }
    };
    // for `Result`
    ($self:expr, $expr:expr) => {
        match $expr {
            Ok(x) => x,
            Err(_) if $self.is_lax() => return Ok(vec![]),
            Err(e) => return Err(e),
        }
    };
}

impl<'a, T: Json> Evaluator<'a, T> {
    /// Returns true if the evaluator is in lax mode.
    fn is_lax(&self) -> bool {
        matches!(self.mode, Mode::Lax)
    }

    /// Creates a new evaluator with the given current value.
    fn with_current<'b>(&self, current: T::Borrowed<'b>) -> Evaluator<'b, T>
    where
        'a: 'b,
    {
        Evaluator {
            current,
            root: T::borrow(self.root),
            vars: T::borrow(self.vars),
            array: T::borrow(self.array),
            mode: self.mode,
        }
    }

    /// Returns the value of the given variable.
    fn get_variable(&self, name: &str) -> Result<T::Borrowed<'a>> {
        self.vars
            .as_object()
            .ok_or_else(|| Error::VarsNotObject)?
            .get(name)
            .ok_or_else(|| Error::NoVariable(name.into()))
    }

    /// Evaluates the expression or predicate.
    fn eval_expr_or_predicate(&self, expr: &ExprOrPredicate) -> Result<Vec<Cow<'a, T>>> {
        match expr {
            ExprOrPredicate::Expr(expr) => self.eval_expr(expr),
            ExprOrPredicate::Pred(pred) => self
                .eval_predicate(pred)
                .map(|t| vec![Cow::Owned(t.to_json())]),
        }
    }

    /// Evaluates the predicate.
    fn eval_predicate(&self, pred: &Predicate) -> Result<Truth> {
        match pred {
            Predicate::Compare(op, left, right) => {
                let Ok(left) = self.eval_expr(left) else {
                    return Ok(Truth::Unknown);
                };
                let Ok(right) = self.eval_expr(right) else {
                    return Ok(Truth::Unknown);
                };

                let mut any_unknown = false;
                let mut any_true = false;
                // The cross product of these SQL/JSON sequences is formed.
                // Each SQL/JSON item in one SQL/JSON sequence is compared to each item in the other SQL/JSON sequence.
                for l in left.iter() {
                    for r in right.iter() {
                        let res = eval_compare::<T>(*op, l.as_ref(), r.as_ref());
                        // The predicate is Unknown if there any pair of SQL/JSON items in the cross product is not comparable.
                        if res.is_unknown() {
                            // In lax mode, the path engine is permitted to stop evaluation early if it detects either an error or a success.
                            if self.is_lax() {
                                return Ok(Truth::Unknown);
                            }
                            any_unknown = true;
                        }
                        // the predicate is True if any pair is comparable and satisfies the comparison operator.
                        if res.is_true() {
                            // In lax mode, the path engine is permitted to stop evaluation early if it detects either an error or a success.
                            if self.is_lax() {
                                return Ok(Truth::True);
                            }
                            any_true = true;
                        }
                        // In strict mode, the path engine must test all comparisons in the cross product.
                    }
                }
                Ok(if any_unknown {
                    // The predicate is Unknown if there any pair of SQL/JSON items in the cross product is not comparable.
                    Truth::Unknown
                } else if any_true {
                    // the predicate is True if any pair is comparable and satisfies the comparison operator.
                    Truth::True
                } else {
                    // In all other cases, the predicate is False.
                    Truth::False
                })
            }
            Predicate::Exists(expr) => {
                let Ok(set) = self.eval_expr(expr) else {
                    // If the result of the path expression is an error, then result is Unknown.
                    return Ok(Truth::Unknown);
                };
                // If the result of the path expression is an empty SQL/JSON sequence, then result is False.
                // Otherwise, result is True.
                Ok(Truth::from(!set.is_empty()))
            }
            Predicate::And(left, right) => {
                let left = self.eval_predicate(left)?;
                let right = self.eval_predicate(right)?;
                Ok(left.and(right))
            }
            Predicate::Or(left, right) => {
                let left = self.eval_predicate(left)?;
                let right = self.eval_predicate(right)?;
                Ok(left.or(right))
            }
            Predicate::Not(inner) => {
                let inner = self.eval_predicate(inner)?;
                Ok(inner.not())
            }
            Predicate::IsUnknown(inner) => {
                let inner = self.eval_predicate(inner)?;
                Ok(Truth::from(inner.is_unknown()))
            }
            Predicate::StartsWith(expr, prefix) => {
                let Ok(set) = self.eval_expr(expr) else {
                    return Ok(Truth::Unknown);
                };
                let prefix = self.eval_value(prefix)?;
                let prefix = prefix.as_ref().as_str().unwrap();
                let mut result = Truth::False;
                for v in set {
                    let res = v.as_ref().as_str().map_or(false, |s| s.starts_with(prefix));
                    result = result.or(res.into());
                    if result.is_true() {
                        break;
                    }
                }
                Ok(result)
            }
            Predicate::LikeRegex(expr, regex) => {
                let Ok(set) = self.eval_expr(expr) else {
                    return Ok(Truth::Unknown);
                };
                let mut result = Truth::False;
                for v in set {
                    let res = v.as_ref().as_str().map_or(false, |s| regex.is_match(s));
                    result = result.or(res.into());
                    if result.is_true() {
                        break;
                    }
                }
                Ok(result)
            }
        }
    }

    /// Evaluates the expression.
    fn eval_expr(&self, expr: &Expr) -> Result<Vec<Cow<'a, T>>> {
        match expr {
            Expr::Accessor(primary, ops) => {
                let mut set = self.eval_path_primary(primary)?;
                let mut new_set = vec![];
                for op in ops {
                    for v in &set {
                        match v {
                            Cow::Owned(v) => {
                                let sset = self.with_current(v.as_ref()).eval_accessor_op(op)?;
                                new_set.extend(
                                    // the returned set requires lifetime 'a,
                                    // however, elements in `sset` only have lifetime 'b < 'v = 'set < 'a
                                    // therefore, we need to convert them to owned values
                                    sset.into_iter().map(|cow| Cow::Owned(cow.into_owned())),
                                )
                            }
                            Cow::Borrowed(v) => {
                                new_set.extend(self.with_current(*v).eval_accessor_op(op)?);
                            }
                        }
                    }
                    std::mem::swap(&mut set, &mut new_set);
                    new_set.clear();
                }
                Ok(set)
            }
            Expr::UnaryOp(op, expr) => {
                let set = self.eval_expr(expr)?;
                let mut new_set = Vec::with_capacity(set.len());
                for v in set {
                    new_set.push(Cow::Owned(eval_unary_op(*op, v.as_ref())?));
                }
                Ok(new_set)
            }
            Expr::BinaryOp(op, left, right) => {
                let left = self.eval_expr(left)?;
                let right = self.eval_expr(right)?;
                if left.len() != 1 {
                    return Err(Error::LeftOperandNotNumeric(*op));
                }
                if right.len() != 1 {
                    return Err(Error::RightOperandNotNumeric(*op));
                }
                Ok(vec![Cow::Owned(eval_binary_op(
                    *op,
                    left[0].as_ref(),
                    right[0].as_ref(),
                )?)])
            }
        }
    }

    /// Evaluates the path primary.
    fn eval_path_primary(&self, primary: &PathPrimary) -> Result<Vec<Cow<'a, T>>> {
        match primary {
            PathPrimary::Root => Ok(vec![Cow::Borrowed(self.root.clone())]),
            PathPrimary::Current => Ok(vec![Cow::Borrowed(self.current.clone())]),
            PathPrimary::Value(v) => Ok(vec![self.eval_value(v)?]),
            PathPrimary::Last => {
                let array = self.array.as_array().ok_or_else(|| Error::UnexpectedLast)?;
                Ok(vec![Cow::Owned(T::from_i64(array.len() as i64 - 1))])
            }
            PathPrimary::ExprOrPred(expr) => self.eval_expr_or_predicate(expr),
        }
    }

    /// Evaluates the accessor operator.
    fn eval_accessor_op(&self, op: &AccessorOp) -> Result<Vec<Cow<'a, T>>> {
        match op {
            AccessorOp::MemberWildcard => {
                let object = lax!(self, self.current.as_object(), Error::MemberAccess);
                Ok(object.list_value().into_iter().map(Cow::Borrowed).collect())
            }
            AccessorOp::ElementWildcard => {
                let array = lax!(self, self.current.as_array(), Error::ArrayAccess);
                Ok(array.list().into_iter().map(Cow::Borrowed).collect())
            }
            AccessorOp::Member(name) => {
                let object = lax!(self, self.current.as_object(), Error::MemberAccess);
                let member = lax!(self, object.get(name), Error::NoKey(name.clone().into()));
                Ok(vec![Cow::Borrowed(member)])
            }
            AccessorOp::Element(indices) => self.eval_element_accessor(indices),
            AccessorOp::FilterExpr(pred) => {
                if self.eval_predicate(pred)?.is_true() {
                    Ok(vec![Cow::Borrowed(self.current.clone())])
                } else {
                    Ok(vec![])
                }
            }
            AccessorOp::Method(method) => self.eval_method(method).map(|v| vec![v]),
        }
    }

    /// Evaluates the element accessor.
    fn eval_element_accessor(&self, indices: &[ArrayIndex]) -> Result<Vec<Cow<'a, T>>> {
        let array = lax!(self, self.current.as_array(), Error::ArrayAccess);
        let mut elems = Vec::with_capacity(indices.len());
        for index in indices {
            let eval_index = |expr: &Expr| {
                // errors in this closure can not be ignored
                let set = Self {
                    // update `array` context
                    array: self.current,
                    ..*self
                }
                .eval_expr(expr)?;
                if set.len() != 1 {
                    return Err(Error::ArraySubscriptNotNumeric);
                }
                set[0]
                    .as_ref()
                    .as_number()
                    .ok_or_else(|| Error::ArraySubscriptNotNumeric)
            };
            match index {
                ArrayIndex::Index(expr) => {
                    let index_number = eval_index(expr)?;
                    let index =
                        lax!(self, index_number.as_u64(), Error::ArrayIndexOutOfBounds) as usize;
                    let elem = lax!(self, array.get(index), Error::ArrayIndexOutOfBounds);
                    elems.push(Cow::Borrowed(elem));
                }
                ArrayIndex::Slice(begin, end) => {
                    let begin_number = eval_index(begin)?;
                    let end_number = eval_index(end)?;
                    let begin =
                        lax!(self, begin_number.as_u64(), Error::ArrayIndexOutOfBounds) as usize;
                    let end =
                        lax!(self, end_number.as_u64(), Error::ArrayIndexOutOfBounds) as usize;
                    lax!(
                        self,
                        (begin <= end && end < array.len()).then_some(()),
                        Error::ArrayIndexOutOfBounds
                    );
                    elems.extend((begin..=end).map(|i| Cow::Borrowed(array.get(i).unwrap())));
                }
            }
        }
        Ok(elems)
    }

    /// Evaluates the item method.
    fn eval_method(&self, method: &Method) -> Result<Cow<'a, T>> {
        match method {
            Method::Type => {
                let s = if self.current.is_null() {
                    "null"
                } else if self.current.is_bool() {
                    "boolean"
                } else if self.current.is_number() {
                    "number"
                } else if self.current.is_string() {
                    "string"
                } else if self.current.is_array() {
                    "array"
                } else if self.current.is_object() {
                    "object"
                } else {
                    unreachable!()
                };
                Ok(Cow::Owned(T::from_string(s)))
            }
            Method::Size => {
                let size = if let Some(array) = self.current.as_array() {
                    // The size of an SQL/JSON array is the number of elements in the array.
                    array.len()
                } else {
                    // The size of an SQL/JSON object or a scalar is 1.
                    1
                };
                Ok(Cow::Owned(T::from_u64(size as u64)))
            }
            Method::Double => {
                if let Some(s) = self.current.as_str() {
                    let n = s.parse::<f64>().map_err(|_| Error::InvalidDouble)?;
                    Ok(Cow::Owned(T::from_f64(n)))
                } else if self.current.is_number() {
                    Ok(Cow::Borrowed(self.current.clone()))
                } else {
                    Err(Error::DoubleTypeError)
                }
            }
            Method::Ceiling => todo!(),
            Method::Floor => todo!(),
            Method::Abs => todo!(),
            Method::Keyvalue => todo!(),
        }
    }

    /// Evaluates the scalar value.
    fn eval_value(&self, value: &Value) -> Result<Cow<'a, T>> {
        Ok(match value {
            Value::Null => Cow::Owned(T::null()),
            Value::Boolean(b) => Cow::Owned(T::bool(*b)),
            Value::Number(n) => Cow::Owned(T::from_number(n.clone())),
            Value::String(s) => Cow::Owned(T::from_string(s)),
            Value::Variable(v) => Cow::Borrowed(self.get_variable(v)?),
        })
    }
}

/// Compare two values.
///
/// Return unknown if the values are not comparable.
fn eval_compare<T: Json>(op: CompareOp, left: T::Borrowed<'_>, right: T::Borrowed<'_>) -> Truth {
    use CompareOp::*;
    if left.is_null() && right.is_null() {
        return compare_ord(op, (), ()).into();
    }
    if left.is_null() || right.is_null() {
        return false.into();
    }
    if let (Some(left), Some(right)) = (left.as_bool(), right.as_bool()) {
        return compare_ord(op, left, right).into();
    }
    if let (Some(left), Some(right)) = (left.as_number(), right.as_number()) {
        return match op {
            Eq => left.equal(&right),
            Ne => !left.equal(&right),
            Gt => right.less_than(&left),
            Ge => !left.less_than(&right),
            Lt => left.less_than(&right),
            Le => !right.less_than(&left),
        }
        .into();
    }
    if let (Some(left), Some(right)) = (left.as_str(), right.as_str()) {
        return compare_ord(op, left, right).into();
    }
    // others (include arrays and objects) are not comparable
    Truth::Unknown
}

/// Evaluate the unary operator.
fn eval_unary_op<T: Json>(op: UnaryOp, value: T::Borrowed<'_>) -> Result<T> {
    let n = value
        .as_number()
        .ok_or_else(|| Error::UnaryOperandNotNumeric(op))?;
    Ok(match op {
        UnaryOp::Plus => value.to_owned(),
        UnaryOp::Minus => T::from_number(n.neg()),
    })
}

/// Evaluate the binary operator.
fn eval_binary_op<T: Json>(
    op: BinaryOp,
    left: T::Borrowed<'_>,
    right: T::Borrowed<'_>,
) -> Result<T> {
    let left = left
        .as_number()
        .ok_or_else(|| Error::LeftOperandNotNumeric(op))?;
    let right = right
        .as_number()
        .ok_or_else(|| Error::RightOperandNotNumeric(op))?;
    Ok(T::from_number(match op {
        BinaryOp::Add => left.add(&right),
        BinaryOp::Sub => left.sub(&right),
        BinaryOp::Mul => left.mul(&right),
        BinaryOp::Div => left.div(&right),
        BinaryOp::Rem => left.rem(&right),
    }))
}

/// Compare two values that implement `Ord`.
fn compare_ord<T: Ord>(op: CompareOp, left: T, right: T) -> bool {
    use CompareOp::*;
    match op {
        Eq => left == right,
        Ne => left != right,
        Gt => left > right,
        Ge => left >= right,
        Lt => left < right,
        Le => left <= right,
    }
}

/// Extension methods for `Number`.
trait NumberExt {
    fn equal(&self, other: &Self) -> bool;
    fn less_than(&self, other: &Self) -> bool;
    fn neg(&self) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn div(&self, other: &Self) -> Self;
    fn rem(&self, other: &Self) -> Self;
}

impl NumberExt for Number {
    fn equal(&self, other: &Self) -> bool {
        // The original `Eq` implementation of `Number` does not work
        // if the two numbers have different types. (i64, u64, f64)
        self.as_f64().unwrap() == other.as_f64().unwrap()
    }

    fn less_than(&self, other: &Self) -> bool {
        self.as_f64().unwrap() < other.as_f64().unwrap()
    }

    fn neg(&self) -> Self {
        if let Some(n) = self.as_i64() {
            Number::from(-n)
        } else if let Some(n) = self.as_f64() {
            Number::from_f64(-n).unwrap()
        } else {
            // `as_f64` should always return a value
            unreachable!()
        }
    }

    fn add(&self, other: &Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_i64(), other.as_i64()) {
            Number::from(a + b)
        } else if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            Number::from_f64(a + b).unwrap()
        } else {
            unreachable!()
        }
    }

    fn sub(&self, other: &Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_i64(), other.as_i64()) {
            Number::from(a - b)
        } else if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            Number::from_f64(a - b).unwrap()
        } else {
            unreachable!()
        }
    }

    fn mul(&self, other: &Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_i64(), other.as_i64()) {
            Number::from(a * b)
        } else if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            Number::from_f64(a * b).unwrap()
        } else {
            unreachable!()
        }
    }

    fn div(&self, other: &Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_i64(), other.as_i64()) {
            Number::from(a / b)
        } else if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            Number::from_f64(a / b).unwrap()
        } else {
            unreachable!()
        }
    }

    fn rem(&self, other: &Self) -> Self {
        if let (Some(a), Some(b)) = (self.as_i64(), other.as_i64()) {
            Number::from(a % b)
        } else if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            Number::from_f64(a % b).unwrap()
        } else {
            unreachable!()
        }
    }
}
