use thiserror::Error;

use num_traits::FromPrimitive;

pub use ciborium;

use colabrodo_common::common::{NooValueMap, ServerMessages};
use colabrodo_common::nooid::NooID;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Decode error")]
    DecodeError(String),
    #[error("Root message is not valid")]
    InvalidRootMessage(String),
}

#[derive(Debug)]
pub enum UserError {
    InternalError,
}

pub trait UserClientState {
    fn handle_message(
        &mut self,
        message: ServerMessages,
        content: &NooValueMap,
    ) -> Result<(), UserError>;
}

/// Handle a CBOR message from a server, sanitize, and then pass it along to
/// some state
///
pub fn handle_next<U: UserClientState>(
    root: &ciborium::value::Value,
    state: &mut U,
) -> Result<(), ClientError> {
    if !root.is_array() {
        return Err(ClientError::InvalidRootMessage(
            "Root is not an array".to_string(),
        ));
    }

    let array = root.as_array().unwrap();

    if array.len() % 2 != 0 {
        return Err(ClientError::InvalidRootMessage(
            "Root array is not a multiple of 2".to_string(),
        ));
    }

    for i in (0..array.len()).step_by(2) {
        let mid = array[i].as_integer();

        if mid.is_none() {
            return Err(ClientError::InvalidRootMessage(
                "Missing message id value".to_string(),
            ));
        }

        let mid = u32::try_from(mid.unwrap()).unwrap();

        let msg: Option<ServerMessages> = FromPrimitive::from_u32(mid);

        if msg.is_none() {
            return Err(ClientError::InvalidRootMessage(
                format!("Message id was decoded as {mid}, but this does not correspond with a known message type."),
            ));
        }

        let content = array[i + 1].as_map();

        if content.is_none() {
            return Err(ClientError::InvalidRootMessage(
                format!("Message id was decoded as {mid}, but the message content is missing."),
            ));
        }

        let content = content.unwrap();

        state.handle_message(msg.unwrap(), content).unwrap();
    }

    Ok(())
}

pub trait ComponentListHandler {
    fn insert(id: NooID, values: NooValueMap);
    fn update(id: NooID, values: NooValueMap);
    fn delete(id: NooID, values: NooValueMap);
}

pub struct EmptyList {}

impl ComponentListHandler for EmptyList {
    fn insert(_id: NooID, _values: NooValueMap) {}
    fn update(_id: NooID, _values: NooValueMap) {}
    fn delete(_id: NooID, _values: NooValueMap) {}
}
