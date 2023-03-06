use closure::closure;
use colabrodo_server::{server::*, server_messages::*};


struct PingPongState {
    count: u64,
}

fn setup(state: ServerStatePtr) {
    let ping_pong_state = PingPongState { count: 0 };

    log::debug!("Initializing ping pong state");

    let mut state_lock = state.lock().unwrap();

    let function = closure!(
        move ping_pong_state, |_s: &mut ServerState, m : MethodSignalContent|{
            log::info!("Function called {}", ping_pong_state.count);
            Ok(Some(ciborium::value::Value::Array(m.args)))
        }
    );

    let ptr = state_lock.methods.new_owned_component(MethodState {
        name: "ping_pong".to_string(),
        doc: Some(
            "This method just replies with what you send it.".to_string(),
        ),
        return_doc: None,
        arg_doc: vec![MethodArg {
            name: "First arg".to_string(),
            doc: Some("Example doc".to_string()),
        }],
        state: MethodHandlerSlot::new_from_closure(function),
    });

    state_lock.update_document(ServerDocumentUpdate {
        methods_list: Some(vec![ptr]),
        ..Default::default()
    })
}

#[tokio::main]
async fn main() {
    println!("Connect clients to localhost:50000");
    let opts = ServerOptions::default();

    let state = ServerState::new();

    setup(state.clone());

    server_main(opts, state).await;
}
