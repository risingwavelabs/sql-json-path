//! JSON Path parser.

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, i32, i64, multispace0 as s, u64},
    combinator::{map, value},
    multi::{many0, separated_list1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    Finish, IResult, Offset,
};
use serde_json::Number;

use crate::node::*;
use std::borrow::Cow;

impl<'a> JsonPath<'a> {
    /// Parse a JSON Path from string.
    pub fn from_str(s: &'a str) -> Result<Self, Error> {
        let (rest, json_path) = json_path(s)
            .finish()
            .map_err(|e| Error::from_input_error(s, e))?;
        if !rest.is_empty() {
            return Err(Error {
                position: s.offset(rest),
                message: "unexpected trailing characters".into(),
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

fn json_path(input: &str) -> IResult<&str, JsonPath<'_>> {
    map(
        delimited(s, pair(mode, alt((expr, predicate))), s),
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

fn predicate(input: &str) -> IResult<&str, Expr<'_>> {
    alt((
        delimited_predicate,
        map(
            tuple((expr, delimited(s, cmp_op, s), expr)),
            |(left, op, right)| Expr::binary(op, left, right),
        ),
        map(
            tuple((expr, delimited(s, tag("&&"), s), expr)),
            |(left, _, right)| Expr::binary(BinaryOp::And, left, right),
        ),
        map(
            tuple((expr, delimited(s, tag("||"), s), expr)),
            |(left, _, right)| Expr::binary(BinaryOp::Or, left, right),
        ),
        map(preceded(pair(tag("not"), s), delimited_predicate), |expr| {
            Expr::unary(UnaryOp::Not, expr)
        }),
        map(
            delimited(
                pair(char('('), s),
                predicate,
                tuple((s, char(')'), s, tag("is"), s, tag("unknown"))),
            ),
            |expr| Expr::unary(UnaryOp::IsUnknown, expr),
        ),
        map(
            separated_pair(
                expr,
                tuple((s, tag("starts"), s, tag("with"), s)),
                starts_with_literal,
            ),
            |(expr, literal)| Expr::binary(BinaryOp::StartsWith, expr, Expr::Value(literal)),
        ),
    ))(input)
}

fn delimited_predicate(input: &str) -> IResult<&str, Expr<'_>> {
    alt((
        delimited(pair(char('('), s), predicate, pair(s, char(')'))),
        map(
            delimited(
                tuple((tag("exists"), s, char('('), s)),
                expr,
                pair(s, char(')')),
            ),
            |expr| Expr::unary(UnaryOp::Exists, expr),
        ),
    ))(input)
}

fn expr(input: &str) -> IResult<&str, Expr<'_>> {
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

fn accessor_expr(input: &str) -> IResult<&str, Expr<'_>> {
    map(
        pair(path_primary, many0(preceded(s, accessor_op))),
        |(primary, ops)| Expr::Accessor(primary, ops),
    )(input)
}

fn path_primary(input: &str) -> IResult<&str, PathPrimary> {
    alt((
        value(PathPrimary::Root, char('$')),
        value(PathPrimary::Current, char('@')),
    ))(input)
}

fn accessor_op(input: &str) -> IResult<&str, AccessorOp<'_>> {
    alt((
        value(AccessorOp::MemberWildcard, tag(".*")),
        value(AccessorOp::ElementWildcard, bracket_wildcard),
        map(dot_field, AccessorOp::Member),
        map(array_accessor, AccessorOp::Element),
    ))(input)
}

fn bracket_wildcard(input: &str) -> IResult<&str, ()> {
    value((), tuple((char('['), s, char('*'), s, char(']'))))(input)
}

fn dot_field(input: &str) -> IResult<&str, Cow<'_, str>> {
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

fn filter_expr(input: &str) -> IResult<&str, Expr<'_>> {
    delimited(
        tuple((char('?'), s, char('('), s)),
        predicate,
        tuple((s, char(')'))),
    )(input)
}

fn cmp_op(input: &str) -> IResult<&str, BinaryOp> {
    alt((
        value(BinaryOp::Eq, tag("==")),
        value(BinaryOp::NotEq, tag("!=")),
        value(BinaryOp::NotEq, tag("<>")),
        value(BinaryOp::Le, tag("<=")),
        value(BinaryOp::Lt, char('<')),
        value(BinaryOp::Ge, tag(">=")),
        value(BinaryOp::Gt, char('>')),
    ))(input)
}

fn arith_op(input: &str) -> IResult<&str, BinaryOp> {
    alt((
        value(BinaryOp::Add, char('+')),
        value(BinaryOp::Sub, char('-')),
        value(BinaryOp::Mul, char('*')),
        value(BinaryOp::Div, char('/')),
        value(BinaryOp::Mod, char('%')),
    ))(input)
}

fn scalar_value(input: &str) -> IResult<&str, Value<'_>> {
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

fn starts_with_literal(input: &str) -> IResult<&str, Value<'_>> {
    alt((map(string, Value::String), map(variable, Value::Variable)))(input)
}

fn variable(input: &str) -> IResult<&str, Cow<'_, str>> {
    preceded(char('$'), raw_string)(input)
}

fn string(input: &str) -> IResult<&str, Cow<'_, str>> {
    todo!()
}

fn raw_string(input: &str) -> IResult<&str, Cow<'_, str>> {
    todo!()
}
