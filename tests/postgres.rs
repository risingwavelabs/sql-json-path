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

//! This file contains test cases from Postgres regress tests.
//!
//! <https://github.com/postgres/postgres/blob/master/src/test/regress/expected/jsonpath.out>
//! <https://github.com/postgres/postgres/blob/master/src/test/regress/expected/jsonb_jsonpath.out>

use std::str::FromStr;

use sql_json_path::{EvalError, JsonPath};

#[test]
fn parse() {
    #[track_caller]
    fn test(s: &str) {
        JsonPath::from_str(s).unwrap();
    }
    #[track_caller]
    fn test1(s: &str, display: &str) {
        let path = JsonPath::from_str(s).unwrap();
        assert_eq!(path.to_string(), display);
    }
    #[track_caller]
    fn test_err(s: &str) {
        JsonPath::from_str(s).unwrap_err();
    }

    test_err("");
    test1("$", "$");
    test1("strict $", "strict $");
    test1("lax $", "$");
    test1("$.a", r#"$."a""#);
    test1("$.a.v", r#"$."a"."v""#);
    test1("$.a.*", r#"$."a".*"#);
    test1("$.*[*]", r#"$.*[*]"#);
    test1("$.a[*]", r#"$."a"[*]"#);
    test1("$.a[*][*]", r#"$."a"[*][*]"#);
    test1("$[*]", "$[*]");
    test1("$[0]", "$[0]");
    test1("$[*][0]", "$[*][0]");
    test1("$[*].a", r#"$[*]."a""#);
    test1("$[*][0].a.b", r#"$[*][0]."a"."b""#);
    // FIXME: `**` is not supported
    // test("$.a.**.b");
    // test("$.a.**{2}.b");
    // test("$.a.**{2 to 2}.b");
    // test("$.a.**{2 to 5}.b");
    // test("$.a.**{0 to 5}.b");
    // test("$.a.**{5 to last}.b");
    // test("$.a.**{last}.b");
    // test("$.a.**{last to 5}.b");
    test1("$+1", "($ + 1)");
    test1("$-1", "($ - 1)");
    test1("$--+1", "($ - -1)");
    // test1("$.a/+-1", r#"($."a" / -1)"#);
    // test1("1 * 2 + 4 % -3 != false", "(1 * 2 + 4 % -3 != false)");

    // test(r#""\b\f\r\n\t\v\"\''\\""#);
    // test(r#""\x50\u0067\u{53}\u{051}\u{00004C}""#);
    // test(r#"$.foo\x50\u0067\u{53}\u{051}\u{00004C}\t\"bar"#);
    // test(r#""\z""#); // unrecognized escape is just the literal char

    test(r#"$.g ? ($.a == 1)"#);
    test(r#"$.g ? (@ == 1)"#);
    test(r#"$.g ? (@.a == 1)"#);
    test(r#"$.g ? (@.a == 1 || @.a == 4)"#);
    test(r#"$.g ? (@.a == 1 && @.a == 4)"#);
    test(r#"$.g ? (@.a == 1 || @.a == 4 && @.b == 7)"#);
    test(r#"$.g ? (@.a == 1 || !(@.a == 4) && @.b == 7)"#);
    test(r#"$.g ? (@.a == 1 || !(@.x >= 123 || @.a == 4) && @.b == 7)"#);
    test(r#"$.g ? (@.x >= @[*]?(@.a > "abc"))"#);
    test(r#"$.g ? ((@.x >= 123 || @.a == 4) is unknown)"#);
    test(r#"$.g ? (exists (@.x))"#);
    test(r#"$.g ? (exists (@.x ? (@ == 14)))"#);
    test(r#"$.g ? ((@.x >= 123 || @.a == 4) && exists (@.x ? (@ == 14)))"#);
    test(r#"$.g ? (+@.x >= +-(+@.a + 2))"#);

    test(r#"$a"#);
    test(r#"$a.b"#);
    test(r#"$a[*]"#);
    test(r#"$.g ? (@.zip == $zip)"#);
    test(r#"$.a[1,2, 3 to 16]"#);
    test(r#"$.a[$a + 1, ($b[*]) to -($[0] * 2)]"#);
    test(r#"$.a[$.a.size() - 3]"#);
    test_err(r#"last"#);
    test(r#""last""#);
    test(r#"$.last"#);
    test_err(r#"$ ? (last > 0)"#);
    test(r#"$[last]"#);
    test(r#"$[$[0] ? (last > 0)]"#);

    test(r#"null.type()"#);
    // test_err(r#"1.type()"#);
    test(r#"(1).type()"#);
    test(r#"1.2.type()"#);
    test(r#""aaa".type()"#);
    test(r#"true.type()"#);
    test(r#"$.double().floor().ceiling().abs()"#);
    test(r#"$.keyvalue().key"#);
    // test(r#"$.datetime()"#);
    // test(r#"$.datetime("datetime template")"#);

    test(r#"$ ? (@ starts with "abc")"#);
    test(r#"$ ? (@ starts with $var)"#);

    // FIXME: should return error: parentheses () not balanced
    // test_err(r#"$ ? (@ like_regex "(invalid pattern")"#);
    test(r#"$ ? (@ like_regex "pattern")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "i")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "is")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "isim")"#);
    // FIXME: should return error: XQuery "x" flag (expanded regular expressions) is not implemented
    // test_err(r#"$ ? (@ like_regex "pattern" flag "xsms")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "q")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "iq")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "smixq")"#);
    // FIXME: should return error: Unrecognized flag character "a" in LIKE_REGEX predicate.
    // test_err(r#"$ ? (@ like_regex "pattern" flag "a")"#);

    test(r#"$ < 1"#);
    test(r#"($ < 1) || $.a.b <= $x"#);
    test_err(r#"@ + 1"#);

    test(r#"($).a.b"#);
    test(r#"($.a.b).c.d"#);
    test(r#"($.a.b + -$.x.y).c.d"#);
    test(r#"(-+$.a.b).c.d"#);
    test(r#"1 + ($.a.b + 2).c.d"#);
    test(r#"1 + ($.a.b > 2).c.d"#);
    test(r#"($)"#);
    test(r#"(($))"#);
    test(r#"((($ + 1)).a + ((2)).b ? ((((@ > 1)) || (exists(@.c)))))"#);

    test(r#"$ ? (@.a < 1)"#);
    test(r#"$ ? (@.a < -1)"#);
    test(r#"$ ? (@.a < +1)"#);
    test(r#"$ ? (@.a < .1)"#);
    test(r#"$ ? (@.a < -.1)"#);
    test(r#"$ ? (@.a < +.1)"#);
    test(r#"$ ? (@.a < 0.1)"#);
    test(r#"$ ? (@.a < -0.1)"#);
    test(r#"$ ? (@.a < +0.1)"#);
    test(r#"$ ? (@.a < 10.1)"#);
    test(r#"$ ? (@.a < -10.1)"#);
    test(r#"$ ? (@.a < +10.1)"#);
    // test(r#"$ ? (@.a < 1e1)"#);
    // test(r#"$ ? (@.a < -1e1)"#);
    // test(r#"$ ? (@.a < +1e1)"#);
    // test(r#"$ ? (@.a < .1e1)"#);
    // test(r#"$ ? (@.a < -.1e1)"#);
    // test(r#"$ ? (@.a < +.1e1)"#);
    // test(r#"$ ? (@.a < 0.1e1)"#);
    // test(r#"$ ? (@.a < -0.1e1)"#);
    // test(r#"$ ? (@.a < +0.1e1)"#);
    // test(r#"$ ? (@.a < 10.1e1)"#);
    // test(r#"$ ? (@.a < -10.1e1)"#);
    // test(r#"$ ? (@.a < +10.1e1)"#);
    // test(r#"$ ? (@.a < 1e-1)"#);
    // test(r#"$ ? (@.a < -1e-1)"#);
    // test(r#"$ ? (@.a < +1e-1)"#);
    // test(r#"$ ? (@.a < .1e-1)"#);
    // test(r#"$ ? (@.a < -.1e-1)"#);
    // test(r#"$ ? (@.a < +.1e-1)"#);
    // test(r#"$ ? (@.a < 0.1e-1)"#);
    // test(r#"$ ? (@.a < -0.1e-1)"#);
    // test(r#"$ ? (@.a < +0.1e-1)"#);
    // test(r#"$ ? (@.a < 10.1e-1)"#);
    // test(r#"$ ? (@.a < -10.1e-1)"#);
    // test(r#"$ ? (@.a < +10.1e-1)"#);
    // test(r#"$ ? (@.a < 1e+1)"#);
    // test(r#"$ ? (@.a < -1e+1)"#);
    // test(r#"$ ? (@.a < +1e+1)"#);
    // test(r#"$ ? (@.a < .1e+1)"#);
    // test(r#"$ ? (@.a < -.1e+1)"#);
    // test(r#"$ ? (@.a < +.1e+1)"#);
    // test(r#"$ ? (@.a < 0.1e+1)"#);
    // test(r#"$ ? (@.a < -0.1e+1)"#);
    // test(r#"$ ? (@.a < +0.1e+1)"#);
    // test(r#"$ ? (@.a < 10.1e+1)"#);
    // test(r#"$ ? (@.a < -10.1e+1)"#);
    // test(r#"$ ? (@.a < +10.1e+1)"#);

    // numeric literals

    test(r#"0"#);
    // test_err(r#"00"#);
    // test_err(r#"0755"#);
    test(r#"0.0"#);
    test(r#"0.000"#);
    test(r#"0.000e1"#);
    test(r#"0.000e2"#);
    test(r#"0.000e3"#);
    test(r#"0.0010"#);
    test(r#"0.0010e-1"#);
    test(r#"0.0010e+1"#);
    test(r#"0.0010e+2"#);
    test(r#".001"#);
    test(r#".001e1"#);
    // test(r#"1."#);
    test(r#"1.e1"#);
    test_err(r#"1a"#);
    test_err(r#"1e"#);
    // test_err(r#"1.e"#);
    // test_err(r#"1.2a"#);
    // test_err(r#"1.2e"#);
    // test(r#"1.2.e"#);
    // test(r#"(1.2).e"#);
    // test(r#"1e3"#);
    // test(r#"1.e3"#);
    // test(r#"1.e3.e"#);
    // test(r#"1.e3.e4"#);
    // test(r#"1.2e3"#);
    // test_err(r#"1.2e3a"#);
    // test(r#"1.2.e3"#);
    // test(r#"(1.2).e3"#);
    // test(r#"1..e"#);
    // test(r#"1..e3"#);
    // test(r#"(1.).e"#);
    // test(r#"(1.).e3"#);
    test(r#"1?(2>3)"#);

    // nondecimal
    // test(r#"0b100101"#);
    // test(r#"0o273"#);
    // test(r#"0x42F"#);

    // error cases
    test_err(r#"0b"#);
    test_err(r#"1b"#);
    test_err(r#"0b0x"#);

    test_err(r#"0o"#);
    test_err(r#"1o"#);
    test_err(r#"0o0x"#);

    test_err(r#"0x"#);
    test_err(r#"1x"#);
    test_err(r#"0x0y"#);

    // underscores
    // test(r#"1_000_000"#);
    // test(r#"1_2_3"#);
    // test(r#"0x1EEE_FFFF"#);
    // test(r#"0o2_73"#);
    // test(r#"0b10_0101"#);

    // test(r#"1_000.000_005"#);
    // test(r#"1_000."#);
    // test(r#".000_005"#);
    // test(r#"1_000.5e0_1"#);

    // error cases
    test_err(r#"_100"#);
    test_err(r#"100_"#);
    test_err(r#"100__000"#);

    test_err(r#"_1_000.5"#);
    test_err(r#"1_000_.5"#);
    test_err(r#"1_000._5"#);
    test_err(r#"1_000.5_"#);
    test_err(r#"1_000.5e_1"#);

    // underscore after prefix not allowed in JavaScript (but allowed in SQL)
    test_err(r#"0b_10_0101"#);
    test_err(r#"0o_273"#);
    test_err(r#"0x_42F"#);
}

#[test]
fn query() {
    assert_eq!(jsonb_path_exists(r#"{"a": 1}"#, "$.a"), Ok(true));
    assert_eq!(jsonb_path_exists(r#"{"a": 1}"#, "$.b"), Ok(false));

    assert_eq!(
        jsonb_path_exists(r#"[{"a": 1}, {"a": 2}, 3]"#, "lax $[*].a"),
        Ok(true)
    );
    assert!(jsonb_path_exists(r#"[{"a": 1}, {"a": 2}, 3]"#, "strict $[*].a").is_err());
}

#[test]
fn query_last() {
    assert_eq!(jsonb_path_query("[]", "$[last]"), Ok(vec![]));
    assert_eq!(
        jsonb_path_query("[]", "$[last ? (exists(last))]"),
        Ok(vec![])
    );
    assert_eq!(
        jsonb_path_query("[]", "strict $[last]"),
        Err(EvalError::ArrayIndexOutOfBounds)
    );
    assert_eq!(jsonb_path_query("[1]", "$[last]"), Ok(vec!["1".into()]));
    assert_eq!(jsonb_path_query("[1,2,3]", "$[last]"), Ok(vec!["3".into()]));
    assert_eq!(
        jsonb_path_query("[1,2,3]", "$[last - 1]"),
        Ok(vec!["2".into()])
    );
    assert_eq!(
        jsonb_path_query("[1,2,3]", r#"$[last ? (@.type() == "number")]"#),
        Ok(vec!["3".into()])
    );
    assert_eq!(
        jsonb_path_query("[1,2,3]", r#"$[last ? (@.type() == "string")]"#),
        Err(EvalError::ArraySubscriptNotNumeric)
    );
}

#[test]
fn division_by_zero() {
    assert_eq!(
        jsonb_path_query(r#"[12, {"a": 13}, {"b": 14}]"#, r#"lax $[0 to 10 / 0].a"#),
        Err(EvalError::DivisionByZero)
    );
}

#[test]
fn query_regex() {
    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "abc", "abd", "aBdC", "abdacb", "babc", "adc\nabc", "ab\nadc"]"#,
            r#"lax $[*] ? (@ like_regex "^ab.*c")"#
        ),
        Ok(vec![r#""abc""#.into(), r#""abdacb""#.into()])
    );

    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "abc", "abd", "aBdC", "abdacb", "babc", "adc\nabc", "ab\nadc"]"#,
            r#"lax $[*] ? (@ like_regex "^ab.*c" flag "i")"#
        ),
        Ok(vec![
            r#""abc""#.into(),
            r#""aBdC""#.into(),
            r#""abdacb""#.into()
        ])
    );

    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "abc", "abd", "aBdC", "abdacb", "babc", "adc\nabc", "ab\nadc"]"#,
            r#"lax $[*] ? (@ like_regex "^ab.*c" flag "m")"#
        ),
        Ok(vec![
            r#""abc""#.into(),
            r#""abdacb""#.into(),
            r#""adc\nabc""#.into()
        ])
    );

    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "abc", "abd", "aBdC", "abdacb", "babc", "adc\nabc", "ab\nadc"]"#,
            r#"lax $[*] ? (@ like_regex "^ab.*c" flag "s")"#
        ),
        Ok(vec![
            r#""abc""#.into(),
            r#""abdacb""#.into(),
            r#""ab\nadc""#.into()
        ])
    );

    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "a\b", "a\\b", "^a\\b$"]"#,
            r#"lax $[*] ? (@ like_regex "a\\b" flag "q")"#
        ),
        Ok(vec![r#""a\\b""#.into(), r#""^a\\b$""#.into()])
    );

    // assert_eq!(
    //     jsonb_path_query(
    //         r#"[null, 1, "a\b", "a\\b", "^a\\b$"]"#,
    //         r#"lax $[*] ? (@ like_regex "a\\b" flag "")"#
    //     ),
    //     Ok(vec![r#""a\b""#.into()])
    // );

    // assert_eq!(
    //     jsonb_path_query(
    //         r#"[null, 1, "a\b", "a\\b", "^a\\b$"]"#,
    //         r#"lax $[*] ? (@ like_regex "^a\\b$" flag "q")"#
    //     ),
    //     Ok(vec![r#""^a\\b$""#.into()])
    // );

    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "a\b", "a\\b", "^a\\b$"]"#,
            r#"lax $[*] ? (@ like_regex "^a\\B$" flag "q")"#
        ),
        Ok(vec![])
    );

    assert_eq!(
        jsonb_path_query(
            r#"[null, 1, "a\b", "a\\b", "^a\\b$"]"#,
            r#"lax $[*] ? (@ like_regex "^a\\B$" flag "iq")"#
        ),
        Ok(vec![r#""^a\\b$""#.into()])
    );

    // assert_eq!(
    //     jsonb_path_query(
    //         r#"[null, 1, "a\b", "a\\b", "^a\\b$"]"#,
    //         r#"lax $[*] ? (@ like_regex "^a\\b$" flag "")"#
    //     ),
    //     Ok(vec![r#""a\b""#.into()])
    // );
}

fn jsonb_path_exists(json: &str, path: &str) -> Result<bool, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = path.query(&json)?;
    Ok(!list.is_empty())
}

fn jsonb_path_query(json: &str, path: &str) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = path.query(&json)?;
    Ok(list.into_iter().map(|v| v.to_string()).collect())
}
