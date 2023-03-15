//! Tools to help work with CBOR values

use std::collections::HashMap;

pub use ciborium::value::Value;

pub use colabrodo_macros::CBORTransform;

type NooHashMap = HashMap<String, Value>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum FromValueError {
    #[error("Required field {0} is missing")]
    MissingField(String),
    #[error("Type Mismatch: expected: {expected:?}, found {found:?}")]
    WrongType { expected: String, found: String },
    #[error("Integer Cast: refusing to squash integer to destination")]
    IntegerCast,
    #[error("Array Underflow: source array has insufficient elements")]
    ArrayUnderflow,
}

/// Dictionaries are represented by the decoding library we use by a list of keys and values.
pub type NooLinearMap = Vec<(Value, Value)>;

/// Look up a value in a map
pub fn lookup<'a>(v: &Value, map: &'a NooLinearMap) -> Option<&'a Value> {
    for e in map {
        if &e.0 == v {
            return Some(&e.1);
        }
    }
    None
}

/// Convert the linear representation of a dictionary to a mapped representation
pub fn convert_to_value_map(v: NooLinearMap) -> NooHashMap {
    let mut ret: NooHashMap = NooHashMap::new();
    for (key, value) in v {
        let conv_key = key.as_text();
        if conv_key.is_none() {
            continue;
        }
        ret.insert(conv_key.unwrap().to_string(), value);
    }
    ret
}

/// Get a value from a CBOR map
#[inline]
pub fn get_map(m: &mut NooHashMap, key: &str) -> Result<Value, FromValueError> {
    m.remove(key)
        .ok_or_else(|| FromValueError::MissingField(key.to_string()))
}

/// Get a string corresponding to a CBOR type
pub fn get_value_type(value: &Value) -> String {
    match value {
        Value::Integer(_) => "Integer".into(),
        Value::Bytes(_) => "Bytes".into(),
        Value::Float(_) => "Float".into(),
        Value::Text(_) => "Text".into(),
        Value::Bool(_) => "Bool".into(),
        Value::Null => "Null".into(),
        Value::Tag(_, x) => format!("Tagged: {}", get_value_type(x)),
        Value::Array(_) => "Array".into(),
        Value::Map(_) => "Map".into(),
        _ => "Unknown".into(),
    }
}

// =============================================================================

/// Trait for converting between native and CBOR representation
pub trait CBORTransform
where
    Self: Sized,
{
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError>;
    fn to_cbor(&self) -> Value;
}

/// Convert a CBOR value to a type
#[inline]
pub fn from_cbor<T: CBORTransform>(value: Value) -> Result<T, FromValueError> {
    T::try_from_cbor(value)
}

/// Convert a list of CBOR values to a list of types
#[inline]
pub fn from_cbor_list<T: CBORTransform>(
    value: Vec<Value>,
) -> Result<Vec<T>, FromValueError> {
    Ok(value
        .into_iter()
        .filter_map(|v| T::try_from_cbor(v).ok())
        .collect())
}

/// Convert from an optional value to an optional type
#[inline]
pub fn from_cbor_option<T: CBORTransform>(
    value: Option<Value>,
) -> Result<Option<T>, FromValueError> {
    value.map(|v| T::try_from_cbor(v)).transpose()
}

/// Convert an object to CBOR
#[inline]
pub fn to_cbor<T: CBORTransform>(t: &T) -> Value {
    t.to_cbor()
}

// =============================================================================

impl CBORTransform for std::string::String {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Text(s) = value {
            return Ok(s);
        }
        Err(FromValueError::WrongType {
            expected: "Integer".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Text(self.clone())
    }
}

impl CBORTransform for f32 {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Float(s) = value {
            return Ok(s as f32);
        }
        Err(FromValueError::WrongType {
            expected: "Float".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Float(*self as f64)
    }
}

impl CBORTransform for f64 {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Float(s) = value {
            return Ok(s as f64);
        }
        Err(FromValueError::WrongType {
            expected: "Float".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Float(*self)
    }
}

impl CBORTransform for u32 {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Integer(s) = value {
            return s.try_into().map_err(|_| FromValueError::IntegerCast);
        }
        Err(FromValueError::WrongType {
            expected: "Integer".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Integer(ciborium::value::Integer::from(*self))
    }
}

impl CBORTransform for i32 {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Integer(s) = value {
            return s.try_into().map_err(|_| FromValueError::IntegerCast);
        }
        Err(FromValueError::WrongType {
            expected: "Integer".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Integer(ciborium::value::Integer::from(*self))
    }
}

impl CBORTransform for i64 {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Integer(s) = value {
            return s.try_into().map_err(|_| FromValueError::IntegerCast);
        }
        Err(FromValueError::WrongType {
            expected: "Integer".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Integer(ciborium::value::Integer::from(*self))
    }
}

impl CBORTransform for Value {
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        Ok(value)
    }

    fn to_cbor(&self) -> Value {
        self.clone()
    }
}

impl<T> CBORTransform for Vec<T>
where
    T: CBORTransform,
{
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Array(l) = value {
            // we can use fun maps, but this is just easier
            let mut ret = Self::new();
            ret.reserve(l.len());
            for v in l {
                ret.push(from_cbor(v)?);
            }
            return Ok(ret);
        }
        Err(FromValueError::WrongType {
            expected: "Array".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        Value::Array(self.iter().map(to_cbor).collect())
    }
}

impl<T, const N: usize> CBORTransform for [T; N]
where
    T: CBORTransform,
{
    fn try_from_cbor(value: Value) -> Result<Self, FromValueError> {
        if let Value::Array(l) = value {
            if l.len() < N {
                return Err(FromValueError::ArrayUnderflow);
            }

            let mut iter = l.into_iter();

            let ret: [_; N] =
                core::array::from_fn(|_| from_cbor(iter.next().unwrap()));

            for i in &ret {
                if let Err(x) = i {
                    return Err(x.clone());
                }
            }

            let mut iter = ret.into_iter();

            let ret: Self =
                core::array::from_fn(|_| iter.next().unwrap().unwrap());

            return Ok(ret);
        }
        Err(FromValueError::WrongType {
            expected: "Array".into(),
            found: get_value_type(&value),
        })
    }

    fn to_cbor(&self) -> Value {
        todo!()
    }
}

// =============================================================================

/// Transform a list of values to CBOR
#[macro_export]
macro_rules! tf_to_cbor {
    [$( $x:expr ), *] => {
        {
            let mut ret = Vec::<Value>::new();

            $(
                ret.push( $x.to_cbor() );
            )*

            ret
        }
    };
}

/// Make a reusable function that consumes a CBOR value, and attempts to turn it into a tuple of values.
#[macro_export]
macro_rules! make_decode_function {
    ($y:ident, $( $x:ty ),*) => {
        fn $y (v: Value) -> Option<( $( $x, )* )> {
            if let Value::Array(arr) = v {
                let mut arr = arr.into_iter();
                return Some( (
                $(
                    from_cbor::<$x>(arr.next()?).ok()?,
                )*
                ) );
            }
            None
        }
    };
}

/// Convert a list of values (such as one would get from a method) into a tuple of types.
#[macro_export]
macro_rules! arg_to_tuple {
    ($y:expr, $( $x:ty ),*) => {
        {
            fn arg_to_tuple_check(v: Vec<Value>) -> Option<( $( $x, )* )> {
                let mut arr = v.into_iter();
                return Some( (
                $(
                    from_cbor::<$x>(arr.next()?).ok()?,
                )*
                ) );
            }

            arg_to_tuple_check($y)
        }
    };
}

// =============================================================================

#[cfg(test)]
mod tests {
    use ciborium::cbor;

    use super::*;

    #[derive(CBORTransform, Debug, PartialEq)]
    pub struct Selection {
        pub name: String,
        pub rows: Vec<f32>,
        #[vserde(rename = "other")]
        pub row_ranges: Option<Vec<i64>>,
    }

    #[test]
    fn transform() {
        let x = Selection {
            name: "A Name".to_string(),
            rows: vec![0.0, 1.0, 2.0],
            row_ranges: Some(vec![5, 6, 7]),
        };

        let c = x.to_cbor();

        let dx: Selection = from_cbor(c).unwrap();

        assert_eq!(x, dx);

        let raw = cbor!(
            {
                "name" => "A Name",
                "rows" => [0.0, 1.0, 2.0],
                "other" => [5, 6, 7],
            }
        )
        .unwrap();

        let ex: Selection = from_cbor(raw).unwrap();

        assert_eq!(x, ex);
    }

    #[test]
    fn macro_test() {
        let v = tf_to_cbor![
            1,
            2.0,
            Selection {
                name: "A Name".to_string(),
                rows: vec![0.0, 1.0, 2.0],
                row_ranges: Some(vec![5, 6, 7]),
            }
        ];

        let raw = cbor!(
        [
        1,
        2.0,
        {
            "name" => "A Name",
            "rows" => [0.0, 1.0, 2.0],
            "other" => [5, 6, 7],
        }
        ])
        .unwrap();

        make_decode_function!(to_tup, i32, f32, Selection);

        let c = to_tup(Value::Array(v)).unwrap();
        let d = to_tup(raw).unwrap();

        assert_eq!(c, d);
    }
}
