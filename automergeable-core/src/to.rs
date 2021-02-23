use std::{collections::HashMap, convert::TryInto, time};

use automerge::{MapType, ScalarValue};

/// Require a method to convert the current value into an automerge value
pub trait ToAutomerge {
    fn to_automerge(&self) -> automerge::Value;
}

impl ToAutomerge for Vec<char> {
    fn to_automerge(&self) -> automerge::Value {
        automerge::Value::Text(self.clone())
    }
}

impl<T> ToAutomerge for Vec<T>
where
    T: ToAutomerge,
{
    fn to_automerge(&self) -> automerge::Value {
        let vals = self.iter().map(|v| v.to_automerge()).collect::<Vec<_>>();
        automerge::Value::Sequence(vals)
    }
}

impl<K, V> ToAutomerge for HashMap<K, V>
where
    K: ToString,
    V: ToAutomerge,
{
    fn to_automerge(&self) -> automerge::Value {
        let mut hm = HashMap::new();
        for (k, v) in self {
            hm.insert(k.to_string(), v.to_automerge());
        }
        automerge::Value::Map(hm, MapType::Map)
    }
}

impl ToAutomerge for String {
    fn to_automerge(&self) -> automerge::Value {
        ScalarValue::Str(self.to_owned()).into()
    }
}

impl ToAutomerge for i64 {
    fn to_automerge(&self) -> automerge::Value {
        ScalarValue::Int(*self).into()
    }
}

impl ToAutomerge for u64 {
    fn to_automerge(&self) -> automerge::Value {
        ScalarValue::Uint(*self).into()
    }
}

impl ToAutomerge for f64 {
    fn to_automerge(&self) -> automerge::Value {
        ScalarValue::F64(*self).into()
    }
}

impl ToAutomerge for f32 {
    fn to_automerge(&self) -> automerge::Value {
        ScalarValue::F32(*self).into()
    }
}

impl ToAutomerge for bool {
    fn to_automerge(&self) -> automerge::Value {
        ScalarValue::Boolean(*self).into()
    }
}

impl<T> ToAutomerge for Option<T>
where
    T: ToAutomerge,
{
    fn to_automerge(&self) -> automerge::Value {
        if let Some(v) = self {
            v.to_automerge()
        } else {
            ScalarValue::Null.into()
        }
    }
}

impl ToAutomerge for time::SystemTime {
    fn to_automerge(&self) -> automerge::Value {
        let ts = self
            .duration_since(time::UNIX_EPOCH)
            .expect("time went backwards");
        ScalarValue::Timestamp(ts.as_secs().try_into().unwrap()).into()
    }
}
