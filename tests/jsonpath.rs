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
    // `**` is not supported
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
    test1("$.a/+-1", r#"($."a" / -1)"#);
    test1("1 * 2 + 4 % -3 != false", "(1 * 2 + 4 % -3 != false)");

    test(r#""\b\f\r\n\t\v\"\''\\""#);
    test(r#""\x50\u0067\u{53}\u{051}\u{00004C}""#);
    test(r#"$.foo\x50\u0067\u{53}\u{051}\u{00004C}\t\"bar"#);
    test(r#""\z""#); // unrecognized escape is just the literal char

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
    test(r#"last"#);
    test(r#""last""#);
    test(r#"$.last"#);
    test(r#"$ ? (last > 0)"#);
    test(r#"$[last]"#);
    test(r#"$[$[0] ? (last > 0)]"#);

    test(r#"null.type()"#);
    test(r#"1.type()"#);
    test(r#"(1).type()"#);
    test(r#"1.2.type()"#);
    test(r#""aaa".type()"#);
    test(r#"true.type()"#);
    test(r#"$.double().floor().ceiling().abs()"#);
    test(r#"$.keyvalue().key"#);
    test(r#"$.datetime()"#);
    test(r#"$.datetime("datetime template")"#);

    test(r#"$ ? (@ starts with "abc")"#);
    test(r#"$ ? (@ starts with $var)"#);

    test(r#"$ ? (@ like_regex "(invalid pattern")"#);
    test(r#"$ ? (@ like_regex "pattern")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "i")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "is")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "isim")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "xsms")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "q")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "iq")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "smixq")"#);
    test(r#"$ ? (@ like_regex "pattern" flag "a")"#);

    test(r#"$ < 1"#);
    test(r#"($ < 1) || $.a.b <= $x"#);
    test(r#"@ + 1"#);

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
    test(r#"$ ? (@.a < 1e1)"#);
    test(r#"$ ? (@.a < -1e1)"#);
    test(r#"$ ? (@.a < +1e1)"#);
    test(r#"$ ? (@.a < .1e1)"#);
    test(r#"$ ? (@.a < -.1e1)"#);
    test(r#"$ ? (@.a < +.1e1)"#);
    test(r#"$ ? (@.a < 0.1e1)"#);
    test(r#"$ ? (@.a < -0.1e1)"#);
    test(r#"$ ? (@.a < +0.1e1)"#);
    test(r#"$ ? (@.a < 10.1e1)"#);
    test(r#"$ ? (@.a < -10.1e1)"#);
    test(r#"$ ? (@.a < +10.1e1)"#);
    test(r#"$ ? (@.a < 1e-1)"#);
    test(r#"$ ? (@.a < -1e-1)"#);
    test(r#"$ ? (@.a < +1e-1)"#);
    test(r#"$ ? (@.a < .1e-1)"#);
    test(r#"$ ? (@.a < -.1e-1)"#);
    test(r#"$ ? (@.a < +.1e-1)"#);
    test(r#"$ ? (@.a < 0.1e-1)"#);
    test(r#"$ ? (@.a < -0.1e-1)"#);
    test(r#"$ ? (@.a < +0.1e-1)"#);
    test(r#"$ ? (@.a < 10.1e-1)"#);
    test(r#"$ ? (@.a < -10.1e-1)"#);
    test(r#"$ ? (@.a < +10.1e-1)"#);
    test(r#"$ ? (@.a < 1e+1)"#);
    test(r#"$ ? (@.a < -1e+1)"#);
    test(r#"$ ? (@.a < +1e+1)"#);
    test(r#"$ ? (@.a < .1e+1)"#);
    test(r#"$ ? (@.a < -.1e+1)"#);
    test(r#"$ ? (@.a < +.1e+1)"#);
    test(r#"$ ? (@.a < 0.1e+1)"#);
    test(r#"$ ? (@.a < -0.1e+1)"#);
    test(r#"$ ? (@.a < +0.1e+1)"#);
    test(r#"$ ? (@.a < 10.1e+1)"#);
    test(r#"$ ? (@.a < -10.1e+1)"#);
    test(r#"$ ? (@.a < +10.1e+1)"#);

    // numeric literals

    test(r#"0"#);
    test(r#"00"#);
    test(r#"0755"#);
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
    test(r#"1."#);
    test(r#"1.e1"#);
    test(r#"1a"#);
    test(r#"1e"#);
    test(r#"1.e"#);
    test(r#"1.2a"#);
    test(r#"1.2e"#);
    test(r#"1.2.e"#);
    test(r#"(1.2).e"#);
    test(r#"1e3"#);
    test(r#"1.e3"#);
    test(r#"1.e3.e"#);
    test(r#"1.e3.e4"#);
    test(r#"1.2e3"#);
    test(r#"1.2e3a"#);
    test(r#"1.2.e3"#);
    test(r#"(1.2).e3"#);
    test(r#"1..e"#);
    test(r#"1..e3"#);
    test(r#"(1.).e"#);
    test(r#"(1.).e3"#);
    test(r#"1?(2>3)"#);

    // nondecimal
    test(r#"0b100101"#);
    test(r#"0o273"#);
    test(r#"0x42F"#);

    // error cases
    test(r#"0b"#);
    test(r#"1b"#);
    test(r#"0b0x"#);

    test(r#"0o"#);
    test(r#"1o"#);
    test(r#"0o0x"#);

    test(r#"0x"#);
    test(r#"1x"#);
    test(r#"0x0y"#);

    // underscores
    test(r#"1_000_000"#);
    test(r#"1_2_3"#);
    test(r#"0x1EEE_FFFF"#);
    test(r#"0o2_73"#);
    test(r#"0b10_0101"#);

    test(r#"1_000.000_005"#);
    test(r#"1_000."#);
    test(r#".000_005"#);
    test(r#"1_000.5e0_1"#);

    // error cases
    test(r#"_100"#);
    test(r#"100_"#);
    test(r#"100__000"#);

    test(r#"_1_000.5"#);
    test(r#"1_000_.5"#);
    test(r#"1_000._5"#);
    test(r#"1_000.5_"#);
    test(r#"1_000.5e_1"#);

    // underscore after prefix not allowed in JavaScript (but allowed in SQL)
    test(r#"0b_10_0101"#);
    test(r#"0o_273"#);
    test(r#"0x_42F"#);
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

fn jsonb_path_exists(json: &str, path: &str) -> Result<bool, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = path.query::<serde_json::Value>(&json)?;
    Ok(!list.is_empty())
}
