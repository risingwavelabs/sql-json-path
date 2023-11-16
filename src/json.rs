use serde_json::Number;

use crate::node::Value;

pub trait Json {
    type Borrowed<'a>: JsonRef
    where
        Self: 'a;
    fn as_ref(&self) -> Self::Borrowed<'_>;
    fn from_value(v: Value) -> Self;
    fn from_u64(v: u64) -> Self;
    fn from_f64(v: f64) -> Self;
    fn from_string(s: &str) -> Self;
}

pub trait JsonRef: Clone {
    type Owned: Json;
    type Array: ArrayRef<Json = Self>;
    type Object: ObjectRef<Json = Self>;

    fn null() -> Self;
    fn bool(b: bool) -> Self;
    fn is_null(&self) -> bool;
    fn as_bool(&self) -> Option<bool>;
    fn as_number(&self) -> Option<Number>;
    fn as_str(&self) -> Option<&str>;
    fn as_array(&self) -> Option<Self::Array>;
    fn as_object(&self) -> Option<Self::Object>;

    fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }
    fn is_number(&self) -> bool {
        self.as_number().is_some()
    }
    fn is_str(&self) -> bool {
        self.as_str().is_some()
    }
    fn is_array(&self) -> bool {
        self.as_array().is_some()
    }
    fn is_object(&self) -> bool {
        self.as_object().is_some()
    }
}

pub trait ArrayRef: Clone {
    type Json: JsonRef;

    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Option<Self::Json>;
    fn list(&self) -> Vec<Self::Json>;
}
pub trait ObjectRef: Clone {
    type Json: JsonRef;

    fn len(&self) -> usize;
    fn get(&self, key: &str) -> Option<Self::Json>;
    fn list_value(&self) -> Vec<Self::Json>;
}

impl Json for serde_json::Value {
    type Borrowed<'a> = &'a serde_json::Value;

    fn as_ref(&self) -> Self::Borrowed<'_> {
        self
    }

    fn from_value(v: Value) -> Self {
        match v {
            Value::Null => Self::Null,
            Value::Boolean(b) => Self::Bool(b),
            Value::Number(n) => Self::Number(n),
            Value::String(s) => Self::String(s),
            Value::Variable(_) => todo!(),
        }
    }

    fn from_u64(v: u64) -> Self {
        Self::Number(Number::from(v))
    }

    fn from_f64(v: f64) -> Self {
        Self::Number(Number::from_f64(v).unwrap())
    }

    fn from_string(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl<'a> JsonRef for &'a serde_json::Value {
    type Owned = serde_json::Value;
    type Array = &'a Vec<serde_json::Value>;
    type Object = &'a serde_json::Map<String, serde_json::Value>;

    fn null() -> Self {
        &serde_json::Value::Null
    }

    fn bool(b: bool) -> Self {
        &serde_json::Value::Bool(b)
    }

    fn is_null(&self) -> bool {
        (*self).is_null()
    }

    fn as_bool(&self) -> Option<bool> {
        (*self).as_bool()
    }

    fn as_number(&self) -> Option<Number> {
        (*self).as_number().cloned()
    }

    fn as_str(&self) -> Option<&str> {
        (*self).as_str()
    }

    fn as_array(&self) -> Option<Self::Array> {
        (*self).as_array()
    }

    fn as_object(&self) -> Option<Self::Object> {
        (*self).as_object()
    }
}

impl<'a> ArrayRef for &'a Vec<serde_json::Value> {
    type Json = &'a serde_json::Value;

    fn len(&self) -> usize {
        (*self).len()
    }

    fn get(&self, index: usize) -> Option<Self::Json> {
        (*self).get(index)
    }

    fn list(&self) -> Vec<Self::Json> {
        (*self).iter().collect()
    }
}

impl<'a> ObjectRef for &'a serde_json::Map<String, serde_json::Value> {
    type Json = &'a serde_json::Value;

    fn len(&self) -> usize {
        (*self).len()
    }

    fn get(&self, key: &str) -> Option<Self::Json> {
        (*self).get(key)
    }

    fn list_value(&self) -> Vec<Self::Json> {
        (*self).values().collect()
    }
}
