use sql_json_path::JsonPath;

fn query(input: &str, path: &str) -> Vec<serde_json::Value> {
    let input: serde_json::Value = serde_json::from_str(input).unwrap();
    JsonPath::new(path)
        .unwrap()
        .query(&input)
        .unwrap()
        .into_iter()
        .map(|value| value.into_owned())
        .collect()
}

#[test]
fn numeric_conversion_methods() {
    assert_eq!(query("1.83", "$.bigint()"), vec![serde_json::json!(2)]);
    assert_eq!(query("1.23", "$.integer()"), vec![serde_json::json!(1)]);
    assert_eq!(
        query(r#""+12.3""#, "$.number()"),
        vec![serde_json::json!(12.3)]
    );
    assert_eq!(
        query(r#""-12.3""#, "$.decimal()"),
        vec![serde_json::json!(-12.3)]
    );
}

#[test]
fn boolean_and_string_conversion_methods() {
    assert_eq!(
        query(r#""YES""#, "$.boolean()"),
        vec![serde_json::json!(true)]
    );
    assert_eq!(query("0", "$.boolean()"), vec![serde_json::json!(false)]);
    assert_eq!(query("1234", "$.string()"), vec![serde_json::json!("1234")]);
    assert_eq!(query("true", "$.string()"), vec![serde_json::json!("true")]);
    assert_eq!(
        query(r#"[1,"yes",false]"#, "$[*].string()"),
        vec![
            serde_json::json!("1"),
            serde_json::json!("yes"),
            serde_json::json!("false"),
        ]
    );
}

#[test]
fn datetime_conversion_methods() {
    assert_eq!(
        query(r#""2023-08-15""#, "$.date()"),
        vec![serde_json::json!("2023-08-15")]
    );
    assert_eq!(
        query(r#""12:34:56.789""#, "$.time()"),
        vec![serde_json::json!("12:34:56.789")]
    );
    assert_eq!(
        query(r#""12:34:56 +5:30""#, "$.time_tz()"),
        vec![serde_json::json!("12:34:56+05:30")]
    );
    assert_eq!(
        query(r#""2023-08-15 12:34:56""#, "$.timestamp()"),
        vec![serde_json::json!("2023-08-15T12:34:56")]
    );
    assert_eq!(
        query(r#""2023-08-15 12:34:56 +5:30""#, "$.timestamp_tz()"),
        vec![serde_json::json!("2023-08-15T12:34:56+05:30")]
    );
}

#[test]
fn datetime_items_keep_their_sql_json_type() {
    assert_eq!(
        query(r#""2023-08-15""#, "$.date().type()"),
        vec![serde_json::json!("date")]
    );
    assert_eq!(
        query(r#""12:34:56 +05:30""#, "$.time_tz().type()"),
        vec![serde_json::json!("time with time zone")]
    );
    assert_eq!(
        query(
            r#""2023-08-15 12:34:56 +05:30""#,
            "$.timestamp_tz().string()"
        ),
        vec![serde_json::json!("2023-08-15T12:34:56+05:30")]
    );
}

#[test]
fn datetime_items_are_comparable() {
    assert_eq!(
        query(r#""2023-08-15""#, r#"$.date() == "2023-08-15".date()"#),
        vec![serde_json::json!(true)]
    );
    assert_eq!(
        query(
            r#""12:30:00+01""#,
            r#"$.time_tz() < "11:30:00+00".time_tz()"#
        ),
        vec![serde_json::json!(true)]
    );
}

#[test]
fn invalid_conversions_report_errors() {
    let input = serde_json::json!("1.23");
    assert!(JsonPath::new("$.integer()")
        .unwrap()
        .query(&input)
        .unwrap_err()
        .to_string()
        .contains("invalid for type integer"));

    let input = serde_json::json!("bogus");
    assert!(JsonPath::new("$.date()")
        .unwrap()
        .query(&input)
        .unwrap_err()
        .to_string()
        .contains("date format is not recognized"));

    let input = serde_json::json!("2023-08-15 12:34:56+05:30");
    assert!(JsonPath::new("$.date()")
        .unwrap()
        .query(&input)
        .unwrap_err()
        .to_string()
        .contains("date format is not recognized"));
}
