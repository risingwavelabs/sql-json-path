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
use ::simd_json::{
    base::{ValueAsArray, ValueAsObject},
    derived::{TypedArrayValue, TypedObjectValue},
    prelude::{TypedScalarValue, ValueAsScalar},
    value::{
        borrowed::Object as BorrowedObject, owned::Object as OwnedObject, BorrowedValue, OwnedValue,
    },
    StaticNode,
};

impl<'b> Json for BorrowedValue<'b> {
    type Borrowed<'a>
        = &'a BorrowedValue<'b>
    where
        'b: 'a;

    fn as_ref(&self) -> Self::Borrowed<'_> {
        self
    }

    fn null() -> Self {
        BorrowedValue::from(())
    }

    fn bool(b: bool) -> Self {
        BorrowedValue::from(b)
    }

    fn from_u64(v: u64) -> Self {
        BorrowedValue::from(v)
    }

    fn from_i64(v: i64) -> Self {
        BorrowedValue::from(v)
    }

    fn from_f64(v: f64) -> Self {
        BorrowedValue::from(v)
    }

    fn from_number(n: Number) -> Self {
        if let Some(i) = n.as_i64() {
            BorrowedValue::from(i)
        } else if let Some(u) = n.as_u64() {
            BorrowedValue::from(u)
        } else {
            BorrowedValue::from(n.as_f64().unwrap())
        }
    }

    fn from_string(s: &str) -> Self {
        BorrowedValue::from(s.to_string())
    }

    fn object<'a, I: IntoIterator<Item = (&'a str, Self)>>(iter: I) -> Self {
        Self::from(
            iter.into_iter()
                .map(|(k, v)| (k.to_string().into(), v))
                .collect::<BorrowedObject>(),
        )
    }
}

impl<'a, 'b> JsonRef<'a> for &'a BorrowedValue<'b> {
    type Owned = BorrowedValue<'b>;
    type Array = &'a Vec<BorrowedValue<'b>>;
    type Object = &'a BorrowedObject<'b>;

    fn to_owned(self) -> Self::Owned {
        self.clone()
    }

    fn null() -> Self {
        &BorrowedValue::Static(StaticNode::Null)
    }

    fn as_bool(self) -> Option<bool> {
        ValueAsScalar::as_bool(self)
    }

    fn as_number(self) -> Option<Number> {
        match self {
            BorrowedValue::Static(StaticNode::I64(v)) => Some(Number::from(*v)),
            BorrowedValue::Static(StaticNode::U64(v)) => Some(Number::from(*v)),
            BorrowedValue::Static(StaticNode::F64(v)) => Some(Number::from_f64(*v).unwrap()),
            _ => None,
        }
    }

    fn as_str(self) -> Option<&'a str> {
        ValueAsScalar::as_str(self)
    }

    fn as_array(self) -> Option<Self::Array> {
        ValueAsArray::as_array(self)
    }

    fn as_object(self) -> Option<Self::Object> {
        ValueAsObject::as_object(self)
    }

    fn is_null(self) -> bool {
        TypedScalarValue::is_null(&self)
    }

    fn is_number(self) -> bool {
        TypedScalarValue::is_number(self)
    }

    fn is_string(self) -> bool {
        TypedScalarValue::is_str(self)
    }

    fn is_array(self) -> bool {
        TypedArrayValue::is_array(self)
    }

    fn is_object(self) -> bool {
        TypedObjectValue::is_object(self)
    }
}

impl<'a, 'b> ArrayRef<'a> for &'a Vec<BorrowedValue<'b>> {
    type JsonRef = &'a BorrowedValue<'b>;

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

impl<'a, 'b> ObjectRef<'a> for &'a BorrowedObject<'b> {
    type JsonRef = &'a BorrowedValue<'b>;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, key: &str) -> Option<Self::JsonRef> {
        self.get(key)
    }

    fn list(self) -> Vec<(&'a str, Self::JsonRef)> {
        self.iter().map(|(k, v)| (k.as_ref(), v)).collect()
    }

    fn list_value(self) -> Vec<Self::JsonRef> {
        self.values().collect()
    }
}

impl Json for OwnedValue {
    type Borrowed<'a> = &'a OwnedValue;

    fn as_ref(&self) -> Self::Borrowed<'_> {
        self
    }

    fn null() -> Self {
        OwnedValue::from(())
    }

    fn bool(b: bool) -> Self {
        OwnedValue::from(b)
    }

    fn from_u64(v: u64) -> Self {
        OwnedValue::from(v)
    }

    fn from_i64(v: i64) -> Self {
        OwnedValue::from(v)
    }

    fn from_f64(v: f64) -> Self {
        OwnedValue::from(v)
    }

    fn from_number(n: Number) -> Self {
        if let Some(i) = n.as_i64() {
            OwnedValue::from(i)
        } else if let Some(u) = n.as_u64() {
            OwnedValue::from(u)
        } else {
            OwnedValue::from(n.as_f64().unwrap())
        }
    }

    fn from_string(s: &str) -> Self {
        OwnedValue::from(s.to_string())
    }

    fn object<'a, I: IntoIterator<Item = (&'a str, Self)>>(iter: I) -> Self {
        Self::from(
            iter.into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect::<OwnedObject>(),
        )
    }
}

impl<'a> JsonRef<'a> for &'a OwnedValue {
    type Owned = OwnedValue;
    type Array = &'a Vec<OwnedValue>;
    type Object = &'a OwnedObject;

    fn to_owned(self) -> Self::Owned {
        self.clone()
    }

    fn null() -> Self {
        &OwnedValue::Static(StaticNode::Null)
    }

    fn as_bool(self) -> Option<bool> {
        ValueAsScalar::as_bool(self)
    }

    fn as_number(self) -> Option<Number> {
        match self {
            OwnedValue::Static(StaticNode::I64(v)) => Some(Number::from(*v)),
            OwnedValue::Static(StaticNode::U64(v)) => Some(Number::from(*v)),
            OwnedValue::Static(StaticNode::F64(v)) => Some(Number::from_f64(*v).unwrap()),
            _ => None,
        }
    }

    fn as_str(self) -> Option<&'a str> {
        ValueAsScalar::as_str(self)
    }

    fn as_array(self) -> Option<Self::Array> {
        ValueAsArray::as_array(self)
    }

    fn as_object(self) -> Option<Self::Object> {
        ValueAsObject::as_object(self)
    }

    fn is_null(self) -> bool {
        TypedScalarValue::is_null(&self)
    }

    fn is_number(self) -> bool {
        TypedScalarValue::is_number(self)
    }

    fn is_string(self) -> bool {
        TypedScalarValue::is_str(self)
    }

    fn is_array(self) -> bool {
        TypedArrayValue::is_array(self)
    }

    fn is_object(self) -> bool {
        TypedObjectValue::is_object(self)
    }
}

impl<'a> ArrayRef<'a> for &'a Vec<OwnedValue> {
    type JsonRef = &'a OwnedValue;

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

impl<'a> ObjectRef<'a> for &'a OwnedObject {
    type JsonRef = &'a OwnedValue;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, key: &str) -> Option<Self::JsonRef> {
        self.get(key)
    }

    fn list(self) -> Vec<(&'a str, Self::JsonRef)> {
        self.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    fn list_value(self) -> Vec<Self::JsonRef> {
        self.values().collect()
    }
}
