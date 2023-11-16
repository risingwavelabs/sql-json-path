use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Display;
use std::fmt::Formatter;

use serde_json::Number;

/// Represents a set of JSON Path chains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPath<'a> {
    pub(crate) mode: Mode,
    pub(crate) expr: Expr<'a>,
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
pub enum Expr<'a> {
    /// Accessor expression.
    Accessor(PathPrimary, Vec<AccessorOp<'a>>),
    /// Literal value.
    Value(Value<'a>),
    /// Unary operation.
    UnaryOp { op: UnaryOp, expr: Box<Expr<'a>> },
    /// Binary operation.
    BinaryOp {
        op: BinaryOp,
        left: Box<Expr<'a>>,
        right: Box<Expr<'a>>,
    },
}

impl Expr<'_> {
    pub(crate) fn unary(op: UnaryOp, expr: Expr) -> Self {
        Self::UnaryOp {
            op,
            expr: Box::new(expr),
        }
    }

    pub(crate) fn binary(op: BinaryOp, left: Expr, right: Expr) -> Self {
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
}

/// Represents a valid JSON Path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessorOp<'a> {
    /// `.*` represents selecting all elements in an Object.
    MemberWildcard,
    /// `[*]` represents selecting all elements in an Array.
    ElementWildcard,
    /// `.<name>` represents selecting element that matched the name in an Object, like `$.event`.
    /// The name can also be written as a string literal, allowing the name to contain special characters, like `$." $price"`.
    Member(Cow<'a, str>),
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
    FilterExpr(Box<Expr<'a>>),
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
pub enum Value<'a> {
    /// Null value.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Number value.
    Number(Number),
    /// UTF-8 string.
    String(Cow<'a, str>),
    /// Variable
    Variable(Cow<'a, str>),
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

impl<'a> Display for JsonPath<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.mode, self.expr)
    }
}

impl<'a> Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lax => write!(f, "lax"),
            Self::Strict => write!(f, "strict"),
        }
    }
}

impl<'a> Display for Expr<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Accessor(primary, ops) => {
                write!(f, "{primary}")?;
                for op in ops {
                    write!(f, " {op}")?;
                }
                Ok(())
            }
            Expr::Value(v) => write!(f, "{v}"),
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

impl<'a> Display for PathPrimary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root => write!(f, "$"),
            Self::Current => write!(f, "@"),
        }
    }
}

impl<'a> Display for AccessorOp<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemberWildcard => write!(f, ".*"),
            Self::ElementWildcard => write!(f, "[*]"),
            Self::Member(field) => write!(f, ".{field}"),
            Self::Element(indices) => f.debug_list().entries(indices).finish(),
            Self::FilterExpr(expr) => write!(f, "?({expr})"),
        }
    }
}

impl<'a> Display for Value<'a> {
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
