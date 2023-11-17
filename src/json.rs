use serde_json::Number;
use std::fmt::{Debug, Display};

#[derive(Debug)]
pub enum Cow<'a, T: Json> {
    Borrowed(T::Borrowed<'a>),
    Owned(T),
}

impl<'a, T: Json> Cow<'a, T> {
    pub fn as_ref<'b>(&'b self) -> T::Borrowed<'b>
    where
        'a: 'b,
    {
        match self {
            // SAFETY: 'a: 'b => T::Borrowed<'a>: T::Borrowed<'b>
            Cow::Borrowed(v) => unsafe { std::mem::transmute(*v) },
            Cow::Owned(v) => v.as_ref(),
        }
    }
}

impl<T: Json> Display for Cow<'_, T>
where
    for<'a> T::Borrowed<'a>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.as_ref(), f)
    }
}

pub trait Json: Clone + Debug + 'static {
    type Borrowed<'a>: JsonRef<'a, Owned = Self>;

    fn as_ref(&self) -> Self::Borrowed<'_>;
    fn null() -> Self;
    fn bool(b: bool) -> Self;
    fn from_u64(v: u64) -> Self;
    fn from_i64(v: i64) -> Self;
    fn from_f64(v: f64) -> Self;
    fn from_number(n: Number) -> Self;
    fn from_string(s: &str) -> Self;
}

pub trait JsonRef<'a>: Copy + Debug {
    type Owned: Json<Borrowed<'a> = Self>;
    type Array: ArrayRef<'a, Json = Self>;
    type Object: ObjectRef<'a, Json = Self>;

    fn to_owned(self) -> Self::Owned;
    fn null() -> Self;
    fn is_null(self) -> bool;
    fn as_bool(self) -> Option<bool>;
    fn as_number(self) -> Option<Number>;
    fn as_str(self) -> Option<&'a str>;
    fn as_array(self) -> Option<Self::Array>;
    fn as_object(self) -> Option<Self::Object>;

    fn is_bool(self) -> bool {
        self.as_bool().is_some()
    }
    fn is_number(self) -> bool {
        self.as_number().is_some()
    }
    fn is_str(self) -> bool {
        self.as_str().is_some()
    }
    fn is_array(self) -> bool {
        self.as_array().is_some()
    }
    fn is_object(self) -> bool {
        self.as_object().is_some()
    }
}

pub trait ArrayRef<'a>: Copy {
    type Json: JsonRef<'a>;

    fn len(self) -> usize;
    fn get(self, index: usize) -> Option<Self::Json>;
    fn list(self) -> Vec<Self::Json>;
}

pub trait ObjectRef<'a>: Copy {
    type Json: JsonRef<'a>;

    fn len(self) -> usize;
    fn get(self, key: &str) -> Option<Self::Json>;
    fn list_value(self) -> Vec<Self::Json>;
}

impl Json for serde_json::Value {
    type Borrowed<'a> = &'a serde_json::Value;

    fn as_ref(&self) -> Self::Borrowed<'_> {
        self
    }

    fn null() -> Self {
        serde_json::Value::Null
    }

    fn bool(b: bool) -> Self {
        serde_json::Value::Bool(b)
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

impl<'a> JsonRef<'a> for &'a serde_json::Value {
    type Owned = serde_json::Value;
    type Array = &'a Vec<serde_json::Value>;
    type Object = &'a serde_json::Map<String, serde_json::Value>;

    fn to_owned(self) -> Self::Owned {
        self.clone()
    }

    fn null() -> Self {
        &serde_json::Value::Null
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
}

impl<'a> ArrayRef<'a> for &'a Vec<serde_json::Value> {
    type Json = &'a serde_json::Value;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, index: usize) -> Option<Self::Json> {
        (**self).get(index)
    }

    fn list(self) -> Vec<Self::Json> {
        self.iter().collect()
    }
}

impl<'a> ObjectRef<'a> for &'a serde_json::Map<String, serde_json::Value> {
    type Json = &'a serde_json::Value;

    fn len(self) -> usize {
        self.len()
    }

    fn get(self, key: &str) -> Option<Self::Json> {
        self.get(key)
    }

    fn list_value(self) -> Vec<Self::Json> {
        self.values().collect()
    }
}
