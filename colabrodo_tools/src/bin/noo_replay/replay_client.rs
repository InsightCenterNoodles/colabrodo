use colabrodo_client::client::*;
use colabrodo_client::client_state::*;
use colabrodo_server::server_state::ServerState;
use std::sync::{Arc, Mutex, Weak};

pub fn advance_client(ptr: &ReplayClientPtr, bytes: &[u8]) {
    let mut lock = ptr.lock().unwrap();
    handle_next(&mut lock, bytes).unwrap();
}

pub type ReplayClientPtr = Arc<Mutex<ClientState>>;

pub fn make_client_ptr(server: Weak<Mutex<ServerState>>) -> ReplayClientPtr {
    let channels = start_blank_stream();
    let maker = replay::ReplayDelegateMaker {
        server_link: server,
    };
    Arc::new(Mutex::new(ClientState::new(&channels, maker)))
}
