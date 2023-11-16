use serde_json::Number;

use crate::{
    json::{ArrayRef, Json, JsonRef, ObjectRef},
    node::*,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("jsonpath array accessor can only be applied to an array")]
    ArrayAccess,
    #[error("jsonpath member accessor can only be applied to an object")]
    MemberAccess,
    #[error("jsonpath array subscript is out of bounds")]
    ArrayIndexOutOfBounds,
    #[error("JSON object does not contain key \"{0}\"")]
    NoKey(String),
    #[error("left operand of jsonpath operator {0} is not a single numeric value")]
    LeftOperandNotNumeric(String),
    #[error("right operand of jsonpath operator {0} is not a single numeric value")]
    RightOperandNotNumeric(String),
    #[error("could not find jsonpath variable \"{0}\"")]
    NoVariable(String),
    #[error("\"vars\" argument is not an object")]
    VarsNotObject,
    #[error("jsonpath item method .double() can only be applied to a string or numeric value")]
    DoubleTypeError,
    #[error("string argument of jsonpath item method .double() is not a valid representation of a double precision number")]
    InvalidDouble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Truth {
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
    pub fn is_true(self) -> bool {
        matches!(self, Truth::True)
    }

    pub fn is_false(self) -> bool {
        matches!(self, Truth::False)
    }

    pub fn is_unknown(self) -> bool {
        matches!(self, Truth::Unknown)
    }

    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Truth::True, Truth::True) => Truth::True,
            (Truth::False, _) | (_, Truth::False) => Truth::False,
            _ => Truth::Unknown,
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Truth::True, _) | (_, Truth::True) => Truth::True,
            (Truth::False, Truth::False) => Truth::False,
            _ => Truth::Unknown,
        }
    }

    fn not(self) -> Self {
        match self {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        }
    }

    fn to_json<T: JsonRef>(self) -> T {
        match self {
            Truth::True => T::bool(true),
            Truth::False => T::bool(false),
            Truth::Unknown => T::null(),
        }
    }
}

impl JsonPath {
    pub fn query<T: JsonRef>(&self, value: T) -> Result<Vec<T>, Error> {
        Query {
            root: value,
            current: value,
            vars: T::null(),
            mode: self.mode,
        }
        .query_expr_or_predicate(&self.expr)
    }
}

struct Query<T> {
    current: T,
    root: T,
    vars: T,
    // If the query is in lax mode, then errors are ignored and the result is empty or unknown.
    mode: Mode,
}

impl<T: JsonRef> Query<T> {
    fn lax<O: Default>(&self, result: Result<O, Error>) -> Result<O, Error> {
        match self.mode {
            Mode::Lax => Ok(O::default()),
            _ => result,
        }
    }

    fn is_lax(&self) -> bool {
        matches!(self.mode, Mode::Lax)
    }

    fn with_current(&self, current: T) -> Self {
        Self {
            current: current,
            root: self.root,
            vars: self.vars,
            mode: self.mode,
        }
    }

    fn get_variable(&self, name: &str) -> Result<T, Error> {
        self.vars
            .as_object()
            .ok_or_else(|| Error::VarsNotObject)?
            .get(name)
            .ok_or_else(|| Error::NoVariable(name.to_string()))
    }

    fn query_expr_or_predicate(&self, expr: &ExprOrPredicate) -> Result<Vec<T>, Error> {
        match expr {
            ExprOrPredicate::Expr(expr) => self.query_expr(expr),
            ExprOrPredicate::Pred(pred) => self.query_predicate(pred).map(|t| vec![t.to_json()]),
        }
    }

    fn query_predicate(&self, pred: &Predicate) -> Result<Truth, Error> {
        match pred {
            Predicate::Compare { op, left, right } => {
                let Ok(left) = self.query_expr(left) else {
                    return Ok(Truth::Unknown);
                };
                let Ok(right) = self.query_expr(right) else {
                    return Ok(Truth::Unknown);
                };

                let mut any_unknown = false;
                let mut any_true = false;
                // The cross product of these SQL/JSON sequences is formed.
                // Each SQL/JSON item in one SQL/JSON sequence is compared to each item in the other SQL/JSON sequence.
                for l in left {
                    for r in right {
                        let res = self.compare(*op, l, r);
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
                            any_unknown = true;
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
                let Ok(set) = self.query_expr(expr) else {
                    // If the result of the path expression is an error, then result is Unknown.
                    return Ok(Truth::Unknown);
                };
                // If the result of the path expression is an empty SQL/JSON sequence, then result is False.
                // Otherwise, result is True.
                Ok(Truth::from(!set.is_empty()))
            }
            Predicate::And(left, right) => {
                let left = self.query_predicate(left)?;
                let right = self.query_predicate(right)?;
                Ok(left.and(right))
            }
            Predicate::Or(left, right) => {
                let left = self.query_predicate(left)?;
                let right = self.query_predicate(right)?;
                Ok(left.or(right))
            }
            Predicate::Not(inner) => {
                let inner = self.query_predicate(inner)?;
                Ok(inner.not())
            }
            Predicate::IsUnknown(inner) => {
                let inner = self.query_predicate(inner)?;
                Ok(Truth::from(inner.is_unknown()))
            }
            Predicate::StartsWith(expr, prefix) => {
                let Ok(set) = self.query_expr(expr) else {
                    return Ok(Truth::Unknown);
                };
                let prefix = self.query_value(prefix)?.as_str().unwrap();
                let mut result = Truth::False;
                for v in set {
                    let res = v.as_str().map_or(false, |s| s.starts_with(prefix));
                    result = result.or(res.into());
                    if result.is_true() {
                        break;
                    }
                }
                Ok(result)
            }
        }
    }

    fn query_expr(&self, expr: &Expr) -> Result<Vec<T>, Error> {
        match expr {
            Expr::Accessor(primary, ops) => {
                let mut set = self.query_path_primary(primary);
                for op in ops {
                    let new_set = vec![];
                    for v in set {
                        new_set.extend(self.with_current(v).query_path_op(op)?);
                    }
                    set = new_set;
                }
                Ok(set)
            }
            Expr::UnaryOp { op, expr } => {
                let mut set = self.query_expr(expr)?;
                for v in &mut set {
                    *v = self.query_unary_op(op, *v)?;
                }
                Ok(set)
            }
            Expr::BinaryOp { op, left, right } => {
                let left = self.query_expr(left)?;
                let right = self.query_expr(right)?;
                if left.len() != 1 {
                    return Err(Error::LeftOperandNotNumeric(op.to_string()));
                }
                if right.len() != 1 {
                    return Err(Error::RightOperandNotNumeric(op.to_string()));
                }
                Ok(vec![self.query_binary_op(op, left[0], right[0])?])
            }
        }
    }

    fn query_path_primary(&self, primary: &PathPrimary) -> Result<T, Error> {
        match primary {
            PathPrimary::Root => Ok(self.root.clone()),
            PathPrimary::Current => Ok(self.current.clone()),
            PathPrimary::Value(v) => self.query_value(v),
        }
    }

    fn query_accessor_op(&self, op: &AccessorOp) -> Result<Vec<T>, Error> {
        match op {
            AccessorOp::MemberWildcard => Ok(self
                .current
                .as_object()
                .ok_or_else(|| Error::MemberAccess)?
                .list_value()),
            AccessorOp::ElementWildcard => Ok(self
                .current
                .as_array()
                .ok_or_else(|| Error::ArrayAccess)?
                .list()),
            AccessorOp::Member(name) => self
                .current
                .as_object()
                .ok_or_else(|| Error::MemberAccess)?
                .get(name)
                .ok_or_else(|| Error::NoKey(name.to_string()))
                .map(|v| vec![v]),
            AccessorOp::Element(indices) => {
                let array = self.current.as_array().ok_or_else(|| Error::ArrayAccess)?;
                let mut elems = Vec::with_capacity(indices.len());
                for index in indices {
                    match index {
                        ArrayIndex::Index(i) => {
                            let i = i
                                .to_usize(array.len())
                                .ok_or_else(|| Error::ArrayIndexOutOfBounds)?;
                            let elem = array.get(i).unwrap();
                            elems.push(elem);
                        }
                        ArrayIndex::Slice((begin, end)) => {
                            let begin = begin
                                .to_usize(array.len())
                                .ok_or_else(|| Error::ArrayIndexOutOfBounds)?;
                            let end = end
                                .to_usize(array.len())
                                .ok_or_else(|| Error::ArrayIndexOutOfBounds)?;
                            if begin > end {
                                return Err(Error::ArrayIndexOutOfBounds);
                            }
                            elems
                                .extend(array.list().into_iter().skip(begin).take(end + 1 - begin));
                        }
                    }
                }
                Ok(elems)
            }
            AccessorOp::FilterExpr(_) => todo!(),
            AccessorOp::Method(method) => self.query_method(method).map(|v| vec![v]),
        }
    }

    fn query_method(&self, method: &Method) -> Result<T, Error> {
        match method {
            Method::Type => {
                let s = if self.current.is_null() {
                    "null"
                } else if self.current.is_bool() {
                    "boolean"
                } else if self.current.is_number() {
                    "number"
                } else if self.current.is_str() {
                    "string"
                } else if self.current.is_array() {
                    "array"
                } else if self.current.is_object() {
                    "object"
                } else {
                    unreachable!()
                };
                Ok(<T::Owned>::from_string(s))
            }
            Method::Size => {
                let size = if let Some(array) = self.current.as_array() {
                    // The size of an SQL/JSON array is the number of elements in the array.
                    array.len()
                } else {
                    // The size of an SQL/JSON object or a scalar is 1.
                    1
                };
                Ok(<T::Owned>::from_u64(size as u64))
            }
            Method::Double => {
                if let Some(s) = self.current.as_str() {
                    let n = s.parse::<f64>().map_err(|_| Error::InvalidDouble)?;
                    Ok(<T::Owned>::from_f64(n))
                } else if self.current.is_number() {
                    Ok(self.current.clone())
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

    fn query_value(&self, value: &Value) -> Result<T, Error> {
        Ok(T::from_value(value.clone()))
    }

    /// Compare two values.
    ///
    /// Return unknown if the values are not comparable.
    fn compare(&self, op: CompareOp, left: T, right: T) -> Truth {
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
                Eq => number_equal_to(&left, &right),
                Ne => !number_equal_to(&left, &right),
                Gt => number_less_than(&right, &left),
                Ge => !number_less_than(&left, &right),
                Lt => number_less_than(&left, &right),
                Le => !number_less_than(&right, &left),
            }
            .into();
        }
        if let (Some(left), Some(right)) = (left.as_str(), right.as_str()) {
            return compare_ord(op, left, right).into();
        }
        // others (include arrays and objects) are not comparable
        Truth::Unknown
    }
}

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

fn number_equal_to(left: &Number, right: &Number) -> bool {
    if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
        l == r
    } else if let (Some(l), Some(r)) = (left.as_i64(), right.as_i64()) {
        l == r
    } else if let (Some(l), Some(r)) = (left.as_u64(), right.as_u64()) {
        l == r
    } else {
        false
    }
}

fn number_less_than(n1: &Number, n2: &Number) -> bool {
    if let (Some(a), Some(b)) = (n1.as_f64(), n2.as_f64()) {
        a < b
    } else if let (Some(a), Some(b)) = (n1.as_i64(), n2.as_i64()) {
        a < b
    } else if let (Some(a), Some(b)) = (n1.as_u64(), n2.as_u64()) {
        a < b
    } else {
        false
    }
}
