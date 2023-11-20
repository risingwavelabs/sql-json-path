//! Abstraction over JSON values.
//!
//! This module provides several traits to abstract over JSON values.
//!
//! - [`Json`]: A trait for owned JSON values.
//! - [`JsonRef`]: A trait for borrowed JSON values.
//! - [`ArrayRef`]: A trait for borrowed JSON arrays.
//! - [`ObjectRef`]: A trait for borrowed JSON objects.

use ::serde_json::Number;
use std::fmt::{Debug, Display};

mod serde_json;
#[cfg(feature = "simd-json")]
mod simd_json;

/// A borrowed or owned JSON value.
///
/// This type is used for the return type of [`JsonPath::query`]. It is similar to
/// [`std::borrow::Cow`], but is specialized for [`Json`] and [`JsonRef`].
///
/// [`JsonPath::query`]: crate::JsonPath::query
#[derive(Debug)]
pub enum Cow<'a, T: Json + 'a> {
    /// Borrowed data.
    Borrowed(T::Borrowed<'a>),
    /// Owned data.
    Owned(T),
}

impl<'a, T: Json> Cow<'a, T> {
    /// Returns a reference to the JSON value.
    pub fn as_ref<'b>(&'b self) -> T::Borrowed<'b>
    where
        'a: 'b,
    {
        match self {
            Cow::Borrowed(v) => T::borrow(*v),
            Cow::Owned(v) => v.as_ref(),
        }
    }

    /// Extracts the owned JSON.
    ///
    /// Clones the JSON if it is not already owned.
    pub fn into_owned(self) -> T {
        match self {
            Cow::Borrowed(v) => v.to_owned(),
            Cow::Owned(v) => v,
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

/// A trait for owned JSON values.
pub trait Json: Clone + Debug {
    /// The type of borrowed JSON values.
    type Borrowed<'a>: JsonRef<'a, Owned = Self>
    where
        Self: 'a;

    /// Returns a reference to the JSON value.
    fn as_ref(&self) -> Self::Borrowed<'_>;

    /// Returns a null value.
    fn null() -> Self;

    /// Returns a boolean value.
    fn bool(b: bool) -> Self;

    /// Returns a number value from u64.
    fn from_u64(v: u64) -> Self;

    /// Returns a number value from i64.
    fn from_i64(v: i64) -> Self;

    /// Returns a number value from f64.
    fn from_f64(v: f64) -> Self;

    /// Returns a number value.
    fn from_number(n: Number) -> Self;

    /// Returns a string value.
    fn from_string(s: &str) -> Self;

    /// Narrow down the lifetime of a borrowed value.
    fn borrow<'b, 'a: 'b>(p: Self::Borrowed<'a>) -> Self::Borrowed<'b> {
        // SAFETY: 'a: 'b => T::Borrowed<'a>: T::Borrowed<'b>
        unsafe { std::mem::transmute(p) }
    }
}

/// A trait for borrowed JSON values.
pub trait JsonRef<'a>: Copy + Debug {
    /// The type of owned JSON values.
    type Owned: Json<Borrowed<'a> = Self> + 'a;

    /// The type of borrowed JSON arrays.
    type Array: ArrayRef<'a, JsonRef = Self>;

    /// The type of borrowed JSON objects.
    type Object: ObjectRef<'a, JsonRef = Self>;

    /// Creates an owned JSON value.
    fn to_owned(self) -> Self::Owned;

    /// Returns a null value.
    fn null() -> Self;

    /// If the JSON is a boolean, returns the associated bool. Returns `None` otherwise.
    fn as_bool(self) -> Option<bool>;

    /// If the JSON is a number, returns the associated number. Returns `None` otherwise.
    fn as_number(self) -> Option<Number>;

    /// If the JSON is a string, returns the associated string. Returns `None` otherwise.
    fn as_str(self) -> Option<&'a str>;

    /// If the JSON is an array, returns the associated array. Returns `None` otherwise.
    fn as_array(self) -> Option<Self::Array>;

    /// If the JSON is an object, returns the associated object. Returns `None` otherwise.
    fn as_object(self) -> Option<Self::Object>;

    /// Returns `true` if the value is null.
    fn is_null(self) -> bool;

    /// Returns `true` if the value is a boolean.
    fn is_bool(self) -> bool {
        self.as_bool().is_some()
    }

    /// Returns `true` if the value is a number.
    fn is_number(self) -> bool {
        self.as_number().is_some()
    }

    /// Returns `true` if the value is a string.
    fn is_string(self) -> bool {
        self.as_str().is_some()
    }

    /// Returns `true` if the value is an array.
    fn is_array(self) -> bool {
        self.as_array().is_some()
    }

    /// Returns `true` if the value is an object.
    fn is_object(self) -> bool {
        self.as_object().is_some()
    }
}

/// A trait for borrowed JSON arrays.
pub trait ArrayRef<'a>: Copy {
    /// The type of borrowed JSON values.
    type JsonRef: JsonRef<'a>;

    /// Returns the length of the array.
    fn len(self) -> usize;

    /// Returns the value at the given index.
    fn get(self, index: usize) -> Option<Self::JsonRef>;

    /// Returns all values in the array.
    fn list(self) -> Vec<Self::JsonRef>;
}

/// A trait for borrowed JSON objects.
pub trait ObjectRef<'a>: Copy {
    /// The type of borrowed JSON values.
    type JsonRef: JsonRef<'a>;

    // Returns the number of elements in the object.
    fn len(self) -> usize;

    /// Returns the value associated with the given key.
    fn get(self, key: &str) -> Option<Self::JsonRef>;

    /// Returns all values in the object.
    fn list_value(self) -> Vec<Self::JsonRef>;
}
