use serde::de::Error;
use serde::{de::Visitor, Deserialize, Serialize};

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
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ByteBuff(Vec<u8>);

impl ByteBuff {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CommonDeleteMessage<IDType> {
    pub id: IDType,
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
        data: url::Url,
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
            data: "http://google.com".parse().unwrap(),
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

    #[test]
    fn url_opt_serde() {
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct Thingy {
            v: Option<url::Url>,
        }

        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct Host {
            #[serde(flatten)]
            parts: Thingy,
        }

        let to_pack = Host {
            parts: Thingy {
                v: Some("http://google.com".parse().unwrap()),
            },
        };

        let mut pack = Vec::<u8>::new();

        ciborium::ser::into_writer(&to_pack, &mut pack).expect("Pack");

        let other: Host = match ciborium::de::from_reader(pack.as_slice()) {
            Ok(x) => x,
            Err(x) => panic!("Unpack failed: {x:?}"),
        };

        assert_eq!(to_pack, other);
    }
}
