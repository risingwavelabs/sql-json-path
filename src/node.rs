//! The AST of JSON Path.

use std::cmp::Ordering;
use std::fmt::Display;
use std::fmt::Formatter;

use serde_json::Number;

/// Represents a set of JSON Path chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPath {
    pub(crate) mode: Mode,
    pub(crate) expr: Expr,
}

/// The mode of JSON Path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    /// Lax mode converts errors to empty SQL/JSON sequences.
    #[default]
    Lax,
    /// Strict mode raises an error if the data does not strictly adhere to the requirements of a path expression.
    Strict,
}

/// An expression in JSON Path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Accessor expression.
    Accessor(PathPrimary, Vec<AccessorOp>),
    /// Unary operation.
    UnaryOp { op: UnaryOp, expr: Box<Expr> },
    /// Binary operation.
    BinaryOp {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    pub(crate) fn value(v: Value) -> Self {
        Self::Accessor(PathPrimary::Value(v), vec![])
    }

    pub(crate) fn unary(op: UnaryOp, expr: Self) -> Self {
        Self::UnaryOp {
            op,
            expr: Box::new(expr),
        }
    }

    pub(crate) fn binary(op: BinaryOp, left: Self, right: Self) -> Self {
        Self::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathPrimary {
    /// `$` represents the root node or element.
    Root,
    /// `@` represents the current node or element being processed in the filter expression.
    Current,
    /// Literal value.
    Value(Value),
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
    /// There are several forms of index.
    /// 1. A single number representing the 0-based `n-th` element in the Array.
    ///    e.g. `[0]` represents the first element in an Array.
    /// 2. The keyword `last` represents the last element in the Array,
    ///    and last minus a number represents the n-th element before the last element,
    ///    e.g. `[last-1]` represents the penultimate element.
    /// 3. The keyword `to` between two numbers represent all elements of a range in an Array,
    ///    e.g. `[1 to last]` represents all the elements in the Array from the second to the last.
    ///
    /// There can be more than one index, e.g. `$[0, last-1 to last, 5]` represents the first,
    /// the last two, and the sixth element in an Array.
    Element(Vec<ArrayIndex>),
    /// `?(<expression>)` represents selecting all elements in an object or array that match the filter expression, like `$.book[?(@.price < 10)]`.
    FilterExpr(Box<Expr>),
    /// `.method()` represents calling a method.
    Method(Method),
}

/// Represents the single index in an Array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Index {
    /// The 0-based index in an Array.
    Index(i32),
    /// The last n-th index in an Array.
    LastIndex(i32),
}

/// Represents the index in an Array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIndex {
    /// The single number index.
    Index(Index),
    /// The range index between two number.
    Slice((Index, Index)),
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

/// A unary operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    /// `+` represents plus.
    Plus,
    /// `-` represents minus.
    Minus,
    /// `!` represents logical Not operation.
    Not,
    /// `exists`
    Exists,
    /// `is unknown`
    IsUnknown,
}

/// A binary operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    /// `&&` represents logical And operation.
    And,
    /// `||` represents logical Or operation.
    Or,
    /// `==` represents left is equal to right.
    Eq,
    /// `!=` and `<>` represents left is not equal to right.
    NotEq,
    /// `<` represents left is less than right.
    Lt,
    /// `<=` represents left is less or equal to right.
    Le,
    /// `>` represents left is greater than right.
    Gt,
    /// `>=` represents left is greater than or equal to right.
    Ge,
    /// `+` represents left plus right.
    Add,
    /// `-` represents left minus right.
    Sub,
    /// `*` represents left multiply right.
    Mul,
    /// `/` represents left divide right.
    Div,
    /// `%` represents left mod right.
    Mod,
    /// `starts with`
    StartsWith,
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
        write!(f, "{} {}", self.mode, self.expr)
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

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Accessor(primary, ops) => {
                write!(f, "{primary}")?;
                for op in ops {
                    write!(f, " {op}")?;
                }
                Ok(())
            }
            Expr::UnaryOp { op, expr } => write!(f, "{op} {expr}"),
            Expr::BinaryOp { op, left, right } => write!(f, "({left} {op} {right})"),
        }
    }
}

impl Display for ArrayIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index(idx) => write!(f, "{idx}"),
            Self::Slice((start, end)) => write!(f, "{start} to {end}"),
        }
    }
}

impl Display for Index {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Index::Index(idx) => write!(f, "{idx}"),
            Index::LastIndex(idx) => {
                write!(f, "last")?;
                match idx.cmp(&0) {
                    Ordering::Greater => write!(f, "+{idx}"),
                    Ordering::Less => write!(f, "{idx}"),
                    Ordering::Equal => Ok(()),
                }
            }
        }
    }
}

impl Display for PathPrimary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root => write!(f, "$"),
            Self::Current => write!(f, "@"),
            Self::Value(v) => write!(f, "{v}"),
        }
    }
}

impl Display for AccessorOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemberWildcard => write!(f, ".*"),
            Self::ElementWildcard => write!(f, "[*]"),
            Self::Member(field) => write!(f, ".{field}"),
            Self::Element(indices) => f.debug_list().entries(indices).finish(),
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
            Self::Not => write!(f, "!"),
            Self::Exists => write!(f, "exists"),
            Self::IsUnknown => write!(f, "is unknown"),
        }
    }
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Eq => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::Le => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::Ge => write!(f, ">="),
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
            Self::StartsWith => write!(f, "starts with"),
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
