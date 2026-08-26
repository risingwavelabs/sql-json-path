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

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use serde_json::Number;

use crate::{
    ast::*,
    json::{ArrayRef, Cow, Json, JsonRef, ObjectRef},
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DateTimeValue {
    Date(NaiveDate),
    Time(NaiveTime),
    TimeTz(NaiveTime, FixedOffset),
    Timestamp(NaiveDateTime),
    TimestampTz(DateTime<FixedOffset>),
}

impl DateTimeValue {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Date(_) => "date",
            Self::Time(_) => "time without time zone",
            Self::TimeTz(_, _) => "time with time zone",
            Self::Timestamp(_) => "timestamp without time zone",
            Self::TimestampTz(_) => "timestamp with time zone",
        }
    }

    fn to_iso_string(&self) -> String {
        match self {
            Self::Date(value) => value.format("%Y-%m-%d").to_string(),
            Self::Time(value) => value.format("%H:%M:%S%.f").to_string(),
            Self::TimeTz(value, offset) => {
                format!("{}{}", value.format("%H:%M:%S%.f"), offset)
            }
            Self::Timestamp(value) => value.format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
            Self::TimestampTz(value) => value.format("%Y-%m-%dT%H:%M:%S%.f%:z").to_string(),
        }
    }
}

#[derive(Debug)]
enum Item<'a, T: Json + 'a> {
    Json(Cow<'a, T>),
    DateTime(DateTimeValue),
}

impl<'a, T: Json> Item<'a, T> {
    fn as_ref(&self) -> ItemRef<'_, T> {
        match self {
            Self::Json(value) => ItemRef::Json(value.as_ref()),
            Self::DateTime(value) => ItemRef::DateTime(value),
        }
    }

    fn into_owned<'b>(self) -> Item<'b, T> {
        match self {
            Self::Json(value) => Item::Json(Cow::Owned(value.into_owned())),
            Self::DateTime(value) => Item::DateTime(value),
        }
    }

    fn into_json(self) -> Cow<'a, T> {
        match self {
            Self::Json(value) => value,
            Self::DateTime(value) => Cow::Owned(T::from_string(&value.to_iso_string())),
        }
    }
}

#[derive(Debug)]
enum ItemRef<'a, T: Json + 'a> {
    Json(T::Borrowed<'a>),
    DateTime(&'a DateTimeValue),
}

impl<T: Json> Copy for ItemRef<'_, T> {}

impl<T: Json> Clone for ItemRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: Json> ItemRef<'a, T> {
    fn as_json(self) -> Option<T::Borrowed<'a>> {
        match self {
            Self::Json(value) => Some(value),
            Self::DateTime(_) => None,
        }
    }

    fn to_owned<'b>(self) -> Item<'b, T> {
        match self {
            Self::Json(value) => Item::Json(Cow::Owned(value.to_owned())),
            Self::DateTime(value) => Item::DateTime(value.clone()),
        }
    }
}

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
    #[error("jsonpath item method .{0}() can only be applied to a string or numeric value")]
    NumericConversionType(&'static str),
    #[error("argument \"{0}\" of jsonpath item method .{1}() is invalid for type {2}")]
    InvalidConversion(Box<str>, &'static str, &'static str),
    #[error("jsonpath item method .boolean() can only be applied to a boolean, string, or numeric value")]
    BooleanTypeError,
    #[error("jsonpath item method .string() can only be applied to a boolean, string, numeric, or datetime value")]
    StringTypeError,
    #[error("jsonpath item method .{0}() can only be applied to a string")]
    DateTimeTypeError(&'static str),
    #[error("{0} format is not recognized: \"{1}\"")]
    InvalidDateTime(&'static str, Box<str>),
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
            current: ItemRef::Json(value),
            vars: T::null(),
            array: T::null(),
            mode: self.mode,
            first: false,
        }
        .eval_expr_or_predicate(&self.expr)
        .map(|set| set.into_iter().map(Item::into_json).collect())
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
            current: ItemRef::Json(value),
            vars,
            array: T::null(),
            mode: self.mode,
            first: false,
        }
        .eval_expr_or_predicate(&self.expr)
        .map(|set| set.into_iter().map(Item::into_json).collect())
    }

    /// Evaluate the JSON path against the given JSON value.
    pub fn query_first<'a, T: JsonRef<'a>>(&self, value: T) -> Result<Option<Cow<'a, T::Owned>>> {
        Evaluator {
            root: value,
            current: ItemRef::Json(value),
            vars: T::null(),
            array: T::null(),
            mode: self.mode,
            first: true,
        }
        .eval_expr_or_predicate(&self.expr)
        .map(|set| set.into_iter().next().map(Item::into_json))
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
            current: ItemRef::Json(value),
            vars,
            array: T::null(),
            mode: self.mode,
            first: true,
        }
        .eval_expr_or_predicate(&self.expr)
        .map(|set| set.into_iter().next().map(Item::into_json))
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
    current: ItemRef<'a, T>,
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
    fn with_current<'b>(&self, current: ItemRef<'b, T>) -> Evaluator<'b, T>
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
    fn eval_expr_or_predicate(&self, expr: &ExprOrPredicate) -> Result<Vec<Item<'a, T>>> {
        match expr {
            ExprOrPredicate::Expr(expr) => self.eval_expr(expr),
            ExprOrPredicate::Pred(pred) => self
                .eval_predicate(pred)
                .map(|t| vec![Item::Json(Cow::Owned(t.to_json()))]),
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
                let prefix = prefix.as_ref().as_json().unwrap().as_str().unwrap();
                let mut result = Truth::False;
                for v in set {
                    let res = match v.as_ref().as_json().and_then(JsonRef::as_str) {
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
                    let res = match v.as_ref().as_json().and_then(JsonRef::as_str) {
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
    fn eval_expr(&self, expr: &Expr) -> Result<Vec<Item<'a, T>>> {
        match expr {
            Expr::PathPrimary(primary) => self.eval_path_primary(primary),
            Expr::Accessor(base, op) => {
                let set = self.all().eval_expr(base)?;
                let mut new_set = vec![];
                for v in &set {
                    let sset = self.with_current(v.as_ref()).eval_accessor_op(op)?;
                    new_set.extend(sset.into_iter().map(Item::into_owned));
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
                    let v = v
                        .as_ref()
                        .as_json()
                        .ok_or(Error::UnaryOperandNotNumeric(*op))?;
                    if v.is_array() && self.is_lax() {
                        // unwrap the array and apply the operator to each element
                        for v in v.as_array().unwrap().list() {
                            new_set.push(Item::Json(Cow::Owned(eval_unary_op(*op, v)?)));
                        }
                    } else {
                        new_set.push(Item::Json(Cow::Owned(eval_unary_op(*op, v)?)));
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
                let left = left[0]
                    .as_ref()
                    .as_json()
                    .ok_or(Error::LeftOperandNotNumeric(*op))?;
                let left = if self.is_lax() {
                    if let Some(array) = left.as_array() {
                        if array.len() != 1 {
                            return Err(Error::LeftOperandNotNumeric(*op));
                        }
                        array.get(0).unwrap()
                    } else {
                        left
                    }
                } else {
                    left
                };
                // unwrap right if it is an array
                let right = right[0]
                    .as_ref()
                    .as_json()
                    .ok_or(Error::RightOperandNotNumeric(*op))?;
                let right = if self.is_lax() {
                    if let Some(array) = right.as_array() {
                        if array.len() != 1 {
                            return Err(Error::RightOperandNotNumeric(*op));
                        }
                        array.get(0).unwrap()
                    } else {
                        right
                    }
                } else {
                    right
                };
                Ok(vec![Item::Json(Cow::Owned(eval_binary_op(
                    *op, left, right,
                )?))])
            }
        }
    }

    /// Evaluates the path primary.
    fn eval_path_primary(&self, primary: &PathPrimary) -> Result<Vec<Item<'a, T>>> {
        match primary {
            PathPrimary::Root => Ok(vec![Item::Json(Cow::Borrowed(self.root))]),
            PathPrimary::Current => Ok(vec![self.current.to_owned()]),
            PathPrimary::Value(v) => Ok(vec![self.eval_value(v)?]),
            PathPrimary::Last => {
                let array = self
                    .array
                    .as_array()
                    .expect("LAST is allowed only in array subscripts");
                Ok(vec![Item::Json(Cow::Owned(T::from_i64(
                    array.len() as i64 - 1,
                )))])
            }
            PathPrimary::ExprOrPred(expr) => self.eval_expr_or_predicate(expr),
        }
    }

    /// Evaluates the accessor operator.
    fn eval_accessor_op(&self, op: &AccessorOp) -> Result<Vec<Item<'a, T>>> {
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

    fn eval_member_wildcard(&self) -> Result<Vec<Item<'a, T>>> {
        let current = lax!(self, self.current.as_json(), Error::WildcardMemberAccess);
        let set = match current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![current],
        };
        let mut new_set = vec![];
        for v in set {
            let object = lax!(self, v.as_object(), Error::WildcardMemberAccess);
            for v in object.list_value() {
                new_set.push(Item::Json(Cow::Borrowed(v)));
            }
        }
        Ok(new_set)
    }

    fn eval_descendant_member_wildcard(&self, levels: &LevelRange) -> Result<Vec<Item<'a, T>>> {
        let current = lax!(self, self.current.as_json(), Error::WildcardMemberAccess);
        let mut set = match current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![current],
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
        let new_set = set[set_range]
            .iter()
            .cloned()
            .map(|value| Item::Json(Cow::Borrowed(value)))
            .collect();
        Ok(new_set)
    }

    fn eval_element_wildcard(&self) -> Result<Vec<Item<'a, T>>> {
        let Some(current) = self.current.as_json() else {
            return if self.is_lax() {
                Ok(vec![self.current.to_owned()])
            } else {
                Err(Error::WildcardArrayAccess)
            };
        };
        if !current.is_array() && self.is_lax() {
            // wrap the current value into an array
            return Ok(vec![Item::Json(Cow::Borrowed(current))]);
        }
        let array = lax!(self, current.as_array(), Error::WildcardArrayAccess);
        if self.is_first() {
            return Ok(array
                .get(0)
                .map(|value| Item::Json(Cow::Borrowed(value)))
                .into_iter()
                .collect());
        }
        Ok(array
            .list()
            .into_iter()
            .map(|value| Item::Json(Cow::Borrowed(value)))
            .collect())
    }

    /// Evaluates the member accessor.
    fn eval_member(&self, name: &str) -> Result<Vec<Item<'a, T>>> {
        let current = lax!(self, self.current.as_json(), Error::MemberAccess);
        let set = match current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![current],
        };
        let mut new_set = vec![];
        for v in set {
            let object = lax!(self, v.as_object(), Error::MemberAccess);
            let elem = lax!(self, object.get(name), Error::NoKey(name.into()));
            new_set.push(Item::Json(Cow::Borrowed(elem)));
        }
        Ok(new_set)
    }

    /// Evaluates the element accessor.
    fn eval_element_accessor(&self, indices: &[ArrayIndex]) -> Result<Vec<Item<'a, T>>> {
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
        let current = lax!(self, self.current.as_json(), Error::ArrayAccess);
        let array = match current.as_array() {
            Some(array) => ArrayOrScalar::Array(array),
            None if self.is_lax() => ArrayOrScalar::Scalar(current),
            None => return Err(Error::ArrayAccess),
        };
        let mut elems = Vec::with_capacity(indices.len());
        for index in indices {
            let eval_index = |expr: &Expr| {
                // errors in this closure can not be ignored
                let set = Self {
                    // update `array` context
                    array: current,
                    ..*self
                }
                .eval_expr(expr)?;
                if set.len() != 1 {
                    return Err(Error::ArrayIndexNotNumeric);
                }
                set[0]
                    .as_ref()
                    .as_json()
                    .and_then(JsonRef::as_number)
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
                    elems.push(Item::Json(Cow::Borrowed(elem)));
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
                        elems.push(Item::Json(Cow::Borrowed(elem)));
                    }
                }
            }
        }
        Ok(elems)
    }

    fn eval_filter_expr(&self, pred: &Predicate) -> Result<Vec<Item<'a, T>>> {
        if matches!(self.current, ItemRef::DateTime(_)) {
            return if self
                .with_current(self.current)
                .eval_predicate(pred)?
                .is_true()
            {
                Ok(vec![self.current.to_owned()])
            } else {
                Ok(vec![])
            };
        }
        let current = self.current.as_json().unwrap();
        let set = match current.as_array() {
            Some(array) if self.is_lax() => array.list(),
            _ => vec![current],
        };
        let mut new_set = vec![];
        for v in set {
            if self
                .with_current(ItemRef::Json(v))
                .eval_predicate(pred)?
                .is_true()
            {
                new_set.push(Item::Json(Cow::Borrowed(v)));
                if self.is_first() {
                    break;
                }
            }
        }
        Ok(new_set)
    }

    /// Evaluates the item method.
    fn eval_method(&self, method: &Method) -> Result<Vec<Item<'a, T>>> {
        // unwrap the current value if it is an array
        if let Some(array) = self.current.as_json().and_then(JsonRef::as_array) {
            if self.is_lax() && !matches!(method, Method::Size | Method::Type) {
                let mut new_set = vec![];
                for v in array.list() {
                    new_set.extend(self.with_current(ItemRef::Json(v)).eval_method(method)?);
                }
                return Ok(new_set);
            }
        }
        match method {
            Method::Type => self.eval_method_type().map(|v| vec![v]),
            Method::Size => self.eval_method_size().map(|v| vec![v]),
            Method::Double => self.eval_method_double().map(|v| vec![v]),
            Method::Ceiling => self.eval_method_ceiling().map(|v| vec![v]),
            Method::Floor => self.eval_method_floor().map(|v| vec![v]),
            Method::Abs => self.eval_method_abs().map(|v| vec![v]),
            Method::Keyvalue => self.eval_method_keyvalue(),
            Method::Bigint => self.eval_method_integer::<i64>("bigint").map(|v| vec![v]),
            Method::Integer => self.eval_method_integer::<i32>("integer").map(|v| vec![v]),
            Method::Decimal => self.eval_method_number("decimal").map(|v| vec![v]),
            Method::Number => self.eval_method_number("number").map(|v| vec![v]),
            Method::String => self.eval_method_string().map(|v| vec![v]),
            Method::Boolean => self.eval_method_boolean().map(|v| vec![v]),
            Method::Date => self
                .eval_method_datetime("date", DateTimeKind::Date)
                .map(|v| vec![v]),
            Method::Time => self
                .eval_method_datetime("time", DateTimeKind::Time)
                .map(|v| vec![v]),
            Method::TimeTz => self
                .eval_method_datetime("time_tz", DateTimeKind::TimeTz)
                .map(|v| vec![v]),
            Method::Timestamp => self
                .eval_method_datetime("timestamp", DateTimeKind::Timestamp)
                .map(|v| vec![v]),
            Method::TimestampTz => self
                .eval_method_datetime("timestamp_tz", DateTimeKind::TimestampTz)
                .map(|v| vec![v]),
        }
    }

    fn eval_method_type(&self) -> Result<Item<'a, T>> {
        let s = match self.current {
            ItemRef::DateTime(value) => value.type_name(),
            ItemRef::Json(value) if value.is_null() => "null",
            ItemRef::Json(value) if value.is_bool() => "boolean",
            ItemRef::Json(value) if value.is_number() => "number",
            ItemRef::Json(value) if value.is_string() => "string",
            ItemRef::Json(value) if value.is_array() => "array",
            ItemRef::Json(value) if value.is_object() => "object",
            ItemRef::Json(_) => unreachable!(),
        };
        Ok(Item::Json(Cow::Owned(T::from_string(s))))
    }

    fn eval_method_size(&self) -> Result<Item<'a, T>> {
        let size = if let Some(array) = self.current.as_json().and_then(JsonRef::as_array) {
            // The size of an SQL/JSON array is the number of elements in the array.
            array.len()
        } else if self.is_lax() {
            // The size of an SQL/JSON object or a scalar is 1.
            1
        } else {
            return Err(Error::SizeNotArray);
        };
        Ok(Item::Json(Cow::Owned(T::from_u64(size as u64))))
    }

    fn eval_method_double(&self) -> Result<Item<'a, T>> {
        let current = self.current.as_json().ok_or(Error::DoubleTypeError)?;
        if let Some(s) = current.as_str() {
            let n = s.parse::<f64>().map_err(|_| Error::InvalidDouble)?;
            if n.is_infinite() || n.is_nan() {
                return Err(Error::InvalidDouble);
            }
            Ok(Item::Json(Cow::Owned(T::from_f64(n))))
        } else if current.is_number() {
            Ok(Item::Json(Cow::Borrowed(current)))
        } else {
            Err(Error::DoubleTypeError)
        }
    }

    fn eval_method_ceiling(&self) -> Result<Item<'a, T>> {
        let n = self
            .current
            .as_json()
            .and_then(JsonRef::as_number)
            .ok_or(Error::MethodNotNumeric("ceiling"))?;
        Ok(Item::Json(Cow::Owned(T::from_number(n.ceil()))))
    }

    fn eval_method_floor(&self) -> Result<Item<'a, T>> {
        let n = self
            .current
            .as_json()
            .and_then(JsonRef::as_number)
            .ok_or(Error::MethodNotNumeric("floor"))?;
        Ok(Item::Json(Cow::Owned(T::from_number(n.floor()))))
    }

    fn eval_method_abs(&self) -> Result<Item<'a, T>> {
        let n = self
            .current
            .as_json()
            .and_then(JsonRef::as_number)
            .ok_or(Error::MethodNotNumeric("abs"))?;
        Ok(Item::Json(Cow::Owned(T::from_number(n.abs()))))
    }

    fn eval_method_keyvalue(&self) -> Result<Vec<Item<'a, T>>> {
        let object = self
            .current
            .as_json()
            .and_then(JsonRef::as_object)
            .ok_or(Error::KeyValueNotObject)?;
        Ok(object
            .list()
            .into_iter()
            .map(|(k, v)| {
                Item::Json(Cow::Owned(T::object([
                    ("key", T::from_string(k)),
                    ("value", v.to_owned()),
                    ("id", T::from_i64(0)), // FIXME: provide unique id
                ])))
            })
            .collect())
    }

    fn eval_method_integer<I>(&self, method: &'static str) -> Result<Item<'a, T>>
    where
        I: TryFrom<i64> + std::str::FromStr + Into<i64>,
    {
        let current = self
            .current
            .as_json()
            .ok_or(Error::NumericConversionType(method))?;
        let display;
        let value = if let Some(number) = current.as_number() {
            display = number.to_string();
            number_to_i64(&number)
                .and_then(|value| I::try_from(value).ok())
                .ok_or_else(|| Error::InvalidConversion(display.clone().into(), method, method))?
        } else if let Some(string) = current.as_str() {
            display = string.to_owned();
            string
                .trim()
                .parse::<I>()
                .map_err(|_| Error::InvalidConversion(display.clone().into(), method, method))?
        } else {
            return Err(Error::NumericConversionType(method));
        };
        Ok(Item::Json(Cow::Owned(T::from_i64(value.into()))))
    }

    fn eval_method_number(&self, method: &'static str) -> Result<Item<'a, T>> {
        let current = self
            .current
            .as_json()
            .ok_or(Error::NumericConversionType(method))?;
        if current.is_number() {
            return Ok(Item::Json(Cow::Borrowed(current)));
        }
        let string = current
            .as_str()
            .ok_or(Error::NumericConversionType(method))?;
        let normalized = string.trim().strip_prefix('+').unwrap_or(string.trim());
        let number = normalized
            .parse::<Number>()
            .map_err(|_| Error::InvalidConversion(string.into(), method, "numeric"))?;
        Ok(Item::Json(Cow::Owned(T::from_number(number))))
    }

    fn eval_method_boolean(&self) -> Result<Item<'a, T>> {
        let current = self.current.as_json().ok_or(Error::BooleanTypeError)?;
        let value = if let Some(value) = current.as_bool() {
            value
        } else if let Some(number) = current.as_number() {
            let display = number.to_string();
            let value = number_to_i64(&number)
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| Error::InvalidConversion(display.into(), "boolean", "boolean"))?;
            value != 0
        } else if let Some(string) = current.as_str() {
            parse_boolean(string)
                .ok_or_else(|| Error::InvalidConversion(string.into(), "boolean", "boolean"))?
        } else {
            return Err(Error::BooleanTypeError);
        };
        Ok(Item::Json(Cow::Owned(T::bool(value))))
    }

    fn eval_method_string(&self) -> Result<Item<'a, T>> {
        let value = match self.current {
            ItemRef::DateTime(value) => value.to_iso_string(),
            ItemRef::Json(value) => {
                if let Some(string) = value.as_str() {
                    string.to_owned()
                } else if let Some(number) = value.as_number() {
                    number.to_string()
                } else if let Some(value) = value.as_bool() {
                    value.to_string()
                } else {
                    return Err(Error::StringTypeError);
                }
            }
        };
        Ok(Item::Json(Cow::Owned(T::from_string(&value))))
    }

    fn eval_method_datetime(
        &self,
        method: &'static str,
        kind: DateTimeKind,
    ) -> Result<Item<'a, T>> {
        let string = self
            .current
            .as_json()
            .and_then(JsonRef::as_str)
            .ok_or(Error::DateTimeTypeError(method))?;
        let value = parse_datetime(string, kind)
            .ok_or_else(|| Error::InvalidDateTime(method, string.into()))?;
        Ok(Item::DateTime(value))
    }

    /// Evaluates the scalar value.
    fn eval_value(&self, value: &Value) -> Result<Item<'a, T>> {
        Ok(Item::Json(match value {
            Value::Null => Cow::Owned(T::null()),
            Value::Boolean(b) => Cow::Owned(T::bool(*b)),
            Value::Number(n) => Cow::Owned(T::from_number(n.clone())),
            Value::String(s) => Cow::Owned(T::from_string(s)),
            Value::Variable(v) => Cow::Borrowed(self.get_variable(v)?),
        }))
    }
}

#[derive(Debug, Clone, Copy)]
enum DateTimeKind {
    Date,
    Time,
    TimeTz,
    Timestamp,
    TimestampTz,
}

enum ParsedDateTime {
    Date(NaiveDate),
    Time(NaiveTime),
    TimeTz(NaiveTime, FixedOffset),
    Timestamp(NaiveDateTime),
    TimestampTz(DateTime<FixedOffset>),
}

fn parse_datetime(input: &str, kind: DateTimeKind) -> Option<DateTimeValue> {
    let parsed = parse_iso_datetime(input)?;
    match (kind, parsed) {
        (DateTimeKind::Date, ParsedDateTime::Date(value)) => Some(DateTimeValue::Date(value)),
        (DateTimeKind::Date, ParsedDateTime::Timestamp(value)) => {
            Some(DateTimeValue::Date(value.date()))
        }
        (DateTimeKind::Time, ParsedDateTime::Time(value)) => Some(DateTimeValue::Time(value)),
        (DateTimeKind::Time, ParsedDateTime::Timestamp(value)) => {
            Some(DateTimeValue::Time(value.time()))
        }
        (DateTimeKind::TimeTz, ParsedDateTime::TimeTz(value, offset)) => {
            Some(DateTimeValue::TimeTz(value, offset))
        }
        (DateTimeKind::TimeTz, ParsedDateTime::TimestampTz(value)) => {
            Some(DateTimeValue::TimeTz(value.time(), *value.offset()))
        }
        (DateTimeKind::Timestamp, ParsedDateTime::Date(value)) => Some(DateTimeValue::Timestamp(
            value.and_hms_opt(0, 0, 0).unwrap(),
        )),
        (DateTimeKind::Timestamp, ParsedDateTime::Timestamp(value)) => {
            Some(DateTimeValue::Timestamp(value))
        }
        (DateTimeKind::TimestampTz, ParsedDateTime::TimestampTz(value)) => {
            Some(DateTimeValue::TimestampTz(value))
        }
        _ => None,
    }
}

fn parse_iso_datetime(input: &str) -> Option<ParsedDateTime> {
    let input = input.trim();
    if let Ok(value) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Some(ParsedDateTime::Date(value));
    }

    if let Some((value, offset)) = split_time_zone(input) {
        if let Some(timestamp) = parse_naive_timestamp(value) {
            return offset
                .from_local_datetime(&timestamp)
                .single()
                .map(ParsedDateTime::TimestampTz);
        }
        if let Some(time) = parse_naive_time(value) {
            return Some(ParsedDateTime::TimeTz(time, offset));
        }
    }

    if let Some(value) = parse_naive_timestamp(input) {
        return Some(ParsedDateTime::Timestamp(value));
    }
    parse_naive_time(input).map(ParsedDateTime::Time)
}

fn parse_naive_timestamp(input: &str) -> Option<NaiveDateTime> {
    ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"]
        .into_iter()
        .find_map(|format| NaiveDateTime::parse_from_str(input.trim(), format).ok())
}

fn parse_naive_time(input: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(input.trim(), "%H:%M:%S%.f").ok()
}

fn split_time_zone(input: &str) -> Option<(&str, FixedOffset)> {
    if let Some(value) = input.strip_suffix(['Z', 'z']) {
        let value = value.trim_end();
        if value.contains(':') {
            return Some((value, FixedOffset::east_opt(0).unwrap()));
        }
    }
    let index = input
        .char_indices()
        .rev()
        .find(|(index, ch)| *index > 0 && (*ch == '+' || *ch == '-'))?
        .0;
    let value = input[..index].trim_end();
    if !value.contains(':') {
        return None;
    }
    let offset = input[index..].trim();
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let mut fields = offset[1..].split(':');
    let hour = fields.next()?.parse::<i32>().ok()?;
    let minute = fields
        .next()
        .map(str::parse::<i32>)
        .transpose()
        .ok()?
        .unwrap_or(0);
    if fields.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    FixedOffset::east_opt(sign * (hour * 3600 + minute * 60)).map(|offset| (value, offset))
}

fn number_to_i64(number: &Number) -> Option<i64> {
    if let Some(value) = number.as_i64() {
        return Some(value);
    }
    if let Some(value) = number.as_u64() {
        return value.try_into().ok();
    }
    let value = number.as_f64()?.round();
    if value.is_finite() && value >= i64::MIN as f64 && value < -(i64::MIN as f64) {
        Some(value as i64)
    } else {
        None
    }
}

fn parse_boolean(input: &str) -> Option<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "true" | "t" | "yes" | "y" | "on" | "1" => Some(true),
        "false" | "f" | "no" | "n" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// Compare two values.
///
/// Return unknown if the values are not comparable.
fn eval_compare<T: Json>(op: CompareOp, left: ItemRef<'_, T>, right: ItemRef<'_, T>) -> Truth {
    match (left, right) {
        (ItemRef::Json(left), ItemRef::Json(right)) => eval_compare_json::<T>(op, left, right),
        (ItemRef::DateTime(left), ItemRef::DateTime(right)) => {
            eval_compare_datetime(op, left, right)
        }
        _ => Truth::Unknown,
    }
}

fn eval_compare_datetime(op: CompareOp, left: &DateTimeValue, right: &DateTimeValue) -> Truth {
    let result = match (left, right) {
        (DateTimeValue::Date(left), DateTimeValue::Date(right)) => compare_ord(op, left, right),
        (DateTimeValue::Time(left), DateTimeValue::Time(right)) => compare_ord(op, left, right),
        (
            DateTimeValue::TimeTz(left_time, left_offset),
            DateTimeValue::TimeTz(right_time, right_offset),
        ) => {
            let left = left_time.num_seconds_from_midnight() as i64 * 1_000_000_000
                + left_time.nanosecond() as i64
                - left_offset.local_minus_utc() as i64 * 1_000_000_000;
            let right = right_time.num_seconds_from_midnight() as i64 * 1_000_000_000
                + right_time.nanosecond() as i64
                - right_offset.local_minus_utc() as i64 * 1_000_000_000;
            compare_ord(
                op,
                (left, -left_offset.local_minus_utc()),
                (right, -right_offset.local_minus_utc()),
            )
        }
        (DateTimeValue::Timestamp(left), DateTimeValue::Timestamp(right)) => {
            compare_ord(op, left, right)
        }
        (DateTimeValue::TimestampTz(left), DateTimeValue::TimestampTz(right)) => {
            compare_ord(op, left, right)
        }
        (DateTimeValue::Date(left), DateTimeValue::Timestamp(right)) => {
            compare_ord(op, &left.and_hms_opt(0, 0, 0).unwrap(), right)
        }
        (DateTimeValue::Timestamp(left), DateTimeValue::Date(right)) => {
            compare_ord(op, left, &right.and_hms_opt(0, 0, 0).unwrap())
        }
        _ => return Truth::Unknown,
    };
    result.into()
}

fn eval_compare_json<T: Json>(
    op: CompareOp,
    left: T::Borrowed<'_>,
    right: T::Borrowed<'_>,
) -> Truth {
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
    let n = value.as_number().ok_or(Error::UnaryOperandNotNumeric(op))?;
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
    let left = left.as_number().ok_or(Error::LeftOperandNotNumeric(op))?;
    let right = right.as_number().ok_or(Error::RightOperandNotNumeric(op))?;
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
