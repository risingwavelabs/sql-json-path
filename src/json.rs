use crate::node::Value;

pub trait Json {
    type Borrowed<'a>: JsonRef
    where
        Self: 'a;
    fn as_ref(&self) -> Self::Borrowed<'_>;
    fn from_value(v: Value) -> Self;
}

pub trait JsonRef: Clone {
    type Owned;
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
}

impl JsonRef for &serde_json::Value {
    type Owned = serde_json::Value;
}
