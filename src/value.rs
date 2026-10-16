use bytes::BytesMut;
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Uuid(uuid::Uuid),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    DateTime(chrono::NaiveDateTime),
    DateTimeUtc(chrono::DateTime<chrono::Utc>),
}

pub trait IntoValue {
    fn into_value(self) -> Value;
}
pub trait ToValue {
    fn to_value(&self) -> Value;
}

macro_rules! value_impl {
    ($type:ty, $variant:ident) => {
        impl IntoValue for $type {
            fn into_value(self) -> Value {
                Value::$variant(self)
            }
        }
        impl ToValue for $type {
            fn to_value(&self) -> Value {
                Value::$variant(self.clone())
            }
        }
    };
}

value_impl!(bool, Bool);
value_impl!(i16, I16);
value_impl!(i32, I32);
value_impl!(i64, I64);
value_impl!(f32, F32);
value_impl!(f64, F64);
value_impl!(String, Text);
value_impl!(Vec<u8>, Bytes);
value_impl!(serde_json::Value, Json);
value_impl!(uuid::Uuid, Uuid);
value_impl!(chrono::NaiveDate, Date);
value_impl!(chrono::NaiveTime, Time);
value_impl!(chrono::NaiveDateTime, DateTime);
value_impl!(chrono::DateTime<chrono::Utc>, DateTimeUtc);

impl IntoValue for i8 {
    fn into_value(self) -> Value {
        Value::I16(self.into())
    }
}
impl ToValue for i8 {
    fn to_value(&self) -> Value {
        Value::I16((*self).into())
    }
}
impl IntoValue for u8 {
    fn into_value(self) -> Value {
        Value::I16(self.into())
    }
}
impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::I16((*self).into())
    }
}
impl IntoValue for u16 {
    fn into_value(self) -> Value {
        Value::I32(self.into())
    }
}
impl ToValue for u16 {
    fn to_value(&self) -> Value {
        Value::I32((*self).into())
    }
}
impl IntoValue for u32 {
    fn into_value(self) -> Value {
        Value::I64(self.into())
    }
}
impl ToValue for u32 {
    fn to_value(&self) -> Value {
        Value::I64((*self).into())
    }
}
impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Text(self.to_owned())
    }
}
impl IntoValue for &[u8] {
    fn into_value(self) -> Value {
        Value::Bytes(self.to_vec())
    }
}

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        self.map(IntoValue::into_value).unwrap_or(Value::Null)
    }
}
impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value {
        self.as_ref().map(ToValue::to_value).unwrap_or(Value::Null)
    }
}

#[derive(Debug)]
pub(crate) struct NullValue;

pub(crate) static NULL_VALUE: NullValue = NullValue;

impl ToSql for NullValue {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let _ = (ty, out);
        Ok(IsNull::Yes)
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    to_sql_checked!();
}
