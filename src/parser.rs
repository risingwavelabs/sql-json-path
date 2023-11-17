//! JSON Path parser.

use crate::node::*;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, i32, i64, multispace0 as s, u64},
    combinator::{cut, map, value, verify},
    error::context,
    multi::{fold_many0, many0, separated_list1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    Finish, IResult, Offset,
};
use serde_json::Number;
use std::str::FromStr;

impl FromStr for JsonPath {
    type Err = Error;

    /// Parse a JSON Path from string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (rest, json_path) = json_path(s)
            .finish()
            .map_err(|e| Error::from_input_error(s, e))?;
        if !rest.is_empty() {
            return Err(Error {
                position: s.offset(rest),
                message: format!("unexpected trailing characters: {rest}").into(),
            });
        }
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
                    map(expr, ExprOrPredicate::Expr),
                    map(predicate, ExprOrPredicate::Pred),
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
    alt((
        delimited_predicate,
        map(
            tuple((expr, delimited(s, cmp_op, s), expr)),
            |(left, op, right)| Predicate::Compare {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
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
        map(
            separated_pair(predicate, delimited(s, tag("&&"), s), predicate),
            |(left, right)| Predicate::And(Box::new(left), Box::new(right)),
        ),
        map(
            separated_pair(predicate, delimited(s, tag("||"), s), predicate),
            |(left, right)| Predicate::Or(Box::new(left), Box::new(right)),
        ),
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
    alt((
        accessor_expr,
        delimited(pair(char('('), s), expr, pair(s, char(')'))),
        map(preceded(pair(char('+'), s), expr), |expr| {
            Expr::unary(UnaryOp::Plus, expr)
        }),
        map(preceded(pair(char('-'), s), expr), |expr| {
            Expr::unary(UnaryOp::Minus, expr)
        }),
        map(
            tuple((expr, delimited(s, arith_op, s), expr)),
            |(left, op, right)| Expr::binary(op, left, right),
        ),
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
    ))(input)
}

fn accessor_op(input: &str) -> IResult<&str, AccessorOp> {
    alt((
        value(AccessorOp::MemberWildcard, tag(".*")),
        value(AccessorOp::ElementWildcard, element_wildcard),
        map(member_accessor, AccessorOp::Member),
        map(array_accessor, AccessorOp::Element),
        map(filter_expr, |expr| AccessorOp::FilterExpr(Box::new(expr))),
        map(item_method, AccessorOp::Method),
    ))(input)
}

fn element_wildcard(input: &str) -> IResult<&str, ()> {
    value((), tuple((char('['), s, char('*'), s, char(']'))))(input)
}

fn member_accessor(input: &str) -> IResult<&str, String> {
    preceded(pair(char('.'), s), alt((string, raw_string)))(input)
}

fn index(input: &str) -> IResult<&str, Index> {
    alt((
        map(i32, Index::Index),
        map(
            preceded(tuple((tag_no_case("last"), s, char('-'), s)), i32),
            |v| Index::LastIndex(v.saturating_neg()),
        ),
        map(
            preceded(tuple((tag_no_case("last"), s, char('+'), s)), i32),
            Index::LastIndex,
        ),
        map(tag_no_case("last"), |_| Index::LastIndex(0)),
    ))(input)
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
        map(index, ArrayIndex::Index),
        map(
            separated_pair(index, delimited(s, tag_no_case("to"), s), index),
            |(s, e)| ArrayIndex::Slice((s, e)),
        ),
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

fn arith_op(input: &str) -> IResult<&str, BinaryOp> {
    alt((
        value(BinaryOp::Add, char('+')),
        value(BinaryOp::Sub, char('-')),
        value(BinaryOp::Mul, char('*')),
        value(BinaryOp::Div, char('/')),
        value(BinaryOp::Rem, char('%')),
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
