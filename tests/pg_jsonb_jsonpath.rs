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

//! This file is the runner of Postgres regression test.
//! <https://github.com/postgres/postgres/blob/master/src/test/regress/expected/jsonb_jsonpath.out>

use libtest_mimic::{Arguments, Failed, Trial};
use sql_json_path::{EvalError, JsonPath};
use std::str::FromStr;

fn main() {
    let args = Arguments::from_args();

    let tests = parse_script(include_str!("jsonb_jsonpath.out"));

    // Run all tests and exit the application appropriatly.
    libtest_mimic::run(&args, tests).exit();
}

fn parse_script(script: &'static str) -> Vec<Trial> {
    let mut tests = vec![];
    let mut lines = script
        .lines()
        .enumerate()
        // skip comments
        .filter(|(_, line)| !line.trim_start().starts_with("-- "));
    while let Some((line_no, line)) = lines.next() {
        let line = line.trim();
        if !line.starts_with("select") && !line.starts_with("SELECT") {
            continue;
        }
        let mut sql = line[6..].trim_start().to_string();
        while !sql.ends_with(';') {
            let (_, line) = lines.next().expect("eof");
            sql.push_str(line.trim());
        }
        // not supported
        let ignored = sql.contains(".datetime") || sql.contains("_tz") || sql.contains("**");

        let (_, line) = lines.next().expect("eof");
        if let Some(msg) = line.strip_prefix("ERROR:  ") {
            tests.push(
                Trial::test(format!("jsonb_jsonpath.out:{}", line_no + 1), move || {
                    test(&sql, Err(msg))
                })
                .with_ignored_flag(ignored),
            );
            continue;
        }
        // skip '----' line
        lines.next().expect("eof");
        let mut results = vec![];
        loop {
            let (_, line) = lines.next().expect("eof");
            if line.starts_with('(') {
                // "(1 row)"
                break;
            }
            if let Ok(json) = serde_json::Value::from_str(line) {
                results.push(json.to_string());
            } else {
                results.push(line.trim().to_string());
            }
        }
        tests.push(
            Trial::test(format!("jsonb_jsonpath.out:{}", line_no + 1), move || {
                test(&sql, Ok(results))
            })
            .with_ignored_flag(ignored),
        );
    }
    tests
}

fn test(sql: &str, expected: Result<Vec<String>, &str>) -> Result<(), Failed> {
    let r1 = regex::Regex::new(r#"jsonb '(.*)' @\? '(.*)';"#).unwrap();
    if let Some(capture) = r1.captures(sql) {
        let json = capture.get(1).unwrap().as_str();
        let path = capture.get(2).unwrap().as_str();
        match (jsonb_path_exists(json, path, "{}", true), expected) {
            (Ok(b), Ok(expected)) if b == (expected[0] == "t") => return Ok(()),
            (Err(e), Err(msg)) if e.to_string().contains(msg) => return Ok(()),
            (actual, expected) => {
                return Err(format!("expected: {expected:?}, got: {actual:?}").into())
            }
        }
    }
    let r2 = regex::Regex::new(r#"jsonb_path_query\('(.*)',\s*'(.*)'\);"#).unwrap();
    if let Some(capture) = r2.captures(sql) {
        let json = capture.get(1).unwrap().as_str();
        let path = capture.get(2).unwrap().as_str();
        match (jsonb_path_query(json, path, "{}", false), expected) {
            (Ok(b), Ok(expected)) if b == expected => return Ok(()),
            (actual, expected) => {
                return Err(format!("expected: {expected:?}, got: {actual:?}").into())
            }
        }
    }
    Err("unrecognized query".into())
}

fn jsonb_path_exists(json: &str, path: &str, vars: &str, silent: bool) -> Result<bool, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let exist = path.exists_with_vars(&json, &vars)?;
    Ok(exist)
}

fn jsonb_path_query(
    json: &str,
    path: &str,
    vars: &str,
    silent: bool,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = path.query(&json)?;
    Ok(list.into_iter().map(|v| v.to_string()).collect())
}

fn jsonb_path_query_first(json: &str, path: &str) -> Result<Option<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = path.query_first(&json)?;
    Ok(list.map(|v| v.to_string()))
}
