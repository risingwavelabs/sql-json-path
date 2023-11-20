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

use ::jsonbb::{ArrayRef, ObjectRef, Value, ValueRef};
use ::serde_json::Number;

impl super::Json for Value {
    type Borrowed<'a> = ValueRef<'a>;

    fn as_ref(&self) -> Self::Borrowed<'_> {
        self.as_ref()
    }

    fn null() -> Self {
        Value::null()
    }

    fn bool(b: bool) -> Self {
        Value::from(b)
    }

    fn from_u64(v: u64) -> Self {
        Value::from(v)
    }

    fn from_i64(v: i64) -> Self {
        Value::from(v)
    }

    fn from_f64(v: f64) -> Self {
        Value::from(v)
    }

    fn from_number(n: Number) -> Self {
        Value::from(n)
    }

    fn from_string(s: &str) -> Self {
        Value::from(s)
    }
}

impl<'a> super::JsonRef<'a> for ValueRef<'a> {
    type Owned = Value;
    type Array = ArrayRef<'a>;
    type Object = ObjectRef<'a>;

    fn to_owned(self) -> Self::Owned {
        self.to_owned()
    }

    fn null() -> Self {
        Self::Null
    }

    fn is_null(self) -> bool {
        self.is_null()
    }

    fn as_bool(self) -> Option<bool> {
        self.as_bool()
    }

    fn as_number(self) -> Option<Number> {
        self.as_number().map(|n| n.to_number())
    }

    fn as_str(self) -> Option<&'a str> {
        self.as_str()
    }

    fn as_array(self) -> Option<ArrayRef<'a>> {
        self.as_array()
    }

    fn as_object(self) -> Option<ObjectRef<'a>> {
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

impl<'a> super::ArrayRef<'a> for ArrayRef<'a> {
    type JsonRef = ValueRef<'a>;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, index: usize) -> Option<Self::JsonRef> {
        self.get(index)
    }

    fn list(self) -> Vec<Self::JsonRef> {
        self.iter().collect()
    }
}

impl<'a> super::ObjectRef<'a> for ObjectRef<'a> {
    type JsonRef = ValueRef<'a>;

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
