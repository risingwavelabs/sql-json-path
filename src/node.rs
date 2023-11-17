//! The AST of JSON Path.

use std::fmt::Display;
use std::fmt::Formatter;

use serde_json::Number;

/// Represents a set of JSON Path chains.
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
    /// Accessor expression.
    Accessor(PathPrimary, Vec<AccessorOp>),
    /// Unary operation.
    UnaryOp(UnaryOp, Box<Expr>),
    /// Binary operation.
    BinaryOp(BinaryOp, Box<Expr>, Box<Expr>),
}

///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// Compare operation
    Compare(CompareOp, Box<Expr>, Box<Expr>),
    Exists(Box<Expr>),
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
    IsUnknown(Box<Predicate>),
    StartsWith(Box<Expr>, Value),
    LikeRegex(Box<Expr>, String, Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathPrimary {
    /// `$` represents the root node or element.
    Root,
    /// `@` represents the current node or element being processed in the filter expression.
    Current,
    /// Literal value.
    Value(Value),
    /// `last` is the size of the array minus 1.
    Last,
    /// `(expr)` represents an expression.
    Expr(Box<Expr>),
}

/// Represents a valid JSON Path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessorOp {
    /// `.*` represents selecting all elements in an Object.
    MemberWildcard,
    /// `[*]` represents selecting all elements in an Array.
    ElementWildcard,
    /// `.<name>` represents selecting element that matched the name in an Object, like `$.event`.
    /// The name can also be written as a string literal, allowing the name to contain special characters, like `$." $price"`.
    Member(String),
    /// `[<index1>,<index2>,..]` represents selecting elements specified by the indices in an Array.
    Element(Vec<ArrayIndex>),
    /// `?(<expression>)` represents selecting all elements in an object or array that match the filter expression, like `$.book[?(@.price < 10)]`.
    FilterExpr(Box<Predicate>),
    /// `.method()` represents calling a method.
    Method(Method),
}

/// Represents the index in an Array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIndex {
    /// The single number index.
    Index(Expr),
    /// The range index between two number.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    Type,
    Size,
    Double,
    Ceiling,
    Floor,
    Abs,
    Keyvalue,
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
            Self::Expr(expr) => write!(f, "{}", expr),
            Self::Pred(pred) => write!(f, "{}", pred),
        }
    }
}

impl Display for Predicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compare(op, left, right) => write!(f, "({left} {op} {right})"),
            Self::Exists(expr) => write!(f, "exists({expr})"),
            Self::And(left, right) => write!(f, "({left} && {right})"),
            Self::Or(left, right) => write!(f, "({left} || {right})"),
            Self::Not(expr) => write!(f, "!({expr})"),
            Self::IsUnknown(expr) => write!(f, "{expr} is unknown"),
            Self::StartsWith(expr, v) => write!(f, "({expr} starts with {v})"),
            Self::LikeRegex(expr, v, flag) => {
                write!(f, "({expr} like_regex {v}")?;
                if let Some(flag) = flag {
                    write!(f, " flag {flag}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Accessor(primary, ops) => {
                write!(f, "{primary}")?;
                for op in ops {
                    write!(f, "{op}")?;
                }
                Ok(())
            }
            Expr::UnaryOp(op, expr) => write!(f, "{op}{expr}"),
            Expr::BinaryOp(op, left, right) => write!(f, "({left} {op} {right})"),
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
            Self::Expr(expr) => write!(f, "({expr})"),
        }
    }
}

impl Display for AccessorOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemberWildcard => write!(f, ".*"),
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

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(v) => write!(f, "{v}"),
            Self::Number(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "\"{v}\""),
            Self::Variable(v) => write!(f, "${v}"),
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
