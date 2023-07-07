use colabrodo_client::client::Value;
use colabrodo_client::client::*;
use colabrodo_client::client_state::*;
use colabrodo_client::components::*;
use colabrodo_client::delegate::*;
use colabrodo_common::nooid::*;
use std::sync::Weak;
use std::sync::{Arc, Mutex};

use crate::replay_state::ReplayerServerState;

pub struct Maker {
    server_link: Weak<Mutex<ReplayerServerState>>,
}

impl DelegateMaker for Maker {
    fn make_method(
        &mut self,
        id: MethodID,
        state: ClientMethodState,
        client: &mut ClientDelegateLists,
    ) -> Box<MethodDelegate> {
        Box::new(RMethodDelegate::new(id, state, client))
    }

    fn make_document(&mut self) -> Box<dyn DocumentDelegate + Send> {
        Box::new(ReplayDocDelegate::new(self.server_link.clone()))
    }
}

pub fn advance_client(ptr: &ReplayClientPtr, bytes: &[u8]) {
    let mut lock = ptr.lock().unwrap();
    handle_next(&mut lock, bytes).unwrap();
}

pub type ReplayClientPtr = Arc<Mutex<ClientState>>;

pub fn make_client_ptr(
    server: Weak<Mutex<ReplayerServerState>>,
) -> ReplayClientPtr {
    let channels = start_blank_stream();
    let maker = Maker {
        server_link: server,
    };
    Arc::new(Mutex::new(ClientState::new(&channels, maker)))
}

trait MyClientCrap {}

// =============================================================================

pub struct ReplayDocDelegate {
    server_link: Weak<Mutex<ReplayerServerState>>,
}

impl ReplayDocDelegate {
    fn new(server_link: Weak<Mutex<ReplayerServerState>>) -> Self {
        Self { server_link }
    }
}

impl DocumentDelegate for ReplayDocDelegate {}

// =============================================================================

pub struct RMethodDelegate {
    state: ClientMethodState,
}

impl Delegate for RMethodDelegate {
    type IDType = MethodID;
    type InitStateType = ClientMethodState;

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl RMethodDelegate {
    fn new(
        id: MethodID,
        state: ClientMethodState,
        client: &mut ClientDelegateLists,
    ) -> Self {
        //let c = client.document.unwrap();
        Self { state }
    }
}
