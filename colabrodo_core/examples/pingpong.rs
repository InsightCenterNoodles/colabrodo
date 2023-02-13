use colabrodo_core::server_messages::{ComponentReference, MethodArg};
use colabrodo_core::{
    server_messages::MethodState,
    server_state::{InvokeObj, MethodResult, ServerState, UserServerState},
};
use std::collections::HashMap;

type Function = fn(
    &mut PingPongServer,
    &InvokeObj,
    Vec<ciborium::value::Value>,
) -> MethodResult;

struct PingPongServer {
    state: ServerState,

    method_list: HashMap<ComponentReference<MethodState>, Function>,
}

fn ping_pong(
    _state: &mut PingPongServer,
    _context: &InvokeObj,
    args: Vec<ciborium::value::Value>,
) -> MethodResult {
    Ok(ciborium::value::Value::Array(args))
}

impl UserServerState for PingPongServer {
    fn new(tx: colabrodo_core::server_state::CallbackPtr) -> Self {
        Self {
            state: ServerState::new(tx.clone()),
            method_list: Default::default(),
        }
    }

    fn initialize_state(&mut self) {
        println!("Initializing ping pong state");
        let ptr = self.state.methods.new_component(MethodState {
            name: "ping_pong".to_string(),
            doc: Some(
                "This method just replies with what you send it.".to_string(),
            ),
            return_doc: None,
            arg_doc: vec![MethodArg {
                name: "First arg".to_string(),
                doc: Some("Example doc".to_string()),
            }],
            ..Default::default()
        });

        self.method_list.insert(ptr.clone(), ping_pong);

        self.state.update_document(
            colabrodo_core::server_messages::DocumentUpdate {
                methods_list: Some(vec![ptr]),
                ..Default::default()
            },
        )
    }

    fn mut_state(&mut self) -> &ServerState {
        return &self.state;
    }

    fn state(&self) -> &ServerState {
        return &self.state;
    }

    fn invoke(
        &mut self,
        method: ComponentReference<MethodState>,
        context: colabrodo_core::server_state::InvokeObj,
        args: Vec<ciborium::value::Value>,
    ) -> MethodResult {
        let function = self.method_list.get(&method).unwrap();
        (function)(self, &context, args)
    }
}

#[tokio::main]
async fn main() {
    colabrodo_core::server::server_main::<PingPongServer>().await;
}
