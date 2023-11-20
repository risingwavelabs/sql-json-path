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

use super::*;
use ::serde_json::{Map, Value};

impl Json for Value {
    type Borrowed<'a> = &'a Value;

    fn as_ref(&self) -> Self::Borrowed<'_> {
        self
    }

    fn null() -> Self {
        Value::Null
    }

    fn bool(b: bool) -> Self {
        Value::Bool(b)
    }

    fn from_u64(v: u64) -> Self {
        Self::Number(Number::from(v))
    }

    fn from_i64(v: i64) -> Self {
        Self::Number(Number::from(v))
    }

    fn from_f64(v: f64) -> Self {
        Self::Number(Number::from_f64(v).unwrap())
    }

    fn from_number(n: Number) -> Self {
        Self::Number(n)
    }

    fn from_string(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl<'a> JsonRef<'a> for &'a Value {
    type Owned = Value;
    type Array = &'a Vec<Value>;
    type Object = &'a serde_json::Map<String, Value>;

    fn to_owned(self) -> Self::Owned {
        self.clone()
    }

    fn null() -> Self {
        &Value::Null
    }

    fn is_null(self) -> bool {
        self.is_null()
    }

    fn as_bool(self) -> Option<bool> {
        self.as_bool()
    }

    fn as_number(self) -> Option<Number> {
        self.as_number().cloned()
    }

    fn as_str(self) -> Option<&'a str> {
        self.as_str()
    }

    fn as_array(self) -> Option<Self::Array> {
        self.as_array()
    }

    fn as_object(self) -> Option<Self::Object> {
        self.as_object()
    }

    fn is_number(self) -> bool {
        self.is_number()
    }

    fn is_string(self) -> bool {
        self.is_string()
    }

    fn is_array(self) -> bool {
        self.is_array()
    }

    fn is_object(self) -> bool {
        self.is_object()
    }
}

impl<'a> ArrayRef<'a> for &'a Vec<Value> {
    type JsonRef = &'a Value;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, index: usize) -> Option<Self::JsonRef> {
        (**self).get(index)
    }

    fn list(self) -> Vec<Self::JsonRef> {
        self.iter().collect()
    }
}

impl<'a> ObjectRef<'a> for &'a Map<String, Value> {
    type JsonRef = &'a Value;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, key: &str) -> Option<Self::JsonRef> {
        self.get(key)
    }

    fn list_value(self) -> Vec<Self::JsonRef> {
        self.values().collect()
    }
}
