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

use chrono::FixedOffset;
use libtest_mimic::{Arguments, Failed, Measurement, Trial};
use sql_json_path::{EvalError, JsonPath};
use std::{str::FromStr, time::Instant};

fn main() {
    let args = Arguments::from_args();

    let tests = parse_script(include_str!("jsonb_jsonpath.out"));

    // Run all tests and exit the application appropriatly.
    libtest_mimic::run(&args, tests).exit();
}

fn parse_script(script: &'static str) -> Vec<Trial> {
    let mut tests = vec![];
    let default_timezone = FixedOffset::west_opt(7 * 3600).unwrap();
    let mut timezone = default_timezone;
    let mut saved_timezone = None;
    let mut lines = script
        .lines()
        .enumerate()
        // skip comments
        .filter(|(_, line)| !line.trim_start().starts_with("-- "));
    while let Some((line_no, line)) = lines.next() {
        let line = line.trim();
        match line {
            "begin;" => {
                saved_timezone = Some(timezone);
                continue;
            }
            "rollback;" => {
                if let Some(saved) = saved_timezone.take() {
                    timezone = saved;
                }
                continue;
            }
            "set time zone '+00';" => {
                timezone = FixedOffset::east_opt(0).unwrap();
                continue;
            }
            "set time zone '+10';" => {
                timezone = FixedOffset::east_opt(10 * 3600).unwrap();
                continue;
            }
            "set time zone default;" => {
                timezone = default_timezone;
                continue;
            }
            "set local timezone = 'UTC-10';" => {
                timezone = FixedOffset::east_opt(10 * 3600).unwrap();
                continue;
            }
            _ => {}
        }
        if !line.starts_with("select") && !line.starts_with("SELECT") {
            continue;
        }
        let mut sql = line[6..].trim_start().to_string();
        while !sql.contains(';') {
            let (_, line) = lines.next().expect("eof");
            sql.push_str(line.trim());
        }
        // Queries outside the small SQL subset understood by this test adapter.
        let unsupported_sql = sql.starts_with("x, y,") || sql.contains("jsonb_build_object");
        // Known implementation differences from PostgreSQL; see README.md#testing.
        let known_difference = matches!(
            line_no + 1,
            1079 | 1085 | 1350 | 1471 | 1481 | 1677 | 1700 | 4563
        );
        let ignored = unsupported_sql || known_difference;

        let (_, mut line) = lines.next().expect("eof");
        while line.starts_with("WARNING:  ") {
            (_, line) = lines.next().expect("eof");
        }
        if let Some(msg) = line.strip_prefix("ERROR:  ") {
            tests.push(
                Trial::bench(
                    format!("jsonb_jsonpath.out:{}", line_no + 1),
                    move |test_mode| benchmark(test_mode, || test(&sql, Err(msg), timezone)),
                )
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
            Trial::bench(
                format!("jsonb_jsonpath.out:{}", line_no + 1),
                move |test_mode| benchmark(test_mode, || test(&sql, Ok(results.clone()), timezone)),
            )
            .with_ignored_flag(ignored),
        );
    }
    tests
}

fn benchmark(
    test_mode: bool,
    mut run_once: impl FnMut() -> Result<(), Failed>,
) -> Result<Option<Measurement>, Failed> {
    if test_mode {
        run_once()?;
        return Ok(None);
    }

    const SAMPLE_COUNT: usize = 10;
    const TARGET_SAMPLE_NS: u128 = 1_000_000;
    const MAX_ITERATIONS: u128 = 10_000;

    let start = Instant::now();
    run_once()?;
    let warmup_ns = start.elapsed().as_nanos().max(1);
    let iterations = (TARGET_SAMPLE_NS / warmup_ns).clamp(1, MAX_ITERATIONS) as u64;

    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        for _ in 0..iterations {
            run_once()?;
        }
        samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }

    let avg = samples.iter().sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|sample| (sample - avg).powi(2))
        .sum::<f64>()
        / samples.len() as f64;

    Ok(Some(Measurement {
        avg: avg.round() as u64,
        variance: variance.sqrt().round() as u64,
    }))
}

fn test(
    sql: &str,
    expected: Result<Vec<String>, &str>,
    timezone: FixedOffset,
) -> Result<(), Failed> {
    // match one of:
    // jsonb 'json' @? 'path';
    // jsonb 'json' @@ 'path';
    let r1 = regex::Regex::new(r#"(?:jsonb '(.*)'|'(.*)'::jsonb) (@\?|@@) '(.*)';"#).unwrap();
    if let Some(capture) = r1.captures(sql) {
        let json = capture.get(1).or_else(|| capture.get(2)).unwrap().as_str();
        let op = capture.get(3).unwrap().as_str();
        let path = capture.get(4).unwrap().as_str();
        let path = match JsonPath::from_str(path) {
            Ok(path) => path,
            Err(_) => return assert_match(Ok(vec!["".into()]), expected),
        };
        let actual = match op {
            "@?" => jsonb_path_exists(json, &path, "{}", true, timezone),
            "@@" => jsonb_path_match(json, &path, "{}", true, timezone),
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
        r#"([a-z_]+)\('([^']*)',\s*'([^']*)'(?:\s*::jsonpath)?(?:,\s*(?:vars\s*=>\s*)?(?:'([^']*)'|NULL))?(?:,\s*(?:silent\s*=>\s*)?(\w+))?\);"#,
    )
    .unwrap();
    if let Some(capture) = r2.captures(sql) {
        let func = capture.get(1).unwrap().as_str();
        let json = capture.get(2).unwrap().as_str();
        let path = capture.get(3).unwrap().as_str();
        let vars = capture.get(4).map_or("{}", |s| s.as_str());
        let silent = capture.get(5).is_some_and(|s| s.as_str() == "true");
        let path = match parse_path(path, &expected)? {
            Some(path) => path,
            None => return Ok(()),
        };
        // println!("capture: {:#?}", capture);
        let actual = match func {
            "jsonb_path_exists" => jsonb_path_exists(json, &path, vars, silent, timezone),
            "jsonb_path_match" => jsonb_path_match(json, &path, vars, silent, timezone),
            "jsonb_path_query" => jsonb_path_query(json, &path, vars, silent, timezone),
            "jsonb_path_query_tz" => jsonb_path_query_tz(json, &path, vars, silent, timezone),
            "jsonb_path_query_array" => {
                jsonb_path_query_array(json, &path, vars, silent, timezone).map(|s| vec![s])
            }
            "jsonb_path_query_first" => jsonb_path_query_first(json, &path, vars, silent, timezone)
                .map(|s| match s {
                    Some(s) => vec![s],
                    None => vec!["".into()],
                }),
            _ => return Err(format!("invalid function: {}", func).into()),
        };
        return assert_match(actual, expected);
    }
    Err("unrecognized query".into())
}

fn parse_path(
    path: &str,
    expected: &Result<Vec<String>, &str>,
) -> Result<Option<JsonPath>, Failed> {
    match JsonPath::from_str(path) {
        Ok(path) => Ok(Some(path)),
        Err(_) if matches!(expected, Err(msg) if msg.starts_with("syntax error at or near")) => {
            Ok(None)
        }
        Err(error) => Err(error.to_string().into()),
    }
}

fn assert_match(
    actual: Result<Vec<String>, EvalError>,
    expected: Result<Vec<String>, &str>,
) -> Result<(), Failed> {
    match (actual, expected) {
        (Ok(b), Ok(expected)) if b == expected => Ok(()),
        (Err(e), Err(msg)) if e.to_string().contains(msg) => Ok(()),
        (actual, expected) => Err(format!("expected: {expected:?}, actual: {actual:?}").into()),
    }
}

fn jsonb_path_exists(
    json: &str,
    path: &JsonPath,
    vars: &str,
    silent: bool,
    timezone: FixedOffset,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let exist = match path.query_first_with_vars_at_timezone(&json, &vars, timezone) {
        Ok(x) => x.is_some(),
        Err(e) if silent && e.can_silent() => return Ok(vec!["".into()]),
        Err(e) => return Err(e),
    };
    Ok(vec![if exist { "t" } else { "f" }.to_string()])
}

fn jsonb_path_match(
    json: &str,
    path: &JsonPath,
    vars: &str,
    silent: bool,
    timezone: FixedOffset,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let result = match path.query_with_vars_at_timezone(&json, &vars, timezone) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(vec!["".into()]),
        Err(e) => return Err(e),
    };
    if result.len() != 1 {
        if silent {
            return Ok(vec!["".into()]);
        } else {
            return Err(EvalError::ExpectSingleBoolean);
        }
    }
    if result[0].as_ref().is_null() {
        Ok(vec!["".to_string()])
    } else if let Some(b) = result[0].as_ref().as_bool() {
        Ok(vec![if b { "t" } else { "f" }.to_string()])
    } else if silent {
        Ok(vec!["".to_string()])
    } else {
        Err(EvalError::ExpectSingleBoolean)
    }
}

fn jsonb_path_query(
    json: &str,
    path: &JsonPath,
    vars: &str,
    silent: bool,
    timezone: FixedOffset,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let list = match path.query_with_vars_at_timezone(&json, &vars, timezone) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(vec![]),
        Err(e) => return Err(e),
    };
    Ok(list.into_iter().map(|v| v.to_string()).collect())
}

fn jsonb_path_query_tz(
    json: &str,
    path: &JsonPath,
    vars: &str,
    silent: bool,
    timezone: FixedOffset,
) -> Result<Vec<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let list = match path.query_with_vars_tz(&json, &vars, timezone) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(vec![]),
        Err(e) => return Err(e),
    };
    Ok(list.into_iter().map(|v| v.to_string()).collect())
}

fn jsonb_path_query_array(
    json: &str,
    path: &JsonPath,
    vars: &str,
    silent: bool,
    timezone: FixedOffset,
) -> Result<String, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let list = match path.query_with_vars_at_timezone(&json, &vars, timezone) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok("".into()),
        Err(e) => return Err(e),
    };
    let array = serde_json::Value::Array(list.into_iter().map(|v| v.into_owned()).collect());
    Ok(array.to_string())
}

fn jsonb_path_query_first(
    json: &str,
    path: &JsonPath,
    vars: &str,
    silent: bool,
    timezone: FixedOffset,
) -> Result<Option<String>, EvalError> {
    let json = serde_json::Value::from_str(json).unwrap();
    let vars = serde_json::Value::from_str(vars).unwrap();
    let lax_path;
    let path = if silent {
        let display = path.to_string();
        lax_path = JsonPath::from_str(display.strip_prefix("strict ").unwrap_or(&display)).unwrap();
        &lax_path
    } else {
        path
    };
    let list = match path.query_first_with_vars_at_timezone(&json, &vars, timezone) {
        Ok(x) => x,
        Err(e) if silent && e.can_silent() => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(list.map(|v| v.to_string()))
}
