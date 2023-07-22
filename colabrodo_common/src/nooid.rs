//! ID related tools and specializations

use ciborium::{cbor, value};
use serde::{de::Visitor, ser::SerializeTuple, Deserialize, Serialize};

/// A basic NOODLES ID
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
    /// Create the next generation ID from a given ID
    pub fn next_generation(old: Self) -> Self {
        Self {
            slot: old.slot,
            gen: old.gen + 1,
        }
    }

    /// Create a new ID from a slot and generation
    pub fn new(slot: u32, gen: u32) -> Self {
        Self { slot, gen }
    }

    /// Create a new ID from a slot, with generation 0
    pub fn new_with_slot(slot: u32) -> Self {
        Self { slot, gen: 0 }
    }

    /// Get the slot of the ID
    pub fn slot(&self) -> u32 {
        self.slot
    }

    /// Get the generation of the ID
    pub fn gen(&self) -> u32 {
        self.gen
    }

    /// Asks if the ID is valid, that is, if both slot and generation
    /// are not u32 max.
    pub fn valid(&self) -> bool {
        self.slot != u32::MAX && self.gen != u32::MAX
    }

    /// Create an ID from a CBOR value
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

macro_rules! declare_id_type {
    ($id:ident) => {
        #[derive(
            Copy,
            Clone,
            Debug,
            Default,
            PartialEq,
            Eq,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $id(pub NooID);

        impl IDClass for $id {
            fn new(n: NooID) -> Self {
                Self(n)
            }
            fn as_nooid(&self) -> NooID {
                self.0
            }
        }
    };
}

declare_id_type!(MethodID);
declare_id_type!(SignalID);
declare_id_type!(EntityID);
declare_id_type!(PlotID);
declare_id_type!(BufferID);
declare_id_type!(BufferViewID);
declare_id_type!(MaterialID);
declare_id_type!(ImageID);
declare_id_type!(TextureID);
declare_id_type!(SamplerID);
declare_id_type!(LightID);
declare_id_type!(GeometryID);
declare_id_type!(TableID);

pub trait IDClass:
    core::fmt::Debug + Copy + Clone + Serialize + std::hash::Hash + PartialEq + Eq
{
    fn new(n: NooID) -> Self;
    fn as_nooid(&self) -> NooID;
}
