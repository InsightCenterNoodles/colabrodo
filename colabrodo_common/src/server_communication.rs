use serde::{ser::SerializeStruct, Deserialize, Serialize};
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

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct DocumentUpdate<MethodRef, SignalRef> {
    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
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
#[derive(Serialize)]
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
#[derive(Debug, Default, Serialize)]
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
#[derive(Debug, Default, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct DocumentInit {}

impl ServerMessageID for DocumentInit {
    fn message_id() -> u32 {
        ServerMessageIDs::MsgDocumentInitialized as u32
    }
}
