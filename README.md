# sql-json-path

[![Build status](https://github.com/risingwavelabs/sql-json-path/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/risingwavelabs/sql-json-path/actions/workflows/ci.yml)
[![Crate](https://img.shields.io/crates/v/sql-json-path.svg)](https://crates.io/crates/sql-json-path)
[![Docs](https://docs.rs/sql-json-path/badge.svg)](https://docs.rs/sql-json-path)

[SQL/JSON Path] implementation in Rust.

🚧 Under development 🚧

## Features

- Compatible with [SQL/JSON Path] standard and PostgreSQL implementation.
- Independent from JSON implementation. It supports popular libraries like [`serde_json`], [`simd-json`] and [`jsonbb`]. Custom JSON types are also supported.

[SQL/JSON Path]: https://github.com/obartunov/sqljsondoc/blob/master/jsonpath.md
[`serde_json`]: https://crates.io/crates/serde_json
[`simd-json`]: https://crates.io/crates/simd-json
[`jsonbb`]: https://crates.io/crates/jsonbb

## Usage

```rust
use serde_json::{json, Value};
use sql_json_path::JsonPath;

let json = json!({"a": 1});
let path = JsonPath::new("$.a").unwrap();

let nodes = path.query(&json).unwrap();
assert_eq!(nodes.len(), 1);
assert_eq!(nodes[0].to_string(), "1");
```

## JSON Path Syntax

See [PostgreSQL documentation](https://www.postgresql.org/docs/16/functions-json.html#FUNCTIONS-SQLJSON-PATH) for more details.

- [x] `strict` and `lax` mode
- [x] `$`: Root object
- [x] `@`: Current object
- [x] `[index]`: An array element by index
    - [x] `[start to end]`: An array slice
    - [x] `[index1, index2, ...]`: Multiple array elements
    - [x] `[last]`: The last array element
- [x] `.name`: An object member by name
- [x] `[*]`: Any array element
- [x] `.*`: Any object member
- [x] `.**`: Any descendant object member (Postgres extension)
- [x] `?(predicate)`: Filter expression
    - [x] `==`, `!=` / `<>`, `<`, `<=`, `>`, `>=`: Comparison
    - [x] `&&`, `||`, `!`: Logical operators
    - [x] `is unknown`: Check if the value is unknown
    - [x] `like_regex`: Check if the string matches the regular expression
    - [x] `starts with`: Check if the string starts with the given prefix
    - [x] `exists(expr)`: Check if the expression matches any value
- [x] Operations
    - [x] `+`: Addition / Unary plus
    - [x] `-`: Subtraction / Negation
    - [x] `*`: Multiplication
    - [x] `/`: Division
    - [x] `%`: Modulo
- [x] Methods
    - [x] `.type()`
    - [x] `.size()`
    - [x] `.double()`
    - [x] `.ceiling()`
    - [x] `.floor()`
    - [x] `.abs()`
    - [ ] `.datetime()`
    - [ ] `.datetime(template)`
    - [x] `.keyvalue()`

## License

Licensed under [Apache License, Version 2.0](LICENSE).
