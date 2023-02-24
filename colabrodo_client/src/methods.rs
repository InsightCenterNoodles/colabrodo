use crate::client::*;
use ciborium::value::Value;
use colabrodo_common::client_communication::{
    ClientInvokeMessage, InvokeIDType,
};
use colabrodo_common::common::strings;
use colabrodo_common::tf_to_cbor;
use colabrodo_common::value_tools::*;

pub type NewMethodResult = Option<(uuid::Uuid, ClientInvokeMessage)>;

pub fn send_method(
    state: &mut impl UserClientState,
    context: Option<InvokeIDType>,
    method_name: &str,
    args: Vec<Value>,
) -> NewMethodResult {
    let invoke_id = uuid::Uuid::new_v4();

    state.method_list().get_id_by_name(method_name).map(|id| {
        (
            invoke_id,
            ClientInvokeMessage {
                method: *id,
                context,
                invoke_id: Some(invoke_id.to_string()),
                args,
            },
        )
    })
}

pub fn set_position(
    state: &mut impl UserClientState,
    context: Option<InvokeIDType>,
    p: [f32; 3],
) -> NewMethodResult {
    send_method(state, context, strings::MTHD_SET_POSITION, tf_to_cbor![p])
}
