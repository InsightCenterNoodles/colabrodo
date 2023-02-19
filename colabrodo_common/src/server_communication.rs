use serde::{
    de::{Error, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Serialize,
};
use serde_with;

use crate::{common::ServerMessageIDs, nooid::NooID};

pub trait ServerMessageID {
    fn message_id() -> u32;
}

//
pub enum SignalInvokeObj<EntityRef, TableRef, PlotRef> {
    Entity(EntityRef),
    Table(TableRef),
    Plot(PlotRef),
}

impl<EntityRef, TableRef, PlotRef> Serialize
    for SignalInvokeObj<EntityRef, TableRef, PlotRef>
where
    EntityRef: Serialize,
    TableRef: Serialize,
    PlotRef: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("InvokeIDType", 1)?;
        match self {
            SignalInvokeObj::Entity(e) => s.serialize_field("entity", e)?,
            SignalInvokeObj::Table(e) => s.serialize_field("table", e)?,
            SignalInvokeObj::Plot(e) => s.serialize_field("plot", e)?,
        }
        s.end()
    }
}

struct SignalInvokeObjVisitor<EntityRef, TableRef, PlotRef> {
    phantom0: std::marker::PhantomData<EntityRef>,
    phantom1: std::marker::PhantomData<TableRef>,
    phantom2: std::marker::PhantomData<PlotRef>,
}

impl<'de, EntityRef, TableRef, PlotRef> Visitor<'de>
    for SignalInvokeObjVisitor<EntityRef, TableRef, PlotRef>
where
    EntityRef: Deserialize<'de>,
    TableRef: Deserialize<'de>,
    PlotRef: Deserialize<'de>,
{
    type Value = SignalInvokeObj<EntityRef, TableRef, PlotRef>;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(formatter, "invoke context field")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let key: Option<String> = map.next_key()?;

        if key.is_none() {
            return Err(Error::missing_field("invoke context field"));
        }

        match key.unwrap().as_str() {
            "entity" => Ok(Self::Value::Entity(map.next_value()?)),
            "table" => Ok(Self::Value::Table(map.next_value()?)),
            "plot" => Ok(Self::Value::Plot(map.next_value()?)),
            _ => Err(Error::missing_field("missing context field")),
        }
    }
}

impl<'de, EntityRef, TableRef, PlotRef> Deserialize<'de>
    for SignalInvokeObj<EntityRef, TableRef, PlotRef>
where
    EntityRef: Deserialize<'de>,
    TableRef: Deserialize<'de>,
    PlotRef: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(SignalInvokeObjVisitor::<
            EntityRef,
            TableRef,
            PlotRef,
        > {
            phantom0: std::marker::PhantomData,
            phantom1: std::marker::PhantomData,
            phantom2: std::marker::PhantomData,
        })
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentUpdate<MethodRef, SignalRef> {
    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentReset {}

impl<MethodRef, SignalRef> Default for DocumentUpdate<MethodRef, SignalRef> {
    fn default() -> Self {
        Self {
            methods_list: Default::default(),
            signals_list: Default::default(),
        }
    }
}

impl<MethodRef, SignalRef> ServerMessageID
    for DocumentUpdate<MethodRef, SignalRef>
{
    fn message_id() -> u32 {
        ServerMessageIDs::MsgDocumentUpdate as u32
    }
}

impl ServerMessageID for DocumentReset {
    fn message_id() -> u32 {
        ServerMessageIDs::MsgDocumentReset as u32
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize)]
pub struct MessageSignalInvoke<EntityRef, TableRef, PlotRef> {
    pub id: NooID,
    pub context: Option<SignalInvokeObj<EntityRef, TableRef, PlotRef>>,
    pub signal_data: Vec<ciborium::value::Value>,
}

impl<EntityRef, TableRef, PlotRef> ServerMessageID
    for MessageSignalInvoke<EntityRef, TableRef, PlotRef>
{
    fn message_id() -> u32 {
        ServerMessageIDs::MsgSignalInvoke as u32
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MethodException {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<ciborium::value::Value>,
}

impl MethodException {
    pub fn method_not_found(optional_info: Option<String>) -> Self {
        Self {
            code: ExceptionCodes::MethodNotFound as i32,
            message: optional_info,
            ..Default::default()
        }
    }
}

pub enum ExceptionCodes {
    MethodNotFound = -32601,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MessageMethodReply {
    pub invoke_id: String,
    pub result: Option<ciborium::value::Value>,
    pub method_exception: Option<MethodException>,
}

impl ServerMessageID for MessageMethodReply {
    fn message_id() -> u32 {
        ServerMessageIDs::MsgMethodReply as u32
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentInit {}

impl ServerMessageID for DocumentInit {
    fn message_id() -> u32 {
        ServerMessageIDs::MsgDocumentInitialized as u32
    }
}
