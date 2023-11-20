//! [SQL/JSON Path] implementation in Rust.
//!
//! # Features
//!
//! - Compatible with [SQL/JSON Path] standard and PostgreSQL implementation.
//! - Independent from JSON implementation. It supports popular libraries like [`serde-json`]. [Custom] JSON types are also supported.
//!
//! # Usage
//!
//! ```rust
//! use serde_json::{json, Value};
//! use sql_json_path::JsonPath;
//!
//! let json = json!({"a": 1});
//! let path = JsonPath::new("$.a").unwrap();
//!
//! let nodes = path.query::<Value>(&json).unwrap();
//! assert_eq!(nodes.len(), 1);
//! assert_eq!(nodes[0].to_string(), "1");
//! ```
//!
//! [SQL/JSON Path]: https://github.com/obartunov/sqljsondoc/blob/master/jsonpath.md
//! [`serde-json`]: https://crates.io/crates/serde_json
//! [`simd-json`]: https://crates.io/crates/simd-json
//! [`jsonbb`]: https://crates.io/crates/jsonbb
//! [Custom]: crate::json

mod ast;
mod eval;
pub mod json;
mod parser;

pub use ast::JsonPath;
pub use eval::Error as EvalError;
pub use parser::Error as ParseError;
