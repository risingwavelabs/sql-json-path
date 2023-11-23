// Copyright 2023 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    // structural errors
    #[error("JSON object does not contain key \"{0}\"")]
    NoKey(Box<str>),
    #[error("jsonpath array accessor can only be applied to an array")]
    ArrayAccess,
    #[error("jsonpath wildcard array accessor can only be applied to an array")]
    WildcardArrayAccess,
    #[error("jsonpath member accessor can only be applied to an object")]
    MemberAccess,
    #[error("jsonpath wildcard member accessor can only be applied to an object")]
    WildcardMemberAccess,
    #[error("jsonpath array subscript is out of bounds")]
    ArrayIndexOutOfBounds,

    #[error("jsonpath array subscript is out of integer range")]
    ArrayIndexOutOfRange,
    #[error("jsonpath array subscript is not a single numeric value")]
    ArrayIndexNotNumeric,
    #[error("could not find jsonpath variable \"{0}\"")]
    NoVariable(Box<str>),
    #[error("\"vars\" argument is not an object")]
    VarsNotObject,
    #[error("operand of unary jsonpath operator {0} is not a numeric value")]
    UnaryOperandNotNumeric(UnaryOp),
    #[error("left operand of jsonpath operator {0} is not a single numeric value")]
    LeftOperandNotNumeric(BinaryOp),
    #[error("right operand of jsonpath operator {0} is not a single numeric value")]
    RightOperandNotNumeric(BinaryOp),
    #[error("jsonpath item method .{0}() can only be applied to a numeric value")]
    MethodNotNumeric(&'static str),
    #[error("jsonpath item method .size() can only be applied to an array")]
    SizeNotArray,
    #[error("jsonpath item method .double() can only be applied to a string or numeric value")]
    DoubleTypeError,
    #[error("numeric argument of jsonpath item method .double() is out of range for type double precision")]
    DoubleOutOfRange,
    #[error("string argument of jsonpath item method .double() is not a valid representation of a double precision number")]
    InvalidDouble,
    #[error("jsonpath item method .keyvalue() can only be applied to an object")]
    KeyValueNotObject,
    #[error("division by zero")]
    DivisionByZero,
    #[error("single boolean result is expected")]
    ExpectSingleBoolean,
}

impl Error {
    /// Returns true if the error can be suppressed.
    pub const fn can_silent(&self) -> bool {
        // missing object field or array element
        // unexpected JSON item type
        // datetime and numeric errors.
        !matches!(self, Self::NoVariable(_))
    }

    // A structural error is an attempt to access a non-existent member of an object or element of an array.
    pub const fn is_structural(&self) -> bool {
        matches!(
            self,
            Self::NoKey(_)
                | Self::ArrayAccess
                | Self::WildcardArrayAccess
                | Self::MemberAccess
                | Self::WildcardMemberAccess
                | Self::ArrayIndexOutOfBounds
        )
    }
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

    /// Merge two predicate results.
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
            (Truth::True, _) | (_, Truth::True) => Truth::True,
            (Truth::False, Truth::False) => Truth::False,
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
    pub fn query<'a, T: JsonRef<'a>>(&self, value: T) -> Result<Vec<Cow<'a, T::Owned>>> {
        Evaluator {
            root: value,
            current: value,
            vars: T::null(),
            array: T::null(),
            mode: self.mode,
            first: false,
        }
        .eval_expr_or_predicate(&self.expr)
    }

    /// Evaluate the JSON path against the given JSON value with variables.
    pub fn query_with_vars<'a, T: JsonRef<'a>>(
        &self,
        value: T,
        vars: T,
    ) -> Result<Vec<Cow<'a, T::Owned>>> {
        if !vars.is_object() {
            return Err(Error::VarsNotObject);
        }
        Evaluator {
            root: value,
            current: value,
            vars,
            array: T::null(),
            mode: self.mode,
            first: false,
        }
        .eval_expr_or_predicate(&self.expr)
    }

    /// Evaluate the JSON path against the given JSON value.
    pub fn query_first<'a, T: JsonRef<'a>>(&self, value: T) -> Result<Option<Cow<'a, T::Owned>>> {
        Evaluator {
            root: value,
            current: value,
            vars: T::null(),
            array: T::null(),
            mode: self.mode,
            first: true,
        }
        .eval_expr_or_predicate(&self.expr)
        .map(|set| set.into_iter().next())
    }

    /// Evaluate the JSON path against the given JSON value with variables.
    pub fn query_first_with_vars<'a, T: JsonRef<'a>>(
        &self,
        value: T,
        vars: T,
    ) -> Result<Option<Cow<'a, T::Owned>>> {
        if !vars.is_object() {
            return Err(Error::VarsNotObject);
        }
        Evaluator {
            root: value,
            current: value,
            vars,
            array: T::null(),
            mode: self.mode,
            first: true,
        }
        .eval_expr_or_predicate(&self.expr)
        .map(|set| set.into_iter().next())
    }

    /// Checks whether the JSON path returns any item for the specified JSON value.
    pub fn exists<'a, T: JsonRef<'a>>(&self, value: T) -> Result<bool> {
        self.query_first(value).map(|v| v.is_some())
    }

    /// Checks whether the JSON path returns any item for the specified JSON value,
    /// with variables.
    pub fn exists_with_vars<'a, T: JsonRef<'a>>(&self, value: T, vars: T) -> Result<bool> {
        self.query_first_with_vars(value, vars).map(|v| v.is_some())
    }
}

/// Evaluation context.
#[derive(Debug, Clone, Copy)]
struct Evaluator<'a, T: Json + 'a> {
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
    /// Only return the first result.
    first: bool,
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
    // for `Option`
    ($self:expr, $expr:expr, $err:expr; continue) => {
        match $expr {
            Some(x) => x,
            None if $self.is_lax() => continue,
            None => return Err($err),
        }
    };
    // for `Option`
    ($self:expr, $expr:expr, $err:expr; break) => {
        match $expr {
            Some(x) => x,
            None if $self.is_lax() => break,
            None => return Err($err),
        }
    };
    // for `Result` in predicate
    ($self:expr, $expr:expr) => {
        match $expr {
            Ok(x) => x,
            Err(e @ Error::NoVariable(_)) => return Err(e),
            Err(_) => return Ok(Truth::Unknown),
        }
    };
}

impl<'a, T: Json> Evaluator<'a, T> {
    /// Returns true if the evaluator is in lax mode.
    fn is_lax(&self) -> bool {
        matches!(self.mode, Mode::Lax)
    }

    /// Returns true if the path engine is permitted to stop evaluation early on the first success.
    fn is_first(&self) -> bool {
        self.first && self.is_lax()
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
            first: self.first,
        }
    }

    fn all(&self) -> Self {
        Evaluator {
            first: false,
            ..*self
        }
    }

    fn first(&self) -> Self {
        Evaluator {
            first: true,
            ..*self
        }
    }

    /// Returns the value of the given variable.
    fn get_variable(&self, name: &str) -> Result<T::Borrowed<'a>> {
        self.vars
            .as_object()
            // no `vars` input
            .ok_or_else(|| Error::NoVariable(name.into()))?
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
                let left = lax!(self, self.all().eval_expr(left));
                let right = lax!(self, self.all().eval_expr(right));

                let mut result = Truth::False;
                // The cross product of these SQL/JSON sequences is formed.
                // Each SQL/JSON item in one SQL/JSON sequence is compared to each item in the other SQL/JSON sequence.
                'product: for r in right.iter() {
                    for l in left.iter() {
                        let res = eval_compare::<T>(*op, l.as_ref(), r.as_ref());
                        result = result.merge(res);
                        // The predicate is Unknown if there any pair of SQL/JSON items in the cross product is not comparable.
                        // the predicate is True if any pair is comparable and satisfies the comparison operator.
                        if res.is_unknown() || res.is_true() && self.is_lax() {
                            // In lax mode, the path engine is permitted to stop evaluation early if it detects either an error or a success.
                            break 'product;
                        }
                        // In strict mode, the path engine must test all comparisons in the cross product.
                    }
                }
                Ok(result)
            }
            Predicate::Exists(expr) => {
                let set = lax!(self, self.first().eval_expr(expr));
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
                let set = lax!(self, self.all().eval_expr(expr));
                let prefix = self.eval_value(prefix)?;
                let prefix = prefix.as_ref().as_str().unwrap();
                let mut result = Truth::False;
                for v in set {
                    let res = match v.as_ref().as_str() {
                        Some(s) => s.starts_with(prefix).into(),
                        None => Truth::Unknown,
                    };
                    result = result.merge(res);
                    if result.is_unknown() || result.is_true() && self.is_lax() {
                        break;
                    }
                }
                Ok(result)
            }
            Predicate::LikeRegex(expr, regex) => {
                let set = lax!(self, self.all().eval_expr(expr));
                let mut result = Truth::False;
                for v in set {
                    let res = match v.as_ref().as_str() {
                        Some(s) => regex.is_match(s).into(),
                        None => Truth::Unknown,
                    };
                    result = result.merge(res);
                    if result.is_unknown() || result.is_true() && self.is_lax() {
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
            Expr::PathPrimary(primary) => self.eval_path_primary(primary),
            Expr::Accessor(base, op) => {
                let set = self.all().eval_expr(base)?;
                let mut new_set = vec![];
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
                    if self.is_first() && !new_set.is_empty() {
                        break;
                    }
                }
                Ok(new_set)
            }
            Expr::UnaryOp(op, expr) => {
                let set = self.eval_expr(expr)?;
                let mut new_set = Vec::with_capacity(set.len());
                for v in set {
                    let v = v.as_ref();
                    if v.is_array() && self.is_lax() {
                        // unwrap the array and apply the operator to each element
                        for v in v.as_array().unwrap().list() {
                            new_set.push(Cow::Owned(eval_unary_op(*op, v)?));
                        }
                    } else {
                        new_set.push(Cow::Owned(eval_unary_op(*op, v)?));
                    }
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
                // unwrap left if it is an array
                let left = if self.is_lax() {
                    if let Some(array) = left[0].as_ref().as_array() {
                        if array.len() != 1 {
                            return Err(Error::LeftOperandNotNumeric(*op));
                        }
                        array.get(0).unwrap()
                    } else {
                        left[0].as_ref()
                    }
                } else {
                    left[0].as_ref()
                };
                // unwrap right if it is an array
                let right = if self.is_lax() {
                    if let Some(array) = right[0].as_ref().as_array() {
                        if array.len() != 1 {
                            return Err(Error::RightOperandNotNumeric(*op));
                        }
                        array.get(0).unwrap()
                    } else {
                        right[0].as_ref()
                    }
                } else {
                    right[0].as_ref()
                };
                Ok(vec![Cow::Owned(eval_binary_op(*op, left, right)?)])
            }
        }
    }

    /// Evaluates the path primary.
    fn eval_path_primary(&self, primary: &PathPrimary) -> Result<Vec<Cow<'a, T>>> {
        match primary {
            PathPrimary::Root => Ok(vec![Cow::Borrowed(self.root)]),
            PathPrimary::Current => Ok(vec![Cow::Borrowed(self.current)]),
            PathPrimary::Value(v) => Ok(vec![self.eval_value(v)?]),
            PathPrimary::Last => {
                let array = self
                    .array
                    .as_array()
                    .expect("LAST is allowed only in array subscripts");
                Ok(vec![Cow::Owned(T::from_i64(array.len() as i64 - 1))])
            }
            PathPrimary::ExprOrPred(expr) => self.eval_expr_or_predicate(expr),
        }
    }

    /// Evaluates the accessor operator.
    fn eval_accessor_op(&self, op: &AccessorOp) -> Result<Vec<Cow<'a, T>>> {
        match op {
            AccessorOp::MemberWildcard => self.eval_member_wildcard(),
            AccessorOp::DescendantMemberWildcard(levels) => {
                self.eval_descendant_member_wildcard(levels)
            }
            AccessorOp::ElementWildcard => self.eval_element_wildcard(),
            AccessorOp::Member(name) => self.eval_member(name),
            AccessorOp::Element(indices) => self.eval_element_accessor(indices),
            AccessorOp::FilterExpr(pred) => self.eval_filter_expr(pred),
            AccessorOp::Method(method) => self.eval_method(method),
        }
    }

    fn eval_member_wildcard(&self) -> Result<Vec<Cow<'a, T>>> {
        let set = match self.current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![self.current],
        };
        let mut new_set = vec![];
        for v in set {
            let object = lax!(self, v.as_object(), Error::WildcardMemberAccess);
            for v in object.list_value() {
                new_set.push(Cow::Borrowed(v));
            }
        }
        Ok(new_set)
    }

    fn eval_descendant_member_wildcard(&self, levels: &LevelRange) -> Result<Vec<Cow<'a, T>>> {
        let mut set = match self.current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![self.current],
        };
        // expand all levels
        // level i is set[level_start[i] .. level_start[i+1]]
        let mut level_start = vec![0, set.len()];
        for l in 1..=levels.end() {
            let last_level_range = level_start[l as usize - 1]..level_start[l as usize];
            for i in last_level_range {
                if let Some(object) = set[i].as_object() {
                    set.extend(object.list_value());
                }
            }
            if set.len() == level_start[l as usize] {
                // this level is empty
                break;
            }
            level_start.push(set.len());
        }
        // return the set in level range
        let last_level = level_start.len() - 2;
        let level_range = levels.to_range(last_level);
        let set_range = level_start[level_range.start]..level_start[level_range.end];
        let new_set = set[set_range].iter().cloned().map(Cow::Borrowed).collect();
        Ok(new_set)
    }

    fn eval_element_wildcard(&self) -> Result<Vec<Cow<'a, T>>> {
        if !self.current.is_array() && self.is_lax() {
            // wrap the current value into an array
            return Ok(vec![Cow::Borrowed(self.current)]);
        }
        let array = lax!(self, self.current.as_array(), Error::WildcardArrayAccess);
        if self.is_first() {
            return Ok(array.get(0).map(Cow::Borrowed).into_iter().collect());
        }
        Ok(array.list().into_iter().map(Cow::Borrowed).collect())
    }

    /// Evaluates the member accessor.
    fn eval_member(&self, name: &str) -> Result<Vec<Cow<'a, T>>> {
        let set = match self.current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![self.current],
        };
        let mut new_set = vec![];
        for v in set {
            let object = lax!(self, v.as_object(), Error::MemberAccess);
            let elem = lax!(self, object.get(name), Error::NoKey(name.into()));
            new_set.push(Cow::Borrowed(elem));
        }
        Ok(new_set)
    }

    /// Evaluates the element accessor.
    fn eval_element_accessor(&self, indices: &[ArrayIndex]) -> Result<Vec<Cow<'a, T>>> {
        // wrap the scalar value into an array in lax mode
        enum ArrayOrScalar<'a, T: JsonRef<'a>> {
            Array(T::Array),
            Scalar(T),
        }
        impl<'a, T: JsonRef<'a>> ArrayOrScalar<'a, T> {
            fn get(&self, index: usize) -> Option<T> {
                match self {
                    ArrayOrScalar::Array(array) => array.get(index),
                    ArrayOrScalar::Scalar(scalar) if index == 0 => Some(*scalar),
                    _ => None,
                }
            }
        }
        let array = match self.current.as_array() {
            Some(array) => ArrayOrScalar::Array(array),
            None if self.is_lax() => ArrayOrScalar::Scalar(self.current),
            None => return Err(Error::ArrayAccess),
        };
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
                    return Err(Error::ArrayIndexNotNumeric);
                }
                set[0]
                    .as_ref()
                    .as_number()
                    .ok_or(Error::ArrayIndexNotNumeric)?
                    .to_i64()
                    .ok_or(Error::ArrayIndexOutOfRange)
            };
            match index {
                ArrayIndex::Index(expr) => {
                    let index = eval_index(expr)?;
                    let index =
                        lax!(self, index.try_into().ok(), Error::ArrayIndexOutOfBounds; continue);
                    let elem = lax!(self, array.get(index), Error::ArrayIndexOutOfBounds; continue);
                    elems.push(Cow::Borrowed(elem));
                }
                ArrayIndex::Slice(begin, end) => {
                    let begin = eval_index(begin)?;
                    let end = eval_index(end)?;
                    let begin: usize = match begin.try_into() {
                        Ok(i) => i,
                        Err(_) if self.is_lax() => 0,
                        Err(_) => return Err(Error::ArrayIndexOutOfBounds),
                    };
                    let end: usize =
                        lax!(self, end.try_into().ok(), Error::ArrayIndexOutOfBounds; continue);
                    if begin > end && !self.is_lax() {
                        return Err(Error::ArrayIndexOutOfBounds);
                    }
                    for i in begin..=end {
                        let elem = lax!(self, array.get(i), Error::ArrayIndexOutOfBounds; break);
                        elems.push(Cow::Borrowed(elem));
                    }
                }
            }
        }
        Ok(elems)
    }

    fn eval_filter_expr(&self, pred: &Predicate) -> Result<Vec<Cow<'a, T>>> {
        let set = match self.current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![self.current],
        };
        let mut new_set = vec![];
        for v in set {
            if self.with_current(v).eval_predicate(pred)?.is_true() {
                new_set.push(Cow::Borrowed(v));
                if self.is_first() {
                    break;
                }
            }
        }
        Ok(new_set)
    }

    /// Evaluates the item method.
    fn eval_method(&self, method: &Method) -> Result<Vec<Cow<'a, T>>> {
        // unwrap the current value if it is an array
        if self.current.is_array()
            && self.is_lax()
            && !matches!(method, Method::Size | Method::Type)
        {
            let mut new_set = vec![];
            for v in self.current.as_array().unwrap().list() {
                new_set.extend(self.with_current(v).eval_method(method)?);
            }
            return Ok(new_set);
        }
        match method {
            Method::Type => self.eval_method_type().map(|v| vec![v]),
            Method::Size => self.eval_method_size().map(|v| vec![v]),
            Method::Double => self.eval_method_double().map(|v| vec![v]),
            Method::Ceiling => self.eval_method_ceiling().map(|v| vec![v]),
            Method::Floor => self.eval_method_floor().map(|v| vec![v]),
            Method::Abs => self.eval_method_abs().map(|v| vec![v]),
            Method::Keyvalue => self.eval_method_keyvalue(),
        }
    }

    fn eval_method_type(&self) -> Result<Cow<'a, T>> {
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

    fn eval_method_size(&self) -> Result<Cow<'a, T>> {
        let size = if let Some(array) = self.current.as_array() {
            // The size of an SQL/JSON array is the number of elements in the array.
            array.len()
        } else if self.is_lax() {
            // The size of an SQL/JSON object or a scalar is 1.
            1
        } else {
            return Err(Error::SizeNotArray);
        };
        Ok(Cow::Owned(T::from_u64(size as u64)))
    }

    fn eval_method_double(&self) -> Result<Cow<'a, T>> {
        if let Some(s) = self.current.as_str() {
            let n = s.parse::<f64>().map_err(|_| Error::InvalidDouble)?;
            if n.is_infinite() || n.is_nan() {
                return Err(Error::InvalidDouble);
            }
            Ok(Cow::Owned(T::from_f64(n)))
        } else if self.current.is_number() {
            Ok(Cow::Borrowed(self.current))
        } else {
            Err(Error::DoubleTypeError)
        }
    }

    fn eval_method_ceiling(&self) -> Result<Cow<'a, T>> {
        let n = self
            .current
            .as_number()
            .ok_or(Error::MethodNotNumeric("ceiling"))?;
        Ok(Cow::Owned(T::from_number(n.ceil())))
    }

    fn eval_method_floor(&self) -> Result<Cow<'a, T>> {
        let n = self
            .current
            .as_number()
            .ok_or(Error::MethodNotNumeric("floor"))?;
        Ok(Cow::Owned(T::from_number(n.floor())))
    }

    fn eval_method_abs(&self) -> Result<Cow<'a, T>> {
        let n = self
            .current
            .as_number()
            .ok_or(Error::MethodNotNumeric("abs"))?;
        Ok(Cow::Owned(T::from_number(n.abs())))
    }

    fn eval_method_keyvalue(&self) -> Result<Vec<Cow<'a, T>>> {
        let object = self.current.as_object().ok_or(Error::KeyValueNotObject)?;
        Ok(object
            .list()
            .into_iter()
            .map(|(k, v)| {
                Cow::Owned(T::object([
                    ("key", T::from_string(k)),
                    ("value", v.to_owned()),
                    ("id", T::from_i64(0)), // FIXME: provide unique id
                ]))
            })
            .collect())
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
    // arrays and objects are not comparable
    if left.is_array() || left.is_object() || right.is_array() || right.is_object() {
        return Truth::Unknown;
    }
    // SQL/JSON null is equal to SQL/JSON null, and is not greater than or less than anything.
    if left.is_null() && right.is_null() {
        return compare_ord(op, (), ()).into();
    }
    if left.is_null() || right.is_null() {
        return (op == CompareOp::Ne).into();
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
    // others are not comparable
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
        BinaryOp::Div => left.div(&right)?,
        BinaryOp::Rem => left.rem(&right)?,
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
pub trait NumberExt: Sized {
    fn equal(&self, other: &Self) -> bool;
    fn less_than(&self, other: &Self) -> bool;
    fn neg(&self) -> Self;
    fn add(&self, other: &Self) -> Self;
    fn sub(&self, other: &Self) -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn div(&self, other: &Self) -> Result<Self>;
    fn rem(&self, other: &Self) -> Result<Self>;
    fn ceil(&self) -> Self;
    fn floor(&self) -> Self;
    fn abs(&self) -> Self;
    fn to_i64(&self) -> Option<i64>;
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

    fn div(&self, other: &Self) -> Result<Self> {
        if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            if b == 0.0 {
                return Err(Error::DivisionByZero);
            }
            Ok(Number::from_f64(a / b).unwrap())
        } else {
            unreachable!()
        }
    }

    fn rem(&self, other: &Self) -> Result<Self> {
        if let (Some(a), Some(b)) = (self.as_i64(), other.as_i64()) {
            if b == 0 {
                return Err(Error::DivisionByZero);
            }
            Ok(Number::from(a % b))
        } else if let (Some(a), Some(b)) = (self.as_f64(), other.as_f64()) {
            if b == 0.0 {
                return Err(Error::DivisionByZero);
            }
            Ok(Number::from_f64(a % b).unwrap())
        } else {
            unreachable!()
        }
    }

    fn ceil(&self) -> Self {
        if self.is_f64() {
            Number::from(self.as_f64().unwrap().ceil() as i64)
        } else {
            self.clone()
        }
    }

    fn floor(&self) -> Self {
        if self.is_f64() {
            Number::from(self.as_f64().unwrap().floor() as i64)
        } else {
            self.clone()
        }
    }

    fn abs(&self) -> Self {
        if let Some(n) = self.as_i64() {
            Number::from(n.abs())
        } else if let Some(n) = self.as_f64() {
            Number::from_f64(n.abs()).unwrap()
        } else {
            unreachable!()
        }
    }

    /// Converts to json integer if possible.
    /// Float values are truncated.
    /// Returns `None` if the value is out of range.
    /// Range: [-2^53 + 1, 2^53 - 1]
    fn to_i64(&self) -> Option<i64> {
        const INT_MIN: i64 = -(1 << 53) + 1;
        const INT_MAX: i64 = (1 << 53) - 1;
        if let Some(i) = self.as_i64() {
            if (INT_MIN..=INT_MAX).contains(&i) {
                Some(i)
            } else {
                None
            }
        } else if let Some(f) = self.as_f64() {
            if (INT_MIN as f64..=INT_MAX as f64).contains(&f) {
                Some(f as i64)
            } else {
                None
            }
        } else {
            unreachable!()
        }
    }
}
