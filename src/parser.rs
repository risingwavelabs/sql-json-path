use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, i32, i64, multispace0 as s, u64},
    combinator::{map, value},
    error::{Error, ErrorKind},
    multi::{many0, separated_list1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    IResult,
};
use serde_json::Number;

use crate::node::*;
use std::borrow::Cow;

/// Parsing the input string to JSON Path.
pub fn parse_json_path(input: &[u8]) -> Result<JsonPath<'_>, Error<&[u8]>> {
    let (rest, json_path) = json_path(input)?;
    if !rest.is_empty() {
        return Err(Error::new(rest, ErrorKind::Eof));
    }
    Ok(json_path)
}

fn json_path(input: &[u8]) -> IResult<&[u8], JsonPath<'_>> {
    map(pair(mode, alt((expr, predicate))), |(mode, expr)| {
        JsonPath { mode, expr }
    })(input)
}

fn mode(input: &[u8]) -> IResult<&[u8], Mode> {
    alt((
        value(Mode::Strict, tag("strict")),
        value(Mode::Lax, tag("lax")),
        value(Mode::Lax, tag("")),
    ))(input)
}

fn predicate(input: &[u8]) -> IResult<&[u8], Expr<'_>> {
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

fn delimited_predicate(input: &[u8]) -> IResult<&[u8], Expr<'_>> {
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

fn expr(input: &[u8]) -> IResult<&[u8], Expr<'_>> {
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

fn accessor_expr(input: &[u8]) -> IResult<&[u8], Expr<'_>> {
    map(
        pair(path_primary, many0(preceded(s, accessor_op))),
        |(primary, ops)| Expr::Accessor(primary, ops),
    )(input)
}

fn path_primary(input: &[u8]) -> IResult<&[u8], PathPrimary> {
    alt((
        value(PathPrimary::Root, char('$')),
        value(PathPrimary::Current, char('@')),
    ))(input)
}

fn accessor_op(input: &[u8]) -> IResult<&[u8], AccessorOp<'_>> {
    alt((
        value(AccessorOp::MemberWildcard, tag(".*")),
        value(AccessorOp::ElementWildcard, bracket_wildcard),
        map(dot_field, AccessorOp::Member),
        map(array_accessor, AccessorOp::Element),
    ))(input)
}

fn bracket_wildcard(input: &[u8]) -> IResult<&[u8], ()> {
    value((), tuple((char('['), s, char('*'), s, char(']'))))(input)
}

fn dot_field(input: &[u8]) -> IResult<&[u8], Cow<'_, str>> {
    preceded(pair(char('.'), s), alt((string, raw_string)))(input)
}

fn index(input: &[u8]) -> IResult<&[u8], Index> {
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

fn array_accessor(input: &[u8]) -> IResult<&[u8], Vec<ArrayIndex>> {
    delimited(
        char('['),
        separated_list1(char(','), delimited(s, index_elem, s)),
        char(']'),
    )(input)
}

fn index_elem(input: &[u8]) -> IResult<&[u8], ArrayIndex> {
    alt((
        map(index, ArrayIndex::Index),
        map(
            separated_pair(index, delimited(s, tag_no_case("to"), s), index),
            |(s, e)| ArrayIndex::Slice((s, e)),
        ),
    ))(input)
}

fn filter_expr(input: &[u8]) -> IResult<&[u8], Expr<'_>> {
    delimited(
        tuple((char('?'), s, char('('), s)),
        predicate,
        tuple((s, char(')'))),
    )(input)
}

fn cmp_op(input: &[u8]) -> IResult<&[u8], BinaryOp> {
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

fn arith_op(input: &[u8]) -> IResult<&[u8], BinaryOp> {
    alt((
        value(BinaryOp::Add, char('+')),
        value(BinaryOp::Sub, char('-')),
        value(BinaryOp::Mul, char('*')),
        value(BinaryOp::Div, char('/')),
        value(BinaryOp::Mod, char('%')),
    ))(input)
}

fn scalar_value(input: &[u8]) -> IResult<&[u8], Value<'_>> {
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

fn starts_with_literal(input: &[u8]) -> IResult<&[u8], Value<'_>> {
    alt((map(string, Value::String), map(variable, Value::Variable)))(input)
}

fn variable(input: &[u8]) -> IResult<&[u8], Cow<'_, str>> {
    preceded(char('$'), raw_string)(input)
}

fn string(input: &[u8]) -> IResult<&[u8], Cow<'_, str>> {
    todo!()
}

fn raw_string(input: &[u8]) -> IResult<&[u8], Cow<'_, str>> {
    todo!()
}
