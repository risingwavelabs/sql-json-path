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
    fn test(s: &str, _display: &str) {
        JsonPath::from_str(s).unwrap();
    }
    #[track_caller]
    fn test_match(s: &str, display: &str) {
        let path = JsonPath::from_str(s).unwrap();
        assert_eq!(path.to_string(), display);
    }
    #[track_caller]
    fn test_err(s: &str, _msg: &str) {
        JsonPath::from_str(s).unwrap_err();
    }
    #[track_caller]
    fn test_err_match(s: &str, msg: &str) {
        let err = JsonPath::from_str(s).unwrap_err().to_string();
        if !err.contains(msg) {
            panic!("expected error message {:?}, got {:?}", msg, err);
        }
    }

    test_err(r#""#, r#""#);
    test_match(r#"$"#, r#"$"#);
    test_match(r#"strict $"#, r#"strict $"#);
    test_match(r#"lax $"#, r#"$"#);
    test_match(r#"$.a"#, r#"$."a""#);
    test_match(r#"$.a.v"#, r#"$."a"."v""#);
    test_match(r#"$.a.*"#, r#"$."a".*"#);
    test_match(r#"$.*[*]"#, r#"$.*[*]"#);
    test_match(r#"$.a[*]"#, r#"$."a"[*]"#);
    test_match(r#"$.a[*][*]"#, r#"$."a"[*][*]"#);
    test_match(r#"$[*]"#, r#"$[*]"#);
    test_match(r#"$[0]"#, r#"$[0]"#);
    test_match(r#"$[*][0]"#, r#"$[*][0]"#);
    test_match(r#"$[*].a"#, r#"$[*]."a""#);
    test_match(r#"$[*][0].a.b"#, r#"$[*][0]."a"."b""#);
    // test_match(r#"$.a.**.b"#, r#"$."a".**."b""#);
    // test_match(r#"$.a.**{2}.b"#, r#"$."a".**{2}."b""#);
    // test_match(r#"$.a.**{2 to 2}.b"#, r#"$."a".**{2}."b""#);
    // test_match(r#"$.a.**{2 to 5}.b"#, r#"$."a".**{2 to 5}."b""#);
    // test_match(r#"$.a.**{0 to 5}.b"#, r#"$."a".**{0 to 5}."b""#);
    // test_match(r#"$.a.**{5 to last}.b"#, r#"$."a".**{5 to last}."b""#);
    // test_match(r#"$.a.**{last}.b"#, r#"$."a".**{last}."b""#);
    // test_match(r#"$.a.**{last to 5}.b"#, r#"$."a".**{last to 5}."b""#);
    test_match(r#"$+1"#, r#"($ + 1)"#);
    test_match(r#"$-1"#, r#"($ - 1)"#);
    test_match(r#"$--+1"#, r#"($ - -1)"#);
    test(r#"$.a/+-1"#, r#"($."a" / -1)"#);
    test_match(r#"1 * 2 + 4 % -3 != false"#, r#"(1 * 2 + 4 % -3 != false)"#);
    // test_match(r#""\b\f\r\n\t\v\"\''\\""#, r#""\b\f\r\n\t\u000b\"'\\""#);
    // test_match(r#""\x50\u0067\u{53}\u{051}\u{00004C}""#, r#""PgSQL""#);
    // test_match(
    //     r#"$.foo\x50\u0067\u{53}\u{051}\u{00004C}\t\"bar"#,
    //     r#"$."fooPgSQL\t\"bar""#,
    // );
    // test_match(r#""\z""#, r#""z""#);
    test_match(r#"$.g ? ($.a == 1)"#, r#"$."g"?($."a" == 1)"#);
    test_match(r#"$.g ? (@ == 1)"#, r#"$."g"?(@ == 1)"#);
    test_match(r#"$.g ? (@.a == 1)"#, r#"$."g"?(@."a" == 1)"#);
    test_match(
        r#"$.g ? (@.a == 1 || @.a == 4)"#,
        r#"$."g"?(@."a" == 1 || @."a" == 4)"#,
    );
    test_match(
        r#"$.g ? (@.a == 1 && @.a == 4)"#,
        r#"$."g"?(@."a" == 1 && @."a" == 4)"#,
    );
    test_match(
        r#"$.g ? (@.a == 1 || @.a == 4 && @.b == 7)"#,
        r#"$."g"?(@."a" == 1 || @."a" == 4 && @."b" == 7)"#,
    );
    test_match(
        r#"$.g ? (@.a == 1 || !(@.a == 4) && @.b == 7)"#,
        r#"$."g"?(@."a" == 1 || !(@."a" == 4) && @."b" == 7)"#,
    );
    test_match(
        r#"$.g ? (@.a == 1 || !(@.x >= 123 || @.a == 4) && @.b == 7)"#,
        r#"$."g"?(@."a" == 1 || !(@."x" >= 123 || @."a" == 4) && @."b" == 7)"#,
    );
    test_match(
        r#"$.g ? (@.x >= @[*]?(@.a > "abc"))"#,
        r#"$."g"?(@."x" >= @[*]?(@."a" > "abc"))"#,
    );
    test_match(
        r#"$.g ? ((@.x >= 123 || @.a == 4) is unknown)"#,
        r#"$."g"?((@."x" >= 123 || @."a" == 4) is unknown)"#,
    );
    test_match(r#"$.g ? (exists (@.x))"#, r#"$."g"?(exists (@."x"))"#);
    test_match(
        r#"$.g ? (exists (@.x ? (@ == 14)))"#,
        r#"$."g"?(exists (@."x"?(@ == 14)))"#,
    );
    test_match(
        r#"$.g ? ((@.x >= 123 || @.a == 4) && exists (@.x ? (@ == 14)))"#,
        r#"$."g"?((@."x" >= 123 || @."a" == 4) && exists (@."x"?(@ == 14)))"#,
    );
    test_match(
        r#"$.g ? (+@.x >= +-(+@.a + 2))"#,
        r#"$."g"?(+@."x" >= +(-(+@."a" + 2)))"#,
    );
    test_match(r#"$a"#, r#"$"a""#);
    test_match(r#"$a.b"#, r#"$"a"."b""#);
    test_match(r#"$a[*]"#, r#"$"a"[*]"#);
    test_match(r#"$.g ? (@.zip == $zip)"#, r#"$."g"?(@."zip" == $"zip")"#);
    test_match(r#"$.a[1,2, 3 to 16]"#, r#"$."a"[1,2,3 to 16]"#);
    test_match(
        r#"$.a[$a + 1, ($b[*]) to -($[0] * 2)]"#,
        r#"$."a"[$"a" + 1,$"b"[*] to -($[0] * 2)]"#,
    );
    test_match(r#"$.a[$.a.size() - 3]"#, r#"$."a"[$."a".size() - 3]"#);
    test_err_match(r#"last"#, r#"LAST is allowed only in array subscripts"#);
    test_match(r#""last""#, r#""last""#);
    test_match(r#"$.last"#, r#"$."last""#);
    test_err_match(
        r#"$ ? (last > 0)"#,
        r#"LAST is allowed only in array subscripts"#,
    );
    test_match(r#"$[last]"#, r#"$[last]"#);
    test_match(r#"$[$[0] ? (last > 0)]"#, r#"$[$[0]?(last > 0)]"#);
    test_match(r#"null.type()"#, r#"null.type()"#);
    // test_err(
    //     r#"1.type()"#,
    //     r#"trailing junk after numeric literal at or near "1.t" of jsonpath input"#,
    // );
    test_match(r#"(1).type()"#, r#"(1).type()"#);
    test_match(r#"1.2.type()"#, r#"(1.2).type()"#);
    test_match(r#""aaa".type()"#, r#""aaa".type()"#);
    test_match(r#"true.type()"#, r#"true.type()"#);
    test_match(
        r#"$.double().floor().ceiling().abs()"#,
        r#"$.double().floor().ceiling().abs()"#,
    );
    test_match(r#"$.keyvalue().key"#, r#"$.keyvalue()."key""#);
    // test_match(r#"$.datetime()"#, r#"$.datetime()"#);
    // test_match(
    //     r#"$.datetime("datetime template")"#,
    //     r#"$.datetime("datetime template")"#,
    // );
    test_match(r#"$ ? (@ starts with "abc")"#, r#"$?(@ starts with "abc")"#);
    test_match(r#"$ ? (@ starts with $var)"#, r#"$?(@ starts with $"var")"#);
    test_err(
        r#"$ ? (@ like_regex "(invalid pattern")"#,
        r#"invalid regular expression: parentheses () not balanced"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern")"#,
        r#"$?(@ like_regex "pattern")"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern" flag "")"#,
        r#"$?(@ like_regex "pattern")"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern" flag "i")"#,
        r#"$?(@ like_regex "pattern" flag "i")"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern" flag "is")"#,
        r#"$?(@ like_regex "pattern" flag "is")"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern" flag "isim")"#,
        r#"$?(@ like_regex "pattern" flag "ism")"#,
    );
    test_err(
        r#"$ ? (@ like_regex "pattern" flag "xsms")"#,
        r#"XQuery "x" flag (expanded regular expressions) is not implemented"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern" flag "q")"#,
        r#"$?(@ like_regex "pattern" flag "q")"#,
    );
    test_match(
        r#"$ ? (@ like_regex "pattern" flag "iq")"#,
        r#"$?(@ like_regex "pattern" flag "iq")"#,
    );
    // test_match(
    //     r#"$ ? (@ like_regex "pattern" flag "smixq")"#,
    //     r#"$?(@ like_regex "pattern" flag "ismxq")"#,
    // );
    test_err(
        r#"$ ? (@ like_regex "pattern" flag "a")"#,
        r#"Unrecognized flag character "a" in LIKE_REGEX predicate."#,
    );
    test_match(r#"$ < 1"#, r#"($ < 1)"#);
    test_match(
        r#"($ < 1) || $.a.b <= $x"#,
        r#"($ < 1 || $."a"."b" <= $"x")"#,
    );
    test_err(r#"@ + 1"#, r#"@ is not allowed in root expressions"#);
    test_match(r#"($).a.b"#, r#"$."a"."b""#);
    test_match(r#"($.a.b).c.d"#, r#"$."a"."b"."c"."d""#);
    test_match(
        r#"($.a.b + -$.x.y).c.d"#,
        r#"($."a"."b" + -$."x"."y")."c"."d""#,
    );
    test_match(r#"(-+$.a.b).c.d"#, r#"(-(+$."a"."b"))."c"."d""#);
    test_match(r#"1 + ($.a.b + 2).c.d"#, r#"(1 + ($."a"."b" + 2)."c"."d")"#);
    test_match(r#"1 + ($.a.b > 2).c.d"#, r#"(1 + ($."a"."b" > 2)."c"."d")"#);
    test_match(r#"($)"#, r#"$"#);
    test_match(r#"(($))"#, r#"$"#);
    test_match(
        r#"((($ + 1)).a + ((2)).b ? ((((@ > 1)) || (exists(@.c)))))"#,
        r#"(($ + 1)."a" + (2)."b"?(@ > 1 || exists (@."c")))"#,
    );
    test_match(r#"$ ? (@.a < 1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < -1)"#, r#"$?(@."a" < -1)"#);
    test_match(r#"$ ? (@.a < +1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < .1)"#, r#"$?(@."a" < 0.1)"#);
    test_match(r#"$ ? (@.a < -.1)"#, r#"$?(@."a" < -0.1)"#);
    test_match(r#"$ ? (@.a < +.1)"#, r#"$?(@."a" < 0.1)"#);
    test_match(r#"$ ? (@.a < 0.1)"#, r#"$?(@."a" < 0.1)"#);
    test_match(r#"$ ? (@.a < -0.1)"#, r#"$?(@."a" < -0.1)"#);
    test_match(r#"$ ? (@.a < +0.1)"#, r#"$?(@."a" < 0.1)"#);
    test_match(r#"$ ? (@.a < 10.1)"#, r#"$?(@."a" < 10.1)"#);
    test_match(r#"$ ? (@.a < -10.1)"#, r#"$?(@."a" < -10.1)"#);
    test_match(r#"$ ? (@.a < +10.1)"#, r#"$?(@."a" < 10.1)"#);
    test_match(r#"$ ? (@.a < 1e1)"#, r#"$?(@."a" < 10)"#);
    test_match(r#"$ ? (@.a < -1e1)"#, r#"$?(@."a" < -10)"#);
    test_match(r#"$ ? (@.a < +1e1)"#, r#"$?(@."a" < 10)"#);
    test_match(r#"$ ? (@.a < .1e1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < -.1e1)"#, r#"$?(@."a" < -1)"#);
    test_match(r#"$ ? (@.a < +.1e1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < 0.1e1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < -0.1e1)"#, r#"$?(@."a" < -1)"#);
    test_match(r#"$ ? (@.a < +0.1e1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < 10.1e1)"#, r#"$?(@."a" < 101)"#);
    test_match(r#"$ ? (@.a < -10.1e1)"#, r#"$?(@."a" < -101)"#);
    test_match(r#"$ ? (@.a < +10.1e1)"#, r#"$?(@."a" < 101)"#);
    test_match(r#"$ ? (@.a < 1e-1)"#, r#"$?(@."a" < 0.1)"#);
    test_match(r#"$ ? (@.a < -1e-1)"#, r#"$?(@."a" < -0.1)"#);
    test_match(r#"$ ? (@.a < +1e-1)"#, r#"$?(@."a" < 0.1)"#);
    test_match(r#"$ ? (@.a < .1e-1)"#, r#"$?(@."a" < 0.01)"#);
    test_match(r#"$ ? (@.a < -.1e-1)"#, r#"$?(@."a" < -0.01)"#);
    test_match(r#"$ ? (@.a < +.1e-1)"#, r#"$?(@."a" < 0.01)"#);
    test_match(r#"$ ? (@.a < 0.1e-1)"#, r#"$?(@."a" < 0.01)"#);
    test_match(r#"$ ? (@.a < -0.1e-1)"#, r#"$?(@."a" < -0.01)"#);
    test_match(r#"$ ? (@.a < +0.1e-1)"#, r#"$?(@."a" < 0.01)"#);
    test_match(r#"$ ? (@.a < 10.1e-1)"#, r#"$?(@."a" < 1.01)"#);
    test_match(r#"$ ? (@.a < -10.1e-1)"#, r#"$?(@."a" < -1.01)"#);
    test_match(r#"$ ? (@.a < +10.1e-1)"#, r#"$?(@."a" < 1.01)"#);
    test_match(r#"$ ? (@.a < 1e+1)"#, r#"$?(@."a" < 10)"#);
    test_match(r#"$ ? (@.a < -1e+1)"#, r#"$?(@."a" < -10)"#);
    test_match(r#"$ ? (@.a < +1e+1)"#, r#"$?(@."a" < 10)"#);
    test_match(r#"$ ? (@.a < .1e+1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < -.1e+1)"#, r#"$?(@."a" < -1)"#);
    test_match(r#"$ ? (@.a < +.1e+1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < 0.1e+1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < -0.1e+1)"#, r#"$?(@."a" < -1)"#);
    test_match(r#"$ ? (@.a < +0.1e+1)"#, r#"$?(@."a" < 1)"#);
    test_match(r#"$ ? (@.a < 10.1e+1)"#, r#"$?(@."a" < 101)"#);
    test_match(r#"$ ? (@.a < -10.1e+1)"#, r#"$?(@."a" < -101)"#);
    test_match(r#"$ ? (@.a < +10.1e+1)"#, r#"$?(@."a" < 101)"#);

    // numeric literals
    test_match(r#"0"#, r#"0"#);
    // test_err(
    //     r#"00"#,
    //     r#"trailing junk after numeric literal at or near "00" of jsonpath input"#,
    // );
    // test_err(r#"0755"#, r#"syntax error at end of jsonpath input"#);
    test(r#"0.0"#, r#"0.0"#);
    test(r#"0.000"#, r#"0.000"#);
    test(r#"0.000e1"#, r#"0.00"#);
    test(r#"0.000e2"#, r#"0.0"#);
    test(r#"0.000e3"#, r#"0"#);
    test(r#"0.0010"#, r#"0.0010"#);
    test(r#"0.0010e-1"#, r#"0.00010"#);
    test(r#"0.0010e+1"#, r#"0.010"#);
    test(r#"0.0010e+2"#, r#"0.10"#);
    test_match(r#".001"#, r#"0.001"#);
    test_match(r#".001e1"#, r#"0.01"#);
    test_match(r#"1."#, r#"1"#);
    test_match(r#"1.e1"#, r#"10"#);
    test_err(
        r#"1a"#,
        r#"trailing junk after numeric literal at or near "1a" of jsonpath input"#,
    );
    test_err(
        r#"1e"#,
        r#"trailing junk after numeric literal at or near "1e" of jsonpath input"#,
    );
    test_err(
        r#"1.e"#,
        r#"trailing junk after numeric literal at or near "1.e" of jsonpath input"#,
    );
    test_err(
        r#"1.2a"#,
        r#"trailing junk after numeric literal at or near "1.2a" of jsonpath input"#,
    );
    test_err(
        r#"1.2e"#,
        r#"trailing junk after numeric literal at or near "1.2e" of jsonpath input"#,
    );
    test_match(r#"1.2.e"#, r#"(1.2)."e""#);
    test_match(r#"(1.2).e"#, r#"(1.2)."e""#);
    test_match(r#"1e3"#, r#"1000"#);
    test_match(r#"1.e3"#, r#"1000"#);
    test_match(r#"1.e3.e"#, r#"(1000)."e""#);
    test_match(r#"1.e3.e4"#, r#"(1000)."e4""#);
    test_match(r#"1.2e3"#, r#"1200"#);
    test_err(
        r#"1.2e3a"#,
        r#"trailing junk after numeric literal at or near "1.2e3a" of jsonpath input"#,
    );
    test_match(r#"1.2.e3"#, r#"(1.2)."e3""#);
    test_match(r#"(1.2).e3"#, r#"(1.2)."e3""#);
    test_match(r#"1..e"#, r#"(1)."e""#);
    test_match(r#"1..e3"#, r#"(1)."e3""#);
    test_match(r#"(1.).e"#, r#"(1)."e""#);
    test_match(r#"(1.).e3"#, r#"(1)."e3""#);
    test_match(r#"1?(2>3)"#, r#"(1)?(2 > 3)"#);

    // nondecimal
    // test_match(r#"0b100101"#, r#"37"#);
    // test_match(r#"0o273"#, r#"187"#);
    // test_match(r#"0x42F"#, r#"1071"#);

    // error cases
    test_err(
        r#"0b"#,
        r#"trailing junk after numeric literal at or near "0b" of jsonpath input"#,
    );
    test_err(
        r#"1b"#,
        r#"trailing junk after numeric literal at or near "1b" of jsonpath input"#,
    );
    test_err(r#"0b0x"#, r#"syntax error at end of jsonpath input"#);
    test_err(
        r#"0o"#,
        r#"trailing junk after numeric literal at or near "0o" of jsonpath input"#,
    );
    test_err(
        r#"1o"#,
        r#"trailing junk after numeric literal at or near "1o" of jsonpath input"#,
    );
    test_err(r#"0o0x"#, r#"syntax error at end of jsonpath input"#);
    test_err(
        r#"0x"#,
        r#"trailing junk after numeric literal at or near "0x" of jsonpath input"#,
    );
    test_err(
        r#"1x"#,
        r#"trailing junk after numeric literal at or near "1x" of jsonpath input"#,
    );
    test_err(r#"0x0y"#, r#"syntax error at end of jsonpath input"#);

    // underscores
    // test_match(r#"1_000_000"#, r#"1000000"#);
    // test_match(r#"1_2_3"#, r#"123"#);
    // test_match(r#"0x1EEE_FFFF"#, r#"518979583"#);
    // test_match(r#"0o2_73"#, r#"187"#);
    // test_match(r#"0b10_0101"#, r#"37"#);
    // test_match(r#"1_000.000_005"#, r#"1000.000005"#);
    // test_match(r#"1_000."#, r#"1000"#);
    // test_match(r#".000_005"#, r#"0.000005"#);
    // test_match(r#"1_000.5e0_1"#, r#"10005"#);

    // error cases
    test_err(r#"_100"#, r#"syntax error at end of jsonpath input"#);
    test_err(
        r#"100_"#,
        r#"trailing junk after numeric literal at or near "100_" of jsonpath input"#,
    );
    test_err(r#"100__000"#, r#"syntax error at end of jsonpath input"#);
    test_err(r#"_1_000.5"#, r#"syntax error at end of jsonpath input"#);
    test_err(
        r#"1_000_.5"#,
        r#"trailing junk after numeric literal at or near "1_000_" of jsonpath input"#,
    );
    test_err(
        r#"1_000._5"#,
        r#"trailing junk after numeric literal at or near "1_000._" of jsonpath input"#,
    );
    test_err(
        r#"1_000.5_"#,
        r#"trailing junk after numeric literal at or near "1_000.5_" of jsonpath input"#,
    );
    test_err(
        r#"1_000.5e_1"#,
        r#"trailing junk after numeric literal at or near "1_000.5e" of jsonpath input"#,
    );

    // underscore after prefix not allowed in JavaScript (but allowed in SQL)
    test_err(r#"0b_10_0101"#, r#"syntax error at end of jsonpath input"#);
    test_err(r#"0o_273"#, r#"syntax error at end of jsonpath input"#);
    test_err(r#"0x_42F"#, r#"syntax error at end of jsonpath input"#);
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
        Err(EvalError::ArrayIndexNotNumeric)
    );
}

#[test]
fn query_array_index() {
    assert_eq!(
        jsonb_path_query("[1]", "lax $[10000000000000000]"),
        Err(EvalError::ArrayIndexOutOfRange)
    );
    assert_eq!(
        jsonb_path_query("[1]", "strict $[10000000000000000]"),
        Err(EvalError::ArrayIndexOutOfRange)
    );
    assert_eq!(jsonb_path_query("[1]", "$[0]"), Ok(vec!["1".into()]));
    assert_eq!(jsonb_path_query("[1]", "$[0.3]"), Ok(vec!["1".into()]));
    assert_eq!(jsonb_path_query("[1]", "$[0.5]"), Ok(vec!["1".into()]));
    assert_eq!(jsonb_path_query("[1]", "$[0.9]"), Ok(vec!["1".into()]));
    assert_eq!(jsonb_path_query("[1]", "$[1.2]"), Ok(vec![]));
    assert_eq!(
        jsonb_path_query("[1]", "strict $[1.2]"),
        Err(EvalError::ArrayIndexOutOfBounds)
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
fn numeric_methods() {
    assert_eq!(
        jsonb_path_query(r#"[0, 1, -2, -3.4, 5.6]"#, r#"lax $[*].abs()"#),
        Ok(vec![
            "0".into(),
            "1".into(),
            "2".into(),
            "3.4".into(),
            "5.6".into()
        ])
    );
    assert_eq!(
        jsonb_path_query(r#"[0, 1, -2, -3.4, 5.6]"#, r#"lax $[*].floor()"#),
        Ok(vec![
            "0".into(),
            "1".into(),
            "-2".into(),
            "-4".into(),
            "5".into()
        ])
    );
    assert_eq!(
        jsonb_path_query(r#"[0, 1, -2, -3.4, 5.6]"#, r#"lax $[*].ceiling()"#),
        Ok(vec![
            "0".into(),
            "1".into(),
            "-2".into(),
            "-3".into(),
            "6".into()
        ])
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
