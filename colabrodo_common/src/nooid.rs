use ciborium::{cbor, value};
use serde::{de::Visitor, ser::SerializeTuple, Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct NooID {
    slot: u32,
    gen: u32,
}

impl serde::Serialize for NooID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_tuple(2)?;
        s.serialize_element(&self.slot)?;
        s.serialize_element(&self.gen)?;
        s.end()
    }
}

struct IDTypeDeserializeVisitor;

impl<'de> Visitor<'de> for IDTypeDeserializeVisitor {
    type Value = NooID;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "A tuple with a slot and generation")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let slot_elem: Option<u32> = seq.next_element()?;
        let gen_elem: Option<u32> = seq.next_element()?;

        Ok(NooID {
            slot: slot_elem.unwrap_or(u32::MAX),
            gen: gen_elem.unwrap_or(u32::MAX),
        })
    }
}

impl<'de> Deserialize<'de> for NooID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, IDTypeDeserializeVisitor)
    }
}

impl Default for NooID {
    fn default() -> Self {
        Self {
            slot: u32::MAX,
            gen: u32::MAX,
        }
    }
}

impl NooID {
    pub fn next_generation(old: Self) -> Self {
        Self {
            slot: old.slot,
            gen: old.gen + 1,
        }
    }

    pub fn new(slot: u32, gen: u32) -> Self {
        Self { slot, gen }
    }

    pub fn new_with_slot(slot: u32) -> Self {
        Self { slot, gen: 0 }
    }

    pub fn slot(&self) -> u32 {
        self.slot
    }

    pub fn gen(&self) -> u32 {
        self.gen
    }

    pub fn valid(&self) -> bool {
        self.slot == u32::MAX || self.gen == u32::MAX
    }

    pub fn from_value(v: &value::Value) -> Option<NooID> {
        let arr = v.as_array()?;

        let slot: u32 = arr.get(0)?.as_integer()?.try_into().ok()?;
        let gen: u32 = arr.get(1)?.as_integer()?.try_into().ok()?;

        Some(Self { slot, gen })
    }
}

impl std::fmt::Display for NooID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ID({},{})", self.slot, self.gen)
    }
}

impl From<NooID> for value::Value {
    fn from(item: NooID) -> Self {
        cbor!([item.slot, item.gen]).unwrap()
    }
}

// =============================================================================

#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct MethodID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct SignalID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct EntityID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct PlotID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct BufferID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct BufferViewID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct MaterialID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct ImageID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct TextureID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct SamplerID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct LightID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct GeometryID(pub NooID);
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct TableID(pub NooID);

pub trait IDClass:
    core::fmt::Debug + Copy + Clone + Serialize + std::hash::Hash + PartialEq + Eq
{
    fn new(n: NooID) -> Self;
    fn as_nooid(&self) -> NooID;
}

impl IDClass for MethodID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for SignalID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for EntityID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for PlotID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for TableID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for BufferID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for BufferViewID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for MaterialID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for ImageID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for TextureID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for SamplerID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for LightID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
impl IDClass for GeometryID {
    fn new(n: NooID) -> Self {
        Self(n)
    }
    fn as_nooid(&self) -> NooID {
        self.0
    }
}
