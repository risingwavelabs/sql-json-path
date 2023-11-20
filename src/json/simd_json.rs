use super::*;
use ::simd_json::{
    prelude::{TypedContainerValue, TypedScalarValue, ValueAsContainer, ValueAsScalar},
    value::{
        borrowed::Object as BorrowedObject, owned::Object as OwnedObject, BorrowedValue, OwnedValue,
    },
    StaticNode,
};

impl<'b> Json for BorrowedValue<'b> {
    type Borrowed<'a> = &'a BorrowedValue<'b> where 'b: 'a;

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
        ValueAsContainer::as_array(self)
    }

    fn as_object(self) -> Option<Self::Object> {
        ValueAsContainer::as_object(self)
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
        TypedContainerValue::is_array(self)
    }

    fn is_object(self) -> bool {
        TypedContainerValue::is_object(self)
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
        ValueAsContainer::as_array(self)
    }

    fn as_object(self) -> Option<Self::Object> {
        ValueAsContainer::as_object(self)
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
        TypedContainerValue::is_array(self)
    }

    fn is_object(self) -> bool {
        TypedContainerValue::is_object(self)
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

    fn list_value(self) -> Vec<Self::JsonRef> {
        self.values().collect()
    }
}
