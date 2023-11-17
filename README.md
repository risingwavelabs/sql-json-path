# sql-json-path

[SQL/JSON Path](https://github.com/obartunov/sqljsondoc/blob/master/jsonpath.md) implementation in Rust.

🚧 Under development 🚧

## Usage

```rust
use serde_json::{json, Value};
use sql_json_path::JsonPath;

let json = json!({"a": 1});
let path = JsonPath::new("$.a").unwrap();

let nodes = path.query::<Value>(&json).unwrap();
assert_eq!(nodes.len(), 1);
assert_eq!(nodes[0].to_string(), "1");
```
