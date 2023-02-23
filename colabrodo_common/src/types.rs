use ciborium::tag::Required;
use serde::de::Error;
use serde::{de::Visitor, Deserialize, Serialize};

use crate::nooid::NooID;

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Default)]
pub enum Format {
    #[default]
    U8,
    U16,
    U32,

    U8VEC4,

    U16VEC2,

    VEC2,
    VEC3,
    VEC4,

    MAT3,
    MAT4,
}

pub type RGB = [f32; 3];
pub type RGBA = [f32; 4];

pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

pub type Mat3 = [f32; 9];
pub type Mat4 = [f32; 16];

// =============================================================================

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

// =============================================================================

/// A struct to represent an array of bytes, for proper serialization to CBOR
#[derive(Debug, Default, PartialEq)]
pub struct ByteBuff(Vec<u8>);

impl ByteBuff {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0.as_slice()
    }
}

impl Serialize for ByteBuff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.as_slice())
    }
}

struct ByteBuffVisitor;

impl<'de> Visitor<'de> for ByteBuffVisitor {
    type Value = ByteBuff;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "byte buffer")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut vec = Vec::new();
        vec.extend_from_slice(v);
        Ok(ByteBuff::new(vec))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(ByteBuff::new(v))
    }
}

impl<'de> Deserialize<'de> for ByteBuff {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(ByteBuffVisitor)
    }
}

// =============================================================================

/// A struct to represent a URL, for proper serialization to CBOR
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Url(Required<String, 32>);

impl Url {
    pub fn new(url: String) -> Self {
        Self(Required(url))
    }
    pub fn new_from_slice(url: &str) -> Self {
        Self(Required(url.to_string()))
    }
}

// impl Serialize for Url {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         self.url.serialize(serializer)
//     }
// }

// struct UrlVisitor;

// impl<'de> Visitor<'de> for UrlVisitor {
//     type Value = Url;

//     fn expecting(
//         &self,
//         formatter: &mut std::fmt::Formatter,
//     ) -> std::fmt::Result {
//         write!(formatter, "URL")
//     }

//     fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
//     where
//         E: Error,
//     {
//         let copy = Vec::new();
//         copy.copy_from_slice(v);
//         Ok(ByteBuff { bytes: copy })
//     }
// }

// impl<'de> Deserialize<'de> for ByteBuff {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         deserializer.deserialize_seq(ByteBuffVisitor)
//     }
// }

// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CommonDeleteMessage {
    pub id: NooID,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestBytesStruct {
        bytes: super::ByteBuff,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestUrlStruct {
        data: super::Url,
    }

    #[test]
    fn bytes_serde() {
        let to_pack = TestBytesStruct {
            bytes: super::ByteBuff::new(vec![10, 11, 12]),
        };

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&to_pack, &mut pack).expect("Pack");

        let other: TestBytesStruct =
            match ciborium::de::from_reader(pack.as_slice()) {
                Ok(x) => x,
                Err(x) => panic!("Unpack failed: {x:?}"),
            };

        assert_eq!(to_pack, other);
    }

    #[test]
    fn url_serde() {
        let to_pack = TestUrlStruct {
            data: super::Url::new("http://google.com".to_string()),
        };

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&to_pack, &mut pack).expect("Pack");

        let other: TestUrlStruct =
            match ciborium::de::from_reader(pack.as_slice()) {
                Ok(x) => x,
                Err(x) => panic!("Unpack failed: {x:?}"),
            };

        assert_eq!(to_pack, other);
    }
}
