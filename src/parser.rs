//! JSON Path parser.

use crate::node::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, i64, multispace0 as s, u64},
    combinator::{cut, map, value, verify},
    error::context,
    multi::{fold_many0, many0, separated_list1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    Finish, IResult, Offset,
};
use serde_json::Number;
use std::str::FromStr;

impl JsonPath {
    /// Compiles a JSON Path expression.
    pub fn new(s: &str) -> Result<Self, Error> {
        Self::from_str(s)
    }
}

impl FromStr for JsonPath {
    type Err = Error;

    /// Parse a JSON Path from string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err(Error {
                position: 0,
                message: "empty jsonpath".into(),
            });
        }
        let (rest, json_path) = json_path(s)
            .finish()
            .map_err(|e| Error::from_input_error(s, e))?;
        if !rest.is_empty() {
            return Err(Error {
                position: s.offset(rest),
                message: format!("unexpected trailing characters: {rest}").into(),
            });
        }
        Checker::default()
            .visit_json_path(&json_path)
            .map_err(|msg| Error {
                position: 0,
                message: msg.into(),
            })?;
        Ok(json_path)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("at position {position}, {message}")]
pub struct Error {
    position: usize,
    message: Box<str>,
}

impl Error {
    fn from_input_error(input: &str, err: nom::error::Error<&str>) -> Self {
        let position = input.offset(err.input);
        let message = err.to_string().into();
        Self { position, message }
    }
}

fn json_path(input: &str) -> IResult<&str, JsonPath> {
    map(
        delimited(
            s,
            separated_pair(
                mode,
                s,
                alt((
                    map(predicate, ExprOrPredicate::Pred),
                    map(expr, ExprOrPredicate::Expr),
                )),
            ),
            s,
        ),
        |(mode, expr)| JsonPath { mode, expr },
    )(input)
}

fn mode(input: &str) -> IResult<&str, Mode> {
    alt((
        value(Mode::Strict, tag("strict")),
        value(Mode::Lax, tag("lax")),
        value(Mode::Lax, tag("")),
    ))(input)
}

fn predicate(input: &str) -> IResult<&str, Predicate> {
    let (input, first) = predicate1(input)?;
    let mut first0 = Some(first);
    fold_many0(
        preceded(delimited(s, tag("||"), s), predicate1),
        move || first0.take().unwrap(),
        |acc, pred| Predicate::Or(Box::new(acc), Box::new(pred)),
    )(input)
}

fn predicate1(input: &str) -> IResult<&str, Predicate> {
    let (input, first) = predicate2(input)?;
    let mut first0 = Some(first);
    fold_many0(
        preceded(delimited(s, tag("&&"), s), predicate2),
        move || first0.take().unwrap(),
        |acc, pred| Predicate::And(Box::new(acc), Box::new(pred)),
    )(input)
}

fn predicate2(input: &str) -> IResult<&str, Predicate> {
    alt((
        delimited_predicate,
        map(
            tuple((expr, delimited(s, cmp_op, s), expr)),
            |(left, op, right)| Predicate::Compare(op, Box::new(left), Box::new(right)),
        ),
        map(
            delimited(
                pair(char('('), s),
                predicate,
                tuple((s, char(')'), s, tag("is"), s, tag("unknown"))),
            ),
            |p| Predicate::IsUnknown(Box::new(p)),
        ),
        map(
            separated_pair(
                expr,
                tuple((s, tag("starts"), s, tag("with"), s)),
                starts_with_literal,
            ),
            |(expr, literal)| Predicate::StartsWith(Box::new(expr), literal),
        ),
        map(preceded(pair(tag("!"), s), delimited_predicate), |p| {
            Predicate::Not(Box::new(p))
        }),
    ))(input)
}

fn delimited_predicate(input: &str) -> IResult<&str, Predicate> {
    alt((
        delimited(pair(char('('), s), predicate, pair(s, char(')'))),
        map(
            delimited(
                tuple((tag("exists"), s, char('('), s)),
                expr,
                pair(s, char(')')),
            ),
            |expr| Predicate::Exists(Box::new(expr)),
        ),
    ))(input)
}

fn expr(input: &str) -> IResult<&str, Expr> {
    let (input, first) = expr1(input)?;
    let mut first0 = Some(first);
    fold_many0(
        pair(delimited(s, alt((char('+'), char('-'))), s), expr1),
        move || first0.take().unwrap(),
        |acc, (op, expr)| match op {
            '+' => Expr::BinaryOp(BinaryOp::Add, Box::new(acc), Box::new(expr)),
            '-' => Expr::BinaryOp(BinaryOp::Sub, Box::new(acc), Box::new(expr)),
            _ => unreachable!(),
        },
    )(input)
}

fn expr1(input: &str) -> IResult<&str, Expr> {
    let (input, first) = expr2(input)?;
    let mut first0 = Some(first);
    fold_many0(
        pair(
            delimited(s, alt((char('*'), char('/'), char('%'))), s),
            expr2,
        ),
        move || first0.take().unwrap(),
        |acc, (op, expr)| match op {
            '*' => Expr::BinaryOp(BinaryOp::Mul, Box::new(acc), Box::new(expr)),
            '/' => Expr::BinaryOp(BinaryOp::Div, Box::new(acc), Box::new(expr)),
            '%' => Expr::BinaryOp(BinaryOp::Rem, Box::new(acc), Box::new(expr)),
            _ => unreachable!(),
        },
    )(input)
}

fn expr2(input: &str) -> IResult<&str, Expr> {
    alt((
        accessor_expr,
        delimited(pair(char('('), s), expr, pair(s, char(')'))),
        map(preceded(pair(char('+'), s), expr), |expr| {
            Expr::UnaryOp(UnaryOp::Plus, Box::new(expr))
        }),
        map(preceded(pair(char('-'), s), expr), |expr| {
            Expr::UnaryOp(UnaryOp::Minus, Box::new(expr))
        }),
    ))(input)
}

fn accessor_expr(input: &str) -> IResult<&str, Expr> {
    map(
        pair(path_primary, many0(preceded(s, accessor_op))),
        |(primary, ops)| Expr::Accessor(primary, ops),
    )(input)
}

fn path_primary(input: &str) -> IResult<&str, PathPrimary> {
    alt((
        value(PathPrimary::Root, char('$')),
        value(PathPrimary::Current, char('@')),
        map(scalar_value, PathPrimary::Value),
        value(PathPrimary::Last, tag("last")),
    ))(input)
}

fn accessor_op(input: &str) -> IResult<&str, AccessorOp> {
    alt((
        value(AccessorOp::MemberWildcard, tag(".*")),
        value(AccessorOp::ElementWildcard, element_wildcard),
        map(item_method, AccessorOp::Method),
        map(member_accessor, AccessorOp::Member),
        map(array_accessor, AccessorOp::Element),
        map(filter_expr, |expr| AccessorOp::FilterExpr(Box::new(expr))),
    ))(input)
}

fn element_wildcard(input: &str) -> IResult<&str, ()> {
    value((), tuple((char('['), s, char('*'), s, char(']'))))(input)
}

fn member_accessor(input: &str) -> IResult<&str, String> {
    preceded(pair(char('.'), s), alt((string, raw_string)))(input)
}

fn array_accessor(input: &str) -> IResult<&str, Vec<ArrayIndex>> {
    delimited(
        char('['),
        separated_list1(char(','), delimited(s, index_elem, s)),
        char(']'),
    )(input)
}

fn index_elem(input: &str) -> IResult<&str, ArrayIndex> {
    alt((
        map(
            separated_pair(expr, delimited(s, tag_no_case("to"), s), expr),
            |(start, end)| ArrayIndex::Slice(start, end),
        ),
        map(expr, ArrayIndex::Index),
    ))(input)
}

fn filter_expr(input: &str) -> IResult<&str, Predicate> {
    delimited(
        tuple((char('?'), s, char('('), s)),
        predicate,
        tuple((s, char(')'))),
    )(input)
}

fn cmp_op(input: &str) -> IResult<&str, CompareOp> {
    alt((
        value(CompareOp::Eq, tag("==")),
        value(CompareOp::Ne, tag("!=")),
        value(CompareOp::Ne, tag("<>")),
        value(CompareOp::Le, tag("<=")),
        value(CompareOp::Lt, char('<')),
        value(CompareOp::Ge, tag(">=")),
        value(CompareOp::Gt, char('>')),
    ))(input)
}

fn item_method(input: &str) -> IResult<&str, Method> {
    delimited(
        pair(char('.'), s),
        method,
        tuple((s, char('('), s, char(')'))),
    )(input)
}

fn method(input: &str) -> IResult<&str, Method> {
    alt((
        value(Method::Type, tag("type")),
        value(Method::Size, tag("size")),
        value(Method::Double, tag("double")),
        value(Method::Ceiling, tag("ceiling")),
        value(Method::Floor, tag("floor")),
        value(Method::Abs, tag("abs")),
        value(Method::Keyvalue, tag("keyvalue")),
    ))(input)
}

fn scalar_value(input: &str) -> IResult<&str, Value> {
    alt((
        value(Value::Null, tag("null")),
        value(Value::Boolean(true), tag("true")),
        value(Value::Boolean(false), tag("false")),
        map(u64, |v| Value::Number(Number::from(v))),
        map(i64, |v| Value::Number(Number::from(v))),
        map(double, |v| Value::Number(Number::from_f64(v).unwrap())),
        map(string, Value::String),
        map(variable, Value::Variable),
    ))(input)
}

fn starts_with_literal(input: &str) -> IResult<&str, Value> {
    alt((map(string, Value::String), map(variable, Value::Variable)))(input)
}

fn variable(input: &str) -> IResult<&str, String> {
    preceded(char('$'), raw_string)(input)
}

fn string(input: &str) -> IResult<&str, String> {
    context(
        "double quoted string",
        delimited(
            char('"'),
            fold_many0(
                alt((
                    map(unescaped_str, String::from),
                    map(escaped_char, String::from),
                )),
                String::new,
                |mut string, fragment| {
                    if string.is_empty() {
                        fragment
                    } else {
                        string.push_str(&fragment);
                        string
                    }
                },
            ),
            cut(char('"')),
        ),
    )(input)
}

fn escaped_char(input: &str) -> IResult<&str, char> {
    context(
        "escaped character",
        preceded(
            char('\\'),
            alt((
                value('\u{0008}', char('b')),
                value('\u{0009}', char('t')),
                value('\u{000A}', char('n')),
                value('\u{000C}', char('f')),
                value('\u{000D}', char('r')),
                value('\u{002F}', char('/')),
                value('\u{005C}', char('\\')),
                value('\u{0022}', char('"')),
                // unicode_sequence,
            )),
        ),
    )(input)
}

fn unescaped_str(input: &str) -> IResult<&str, &str> {
    context(
        "unescaped character",
        verify(take_while(|chr| is_valid_unescaped_char(chr)), |s: &str| {
            !s.is_empty()
        }),
    )(input)
}

fn is_valid_unescaped_char(chr: char) -> bool {
    match chr {
        '"' => false,
        '\u{20}'..='\u{5B}' // Omit control characters
        | '\u{5D}'..='\u{10FFFF}' => true, // Omit \
        _ => false,
    }
}

fn raw_string(input: &str) -> IResult<&str, String> {
    map(
        take_while1(|c: char| c.is_ascii_alphanumeric() || c == '_' || c >= '\u{0080}'),
        String::from,
    )(input)
}

/// A visitor that checks if a JSON Path is valid.
#[derive(Debug, Clone, Copy, Default)]
struct Checker {
    non_root: bool,
    inside_element_accessor: bool,
}

impl Checker {
    fn visit_json_path(&self, json_path: &JsonPath) -> Result<(), &'static str> {
        self.visit_expr_or_predicate(&json_path.expr)
    }

    fn visit_expr_or_predicate(&self, expr_or_pred: &ExprOrPredicate) -> Result<(), &'static str> {
        match expr_or_pred {
            ExprOrPredicate::Expr(expr) => self.visit_expr(expr),
            ExprOrPredicate::Pred(pred) => self.visit_predicate(pred),
        }
    }

    fn visit_expr(&self, expr: &Expr) -> Result<(), &'static str> {
        match expr {
            Expr::Accessor(primary, accessors) => {
                for accessor in accessors {
                    self.visit_accessor_op(accessor)?;
                }
                self.visit_path_primary(primary)
            }
            Expr::UnaryOp(_, expr) => self.visit_expr(expr),
            Expr::BinaryOp(_, left, right) => {
                self.visit_expr(left)?;
                self.visit_expr(right)
            }
        }
    }

    fn visit_predicate(&self, pred: &Predicate) -> Result<(), &'static str> {
        match pred {
            Predicate::Compare(_, left, right) => {
                self.visit_expr(left)?;
                self.visit_expr(right)
            }
            Predicate::Exists(expr) => self.visit_expr(expr),
            Predicate::And(left, right) | Predicate::Or(left, right) => {
                self.visit_predicate(left)?;
                self.visit_predicate(right)
            }
            Predicate::Not(pred) => self.visit_predicate(pred),
            Predicate::IsUnknown(pred) => self.visit_predicate(pred),
            Predicate::StartsWith(expr, _) => self.visit_expr(expr),
        }
    }

    fn visit_path_primary(&self, primary: &PathPrimary) -> Result<(), &'static str> {
        match primary {
            PathPrimary::Last if !self.inside_element_accessor => {
                Err("LAST is allowed only in array subscripts")
            }
            PathPrimary::Current if !self.non_root => Err("@ is not allowed in root expressions"),
            _ => Ok(()),
        }
    }

    fn visit_accessor_op(&self, accessor_op: &AccessorOp) -> Result<(), &'static str> {
        match accessor_op {
            AccessorOp::ElementWildcard | AccessorOp::MemberWildcard => Ok(()),
            AccessorOp::Member(_) | AccessorOp::Method(_) => Ok(()),
            AccessorOp::FilterExpr(pred) => Self {
                non_root: true,
                ..*self
            }
            .visit_predicate(pred),
            AccessorOp::Element(indices) => {
                let next = Self {
                    non_root: true,
                    inside_element_accessor: true,
                };
                for index in indices {
                    match index {
                        ArrayIndex::Index(i) => next.visit_expr(i)?,
                        ArrayIndex::Slice(s, e) => {
                            next.visit_expr(s)?;
                            next.visit_expr(e)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_path() {
        JsonPath::from_str(r#"lax $.name ? (@ starts with "O''")"#).unwrap();
        JsonPath::from_str(r#"lax $.name ? (@ starts with "\"hello")"#).unwrap();
        // JsonPath::from_str(r#"lax $.name ? (@ starts with "O\u0027")"#).unwrap();
        // JsonPath::from_str(r#"lax $.name ? (@ starts with "\u0022hello")"#).unwrap();
    }
}
