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

//! [SQL/JSON Path] implementation in Rust.
//!
//! # Features
//!
//! - Compatible with [SQL/JSON Path] standard and PostgreSQL implementation.
//! - Independent from JSON implementation. It supports popular libraries like [`serde-json`],
//!   [`simd-json`] and [`jsonbb`]. [Custom] JSON types are also supported.
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
//! let nodes = path.query(&json).unwrap();
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
