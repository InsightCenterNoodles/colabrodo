use crate::replay_state::*;
use colabrodo_server::server::ciborium::value;
use colabrodo_server::server::*;
use colabrodo_server::server_messages::*;
use std::sync::{Arc, Mutex};

make_method_function!(advance_time,
    ReplayerServerState,
    "advance_replay",
    "Advance the replay by either a number of seconds, or to a marker",
    | to_time : Value : "Seconds or marker name" |,
    {
        match to_time {
            Value::Integer(x) => {
                app.advance_by_time( i128::from(x) as u64 , state);
            },
            Value::Text(x) => {
                app.advance_by_marker(x, state);
            }
            _ => ()
        }
        Ok(None)
    });

fn setup_server(state: ServerStatePtr) {
    let mut state_lock = state.lock().unwrap();

    let sig = state_lock.signals.new_owned_component(SignalState {
        name: "test_signal".to_string(),
        doc: Some("This is a test signal".to_string()),
        arg_doc: vec![MethodArg {
            name: "value".to_string(),
            doc: Some("Some value for testing".to_string()),
        }],
        ..Default::default()
    });

    let sig_copy = sig.clone();

    let method = state_lock.methods.new_owned_component(MethodState {
        name: "ping_pong".to_string(),
        doc: Some(
            "This method just replies with what you send it.".to_string(),
        ),
        return_doc: None,
        arg_doc: vec![MethodArg {
            name: "First arg".to_string(),
            doc: Some("Example doc".to_string()),
        }],
        state: MethodHandlerSlot::assign(move |m| {
            log::info!("Got a ping, sending pong...");
            m.state.lock().unwrap().issue_signal(
                &sig_copy,
                None,
                vec![value::Value::Text("Hi there".to_string())],
            );
            log::info!("Sending reply...");
            Ok(Some(ciborium::value::Value::Array(m.args)))
        }),
    });

    let shutdown_m = state_lock.methods.new_owned_component(MethodState {
        name: "shutdown".to_string(),
        doc: None,
        return_doc: None,
        arg_doc: vec![],
        state: MethodHandlerSlot::assign(|m| {
            log::info!("Shutdown method invoked");
            m.state.lock().unwrap().shutdown();
            Ok(None)
        }),
    });

    state_lock.update_document(ServerDocumentUpdate {
        methods_list: Some(vec![method, shutdown_m]),
        signals_list: Some(vec![sig]),
    })
}
