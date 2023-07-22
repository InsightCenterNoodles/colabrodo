use crate::replay_state::*;
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

/// Initialize a replayer server app
pub fn setup_server(state: ServerStatePtr, app: ReplayerStatePtr) {
    let mut state_lock = state.lock().unwrap();

    let method = state_lock
        .methods
        .new_owned_component(create_advance_time(app));

    state_lock.update_document(ServerDocumentUpdate {
        methods_list: Some(vec![method]),
        //signals_list: Some(vec![sig]),
        ..Default::default()
    })
}
