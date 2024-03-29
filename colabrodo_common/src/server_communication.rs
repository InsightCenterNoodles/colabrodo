//! Components and tools for server-sourced method and signal messages

use colabrodo_macros::DeltaPatch;
use serde::{Deserialize, Serialize};
use serde_with;

use crate::components::{DeltaPatch, UpdatableWith};
use crate::{common::ServerMessageIDs, nooid::NooID};

/// Associates a NOODLES message ID with structs that implement this trait
pub trait ServerMessageID {
    fn message_id() -> u32;
}

/// A way to descriminate the target of a signal
#[serde_with::skip_serializing_none]
#[derive(Debug, Deserialize, Serialize)]
pub struct SignalInvokeObj<EntityRef, TableRef, PlotRef> {
    pub entity: Option<EntityRef>,
    pub table: Option<TableRef>,
    pub plot: Option<PlotRef>,
}

impl<EntityRef, TableRef, PlotRef> Default
    for SignalInvokeObj<EntityRef, TableRef, PlotRef>
{
    fn default() -> Self {
        Self {
            entity: None,
            table: None,
            plot: None,
        }
    }
}

// =============================================================================

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, DeltaPatch)]
#[patch_generic(MethodRef, SignalRef)]
pub struct DocumentUpdate<MethodRef, SignalRef> {
    pub methods_list: Option<Vec<MethodRef>>,
    pub signals_list: Option<Vec<SignalRef>>,
}

impl<MethodRef, SignalRef> UpdatableWith
    for DocumentUpdate<MethodRef, SignalRef>
{
    type Substate = DocumentUpdate<MethodRef, SignalRef>;

    fn update(&mut self, s: Self::Substate) {
        self.patch(s);
    }
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
#[derive(Debug, Serialize, Deserialize)]
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
    pub fn method_not_found(optional_info: Option<&str>) -> Self {
        Self {
            code: ExceptionCodes::MethodNotFound as i32,
            message: optional_info.map(|f| f.to_string()),
            ..Default::default()
        }
    }

    pub fn invalid_parameters(optional_info: Option<&str>) -> Self {
        Self {
            code: ExceptionCodes::InvalidParameters as i32,
            message: optional_info.map(|f| f.to_string()),
            ..Default::default()
        }
    }

    pub fn internal_error(optional_info: Option<&str>) -> Self {
        Self {
            code: ExceptionCodes::InternalError as i32,
            message: optional_info.map(|f| f.to_string()),
            ..Default::default()
        }
    }
}

pub enum ExceptionCodes {
    ParseError = -32700,        // Invalid CBOR was provided
    InvalidRequest = -32600,    // The request is not valid
    MethodNotFound = -32601,    // The method is not available
    InvalidParameters = -32602, // The parameters to the method are incorrect
    InternalError = -32603,     // An internal server error
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
