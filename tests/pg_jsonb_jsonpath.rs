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
    // match one of:
    // jsonb 'json' @? 'path';
    // jsonb 'json' @@ 'path';
    let r1 = regex::Regex::new(r#"jsonb '(.*)' (@\?|@@) '(.*)';"#).unwrap();
    if let Some(capture) = r1.captures(sql) {
        let json = capture.get(1).unwrap().as_str();
        let op = capture.get(2).unwrap().as_str();
        let path = capture.get(3).unwrap().as_str();
        let actual = match op {
            "@?" => jsonb_path_exists(json, path, "{}", true),
            "@@" => jsonb_path_match(json, path, "{}", true),
            _ => return Err(format!("invalid operator: {}", op).into()),
        };
        return assert_match(actual, expected);
    }
    // match one of:
    // jsonb_path_*('json', 'path');
    // jsonb_path_*('json', 'path', 'vars');
    // jsonb_path_*('json', 'path', vars => 'vars');
    // jsonb_path_*('json', 'path', silent => [true|false]);
    let r2 = regex::Regex::new(
        r#"([a-z_]+)\('([^']*)',\s*'([^']*)'(?:,\s*(?:vars =>)? '([^']*)')?(?:,\s*silent => (\w+))?\);"#,
    )
    .unwrap();
    if let Some(capture) = r2.captures(sql) {
        let func = capture.get(1).unwrap().as_str();
        let json = capture.get(2).unwrap().as_str();
        let path = capture.get(3).unwrap().as_str();
        let vars = capture.get(4).map_or("{}", |s| s.as_str());
        let silent = capture.get(5).map_or(false, |s| s.as_str() == "true");
        // println!("capture: {:#?}", capture);
        let actual = match func {
            "jsonb_path_exists" => jsonb_path_exists(json, path, vars, silent),
            "jsonb_path_match" => jsonb_path_match(json, path, vars, silent),
            "jsonb_path_query" => jsonb_path_query(json, path, vars, silent),
            "jsonb_path_query_array" => {
                jsonb_path_query_array(json, path, vars, silent).map(|s| vec![s])
            }
            "jsonb_path_query_first" => {
                jsonb_path_query_first(json, path, vars, silent).map(|s| match s {
                    Some(s) => vec![s],
                    None => vec!["".into()],
                })
            }
            _ => return Err(format!("invalid function: {}", func).into()),
        };
        return assert_match(actual, expected);
    }
    Err("unrecognized query".into())
}

fn assert_match(
    actual: Result<Vec<String>, EvalError>,
    expected: Result<Vec<String>, &str>,
) -> Result<(), Failed> {
    match (actual, expected) {
        (Ok(b), Ok(expected)) if b == expected => return Ok(()),
        (Err(e), Err(msg)) if e.to_string().contains(msg) => return Ok(()),
        (actual, expected) => {
            return Err(format!("expected: {expected:?}, actual: {actual:?}").into())
        }
    }
}

fn jsonb_path_exists(
    json: &str,
    path: &str,
    vars: &str,
    silent: bool,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let exist = match path.exists_with_vars(&json, &vars) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(vec!["".into()]),
        Err(e) => return Err(e),
    };
    Ok(vec![if exist { "t" } else { "f" }.to_string()])
}

fn jsonb_path_match(
    json: &str,
    path: &str,
    vars: &str,
    silent: bool,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let result = match path.query_first_with_vars(&json, &vars) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(vec!["".into()]),
        Err(e) => return Err(e),
    };
    match result {
        Some(value) => {
            if value.as_ref().is_null() {
                Ok(vec!["".to_string()])
            } else if let Some(b) = value.as_ref().as_bool() {
                Ok(vec![if b { "t" } else { "f" }.to_string()])
            } else if silent {
                Ok(vec!["".to_string()])
            } else {
                Err(EvalError::ExpectSingleBoolean)
            }
        }
        None => Ok(vec!["".to_string()]),
    }
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
    let list = match path.query_with_vars(&json, &vars) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(vec![]),
        Err(e) => return Err(e),
    };
    Ok(list.into_iter().map(|v| v.to_string()).collect())
}

fn jsonb_path_query_array(
    json: &str,
    path: &str,
    vars: &str,
    silent: bool,
) -> Result<String, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = match path.query_with_vars(&json, &vars) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok("".into()),
        Err(e) => return Err(e),
    };
    let array = serde_json::Value::Array(list.into_iter().map(|v| v.into_owned()).collect());
    Ok(array.to_string())
}

fn jsonb_path_query_first(
    json: &str,
    path: &str,
    vars: &str,
    silent: bool,
) -> Result<Option<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let path = JsonPath::from_str(path).unwrap();
    let list = match path.query_first_with_vars(&json, &vars) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(list.map(|v| v.to_string()))
}
