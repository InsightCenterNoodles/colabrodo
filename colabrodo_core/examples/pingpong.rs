use colabrodo_core::server::{AsyncServer, DefaultCommand, ServerOptions};
use colabrodo_core::server_messages::{ComponentReference, MethodArg};
use colabrodo_core::{
    server_messages::MethodState,
    server_state::{InvokeObj, MethodResult, ServerState, UserServerState},
};
use log;
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

/// This function will handle a method invocation, parse arguments, and compute a reply.
fn ping_pong(
    _state: &mut PingPongServer,
    _context: &InvokeObj,
    args: Vec<ciborium::value::Value>,
) -> MethodResult {
    Ok(ciborium::value::Value::Array(args))
}

/// All server states should use this trait...
impl UserServerState for PingPongServer {
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

/// And servers that use the provided tokio infrastructure should impl this trait, too...
impl AsyncServer for PingPongServer {
    type CommandType = DefaultCommand;

    fn new(tx: colabrodo_core::server_state::CallbackPtr) -> Self {
        Self {
            state: ServerState::new(tx.clone()),
            method_list: Default::default(),
        }
    }

    fn initialize_state(&mut self) {
        log::debug!("Initializing ping pong state");
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

    // If we had some kind of out-of-band messaging to the server, it would be handled here
    fn handle_command(&mut self, _: Self::CommandType) {
        // pass
    }
}

#[tokio::main]
async fn main() {
    println!("Connect clients to localhost:50000");
    let opts = ServerOptions::default();
    colabrodo_core::server::server_main::<PingPongServer>(opts).await;
}
