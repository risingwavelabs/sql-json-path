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

//! The AST of JSON Path.

use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;

use serde_json::Number;

/// A JSON Path value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPath {
    pub(crate) mode: Mode,
    pub(crate) expr: ExprOrPredicate,
}

/// The mode of JSON Path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Lax mode converts errors to empty SQL/JSON sequences.
    Lax,
    /// Strict mode raises an error if the data does not strictly adhere to the requirements of a path expression.
    Strict,
}

/// An expression or predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprOrPredicate {
    Expr(Expr),
    Pred(Predicate),
}

/// An expression in JSON Path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Path primary
    PathPrimary(PathPrimary),
    /// Accessor expression.
    Accessor(Box<Expr>, AccessorOp),
    /// Unary operation.
    UnaryOp(UnaryOp, Box<Expr>),
    /// Binary operation.
    BinaryOp(BinaryOp, Box<Expr>, Box<Expr>),
}

/// A filter expression that evaluates to a truth value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// `==`, `!=`, `<`, `<=`, `>`, `>=` represents the comparison between two values.
    Compare(CompareOp, Box<Expr>, Box<Expr>),
    /// `exists` represents the value exists.
    Exists(Box<Expr>),
    /// `&&` represents logical AND.
    And(Box<Predicate>, Box<Predicate>),
    /// `||` represents logical OR.
    Or(Box<Predicate>, Box<Predicate>),
    /// `!` represents logical NOT.
    Not(Box<Predicate>),
    /// `is unknown` represents the value is unknown.
    IsUnknown(Box<Predicate>),
    /// `starts with` represents the value starts with the given value.
    StartsWith(Box<Expr>, Value),
    /// `like_regex` represents the value matches the given regular expression.
    LikeRegex(Box<Expr>, Box<Regex>),
}

/// A primary expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathPrimary {
    /// `$` represents the root node or element.
    Root,
    /// `@` represents the current node or element being processed in the filter expression.
    Current,
    /// `last` is the size of the array minus 1.
    Last,
    /// Literal value.
    Value(Value),
    /// `(expr)` represents an expression.
    ExprOrPred(Box<ExprOrPredicate>),
}

/// An accessor operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessorOp {
    /// `.*` represents selecting all elements in an object.
    MemberWildcard,
    /// `.**` represents selecting all elements in an object and its sub-objects.
    DescendantMemberWildcard(LevelRange),
    /// `[*]` represents selecting all elements in an array.
    ElementWildcard,
    /// `.<name>` represents selecting element that matched the name in an object, like `$.event`.
    /// The name can also be written as a string literal, allowing the name to contain special characters, like `$." $price"`.
    Member(String),
    /// `[<index1>,<index2>,..]` represents selecting elements specified by the indices in an Array.
    Element(Vec<ArrayIndex>),
    /// `?(<predicate>)` represents filtering elements using the predicate.
    FilterExpr(Box<Predicate>),
    /// `.method()` represents calling a method.
    Method(Method),
}

/// A level range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelRange {
    /// none
    All,
    /// `{level}`
    One(Level),
    /// `{start to end}`
    Range(Level, Level),
}

/// A level number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Level {
    N(u32),
    Last,
}

/// An array index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIndex {
    /// The single number index.
    Index(Expr),
    /// `<start> to <end>` represents the slice of the array.
    Slice(Expr, Expr),
}

/// Represents a scalar value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Null value.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Number value.
    Number(Number),
    /// UTF-8 string.
    String(String),
    /// Variable
    Variable(String),
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// `==` represents left is equal to right.
    Eq,
    /// `!=` and `<>` represents left is not equal to right.
    Ne,
    /// `<` represents left is less than right.
    Lt,
    /// `<=` represents left is less or equal to right.
    Le,
    /// `>` represents left is greater than right.
    Gt,
    /// `>=` represents left is greater than or equal to right.
    Ge,
}

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `+` represents plus.
    Plus,
    /// `-` represents minus.
    Minus,
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+` represents left plus right.
    Add,
    /// `-` represents left minus right.
    Sub,
    /// `*` represents left multiply right.
    Mul,
    /// `/` represents left divide right.
    Div,
    /// `%` represents left modulo right.
    Rem,
}

/// A item method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    /// `.type()` returns a character string that names the type of the SQL/JSON item.
    Type,
    /// `.size()` returns the size of an SQL/JSON item.
    Size,
    /// `.double()` converts a string or numeric to an approximate numeric value.
    Double,
    /// `.ceiling()` returns the smallest integer that is greater than or equal to the argument.
    Ceiling,
    /// `.floor()` returns the largest integer that is less than or equal to the argument.
    Floor,
    /// `.abs()` returns the absolute value of the argument.
    Abs,
    /// `.keyvalue()` returns the key-value pairs of an object.
    ///
    /// For example, suppose:
    /// ```json
    /// { who: "Fred", what: 64 }
    /// ```
    /// Then:
    /// ```json
    /// $.keyvalue() =
    /// ( { name: "who",  value: "Fred", id: 9045 },
    ///   { name: "what", value: 64,     id: 9045 }
    /// )
    /// ```
    Keyvalue,
}

impl PathPrimary {
    /// If this is a nested path primary, unnest it.
    /// `(primary) => primary`
    pub(crate) fn unnest(self) -> Self {
        match self {
            Self::ExprOrPred(expr) => match *expr {
                ExprOrPredicate::Expr(Expr::PathPrimary(inner)) => inner,
                other => Self::ExprOrPred(Box::new(other)),
            },
            _ => self,
        }
    }
}

impl LevelRange {
    /// Returns the upper bound of the range.
    /// If no upper bound, returns `u32::MAX`.
    pub(crate) fn end(&self) -> u32 {
        match self {
            Self::One(Level::N(n)) => *n,
            Self::Range(_, Level::N(end)) => *end,
            _ => u32::MAX,
        }
    }

    /// Resolve the range with the given `last`.
    ///
    /// # Examples
    ///
    /// ```text
    /// last = 3
    /// .**             => 0..4
    /// .**{1}          => 1..2
    /// .**{1 to 4}     => 1..3
    /// .**{1 to last}  => 1..4
    /// .**{last to 2}  => 3..3
    /// ```
    pub(crate) fn to_range(&self, last: usize) -> std::ops::Range<usize> {
        match self {
            Self::All => 0..last + 1,
            Self::One(level) => {
                level.to_usize(last).min(last + 1)..level.to_usize(last).min(last) + 1
            }
            Self::Range(start, end) => {
                start.to_usize(last).min(last + 1)..end.to_usize(last).min(last) + 1
            }
        }
    }
}

impl Level {
    fn to_usize(&self, last: usize) -> usize {
        match self {
            Self::N(n) => *n as usize,
            Self::Last => last,
        }
    }
}

impl Display for JsonPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.mode == Mode::Strict {
            write!(f, "strict ")?;
        }
        write!(f, "{}", self.expr)
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lax => write!(f, "lax"),
            Self::Strict => write!(f, "strict"),
        }
    }
}

impl Display for ExprOrPredicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expr(expr) => match expr {
                Expr::BinaryOp(_, _, _) => write!(f, "({})", expr),
                _ => write!(f, "{}", expr),
            },
            Self::Pred(pred) => match pred {
                Predicate::Compare(_, _, _) | Predicate::And(_, _) | Predicate::Or(_, _) => {
                    write!(f, "({})", pred)
                }
                _ => write!(f, "{}", pred),
            },
        }
    }
}

impl Display for Predicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compare(op, left, right) => write!(f, "{left} {op} {right}"),
            Self::Exists(expr) => write!(f, "exists ({expr})"),
            Self::And(left, right) => {
                match left.as_ref() {
                    Self::Or(_, _) => write!(f, "({left})")?,
                    _ => write!(f, "{left}")?,
                }
                write!(f, " && ")?;
                match right.as_ref() {
                    Self::Or(_, _) => write!(f, "({right})"),
                    _ => write!(f, "{right}"),
                }
            }
            Self::Or(left, right) => write!(f, "{left} || {right}"),
            Self::Not(expr) => write!(f, "!({expr})"),
            Self::IsUnknown(expr) => write!(f, "({expr}) is unknown"),
            Self::StartsWith(expr, v) => write!(f, "{expr} starts with {v}"),
            Self::LikeRegex(expr, regex) => {
                write!(f, "{expr} like_regex \"{}\"", regex.pattern())?;
                if let Some(flags) = regex.flags() {
                    write!(f, " flag \"{flags}\"")?;
                }
                Ok(())
            }
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::PathPrimary(primary) => write!(f, "{primary}"),
            Expr::Accessor(base, op) => {
                match base.as_ref() {
                    Expr::PathPrimary(PathPrimary::Value(Value::Number(_))) => {
                        write!(f, "({base})")?
                    }
                    Expr::PathPrimary(PathPrimary::ExprOrPred(expr)) => match expr.as_ref() {
                        ExprOrPredicate::Expr(Expr::UnaryOp(_, _)) => write!(f, "({base})")?,
                        _ => write!(f, "{base}")?,
                    },
                    _ => write!(f, "{base}")?,
                }
                write!(f, "{op}")?;
                Ok(())
            }
            Expr::UnaryOp(op, expr) => match expr.as_ref() {
                Expr::PathPrimary(_) | Expr::Accessor(_, _) => write!(f, "{op}{expr}"),
                _ => write!(f, "{op}({expr})"),
            },
            Expr::BinaryOp(op, left, right) => write!(f, "{left} {op} {right}"),
        }
    }
}

impl Display for ArrayIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index(idx) => write!(f, "{idx}"),
            Self::Slice(start, end) => write!(f, "{start} to {end}"),
        }
    }
}

impl Display for PathPrimary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root => write!(f, "$"),
            Self::Current => write!(f, "@"),
            Self::Value(v) => write!(f, "{v}"),
            Self::Last => write!(f, "last"),
            Self::ExprOrPred(expr) => write!(f, "{expr}"),
        }
    }
}

impl Display for AccessorOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemberWildcard => write!(f, ".*"),
            Self::DescendantMemberWildcard(level) => write!(f, ".**{level}"),
            Self::ElementWildcard => write!(f, "[*]"),
            Self::Member(field) => write!(f, ".\"{field}\""),
            Self::Element(indices) => {
                write!(f, "[")?;
                for (i, idx) in indices.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{idx}")?;
                }
                write!(f, "]")
            }
            Self::FilterExpr(expr) => write!(f, "?({expr})"),
            Self::Method(method) => write!(f, ".{method}()"),
        }
    }
}

impl Display for LevelRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => Ok(()),
            Self::One(level) => write!(f, "{{{level}}}"),
            Self::Range(start, end) => write!(f, "{{{start} to {end}}}"),
        }
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::N(n) => write!(f, "{n}"),
            Self::Last => write!(f, "last"),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(v) => write!(f, "{v}"),
            Self::Number(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "\"{v}\""),
            Self::Variable(v) => write!(f, "$\"{v}\""),
        }
    }
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
        }
    }
}

impl Display for CompareOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "=="),
            Self::Ne => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::Le => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::Ge => write!(f, ">="),
        }
    }
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Rem => write!(f, "%"),
        }
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type => write!(f, "type"),
            Self::Size => write!(f, "size"),
            Self::Double => write!(f, "double"),
            Self::Ceiling => write!(f, "ceiling"),
            Self::Floor => write!(f, "floor"),
            Self::Abs => write!(f, "abs"),
            Self::Keyvalue => write!(f, "keyvalue"),
        }
    }
}

/// A wrapper of `regex::Regex` to combine the pattern and flags.
#[derive(Debug, Clone)]
pub struct Regex {
    regex: regex::Regex,
    flags: String,
}

impl Regex {
    pub(crate) fn with_flags(pattern: &str, flags: Option<String>) -> Result<Self, regex::Error> {
        let mut builder = match flags.as_deref() {
            Some(flags) if flags.contains('q') => regex::RegexBuilder::new(&regex::escape(pattern)),
            _ => regex::RegexBuilder::new(pattern),
        };
        let mut out_flags = String::new();
        if let Some(flags) = flags.as_deref() {
            for c in flags.chars() {
                match c {
                    'q' => {}
                    'i' => {
                        builder.case_insensitive(true);
                    }
                    'm' => {
                        builder.multi_line(true);
                    }
                    's' => {
                        builder.dot_matches_new_line(true);
                    }
                    'x' => {
                        return Err(regex::Error::Syntax(
                            "XQuery \"x\" flag (expanded regular expressions) is not implemented"
                                .to_string(),
                        ))
                    }
                    _ => {
                        return Err(regex::Error::Syntax(format!(
                            "Unrecognized flag character \"{c}\" in LIKE_REGEX predicate."
                        )))
                    }
                };
                // Remove duplicated flags.
                if !out_flags.contains(c) {
                    out_flags.push(c);
                }
            }
        }
        let regex = builder.build()?;
        Ok(Self {
            regex,
            flags: out_flags,
        })
    }

    pub fn pattern(&self) -> &str {
        self.regex.as_str()
    }

    pub fn flags(&self) -> Option<&str> {
        if self.flags.is_empty() {
            None
        } else {
            Some(&self.flags)
        }
    }
}

impl Deref for Regex {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.regex
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.pattern() == other.pattern() && self.flags() == other.flags()
    }
}

impl Eq for Regex {}
