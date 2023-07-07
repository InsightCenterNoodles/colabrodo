use colabrodo_client::client::Value;
use colabrodo_client::client::*;
use colabrodo_client::client_state::*;
use colabrodo_client::components::*;
use colabrodo_client::delegate::*;
use colabrodo_common::nooid::*;
use std::sync::Weak;
use std::sync::{Arc, Mutex};

use crate::replay_state::ReplayerServerState;

pub struct Maker {}

impl DelegateMaker for Maker {
    // type MethodDelegate = RMethodDelegate;
    // type SignalDelegate = DefaultSignalDelegate;
    // type BufferDelegate = DefaultBufferDelegate;
    // type BufferViewDelegate = DefaultBufferViewDelegate;
    // type SamplerDelegate = DefaultSamplerDelegate;
    // type ImageDelegate = DefaultImageDelegate;
    // type TextureDelegate = DefaultTextureDelegate;
    // type MaterialDelegate = DefaultMaterialDelegate;
    // type GeometryDelegate = DefaultGeometryDelegate;
    // type LightDelegate = DefaultLightDelegate;
    // type TableDelegate = DefaultTableDelegate;
    // type PlotDelegate = DefaultPlotDelegate;
    // type EntityDelegate = DefaultEntityDelegate;
    // type DocumentDelegate = ReplayDocDelegate;
}

impl ReplayClient {
    pub fn new() -> ReplayClientPtr {
        let channels = start_blank_stream();
        Arc::new(Mutex::new(ClientState::new(&channels)))
    }
}

pub fn advance_client(ptr: &ReplayClientPtr, bytes: &[u8]) {
    let mut lock = ptr.lock().unwrap();
    handle_next(&mut lock, bytes).unwrap();
}

pub type ReplayClientPtr = Arc<Mutex<ClientState<ReplayClient>>>;

trait MyClientCrap {}

// =============================================================================

pub struct ReplayDocDelegate {
    server_link: Weak<Mutex<ReplayerServerState>>,
}

impl Default for ReplayDocDelegate {
    fn default() -> Self {
        Self {}
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

    fn on_new<Provider: DelegateProvider + MyClientCrap>(
        id: Self::IDType,
        state: Self::InitStateType,
        client: &mut ClientState<Provider>,
    ) -> Self {
        let c = client.document.unwrap();
        Self { state }
    }
}
