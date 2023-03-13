//! Methods and structs to create and launch NOODLES clients
//!
//! To create your own client:
//! - First create a [ClientChannels] structure. This sets up required channels for client async communication.
//! - Next, create a [ClientState].
//! - Then launch the client with [start_client].
//!
//! See the simple client example to see how to set up a client, and also how to
//! structure the code to launch tasks as soon as the client connects.

pub use crate::server_root_message::FromServer;
use crate::{components::*, server_root_message::*};
use ciborium::value::Value;
pub use colabrodo_common::client_communication::MethodInvokeMessage;
pub use colabrodo_common::server_communication::MessageMethodReply;
use colabrodo_common::{
    client_communication::{
        ClientMessageID, IntroductionMessage, InvokeIDType,
    },
    components::LightState,
    nooid::*,
    server_communication::MethodException,
};
pub use colabrodo_common::{components::UpdatableWith, nooid::NooID};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, info};
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tokio::{net::TcpStream, sync::oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio_tungstenite::{tungstenite, MaybeTlsStream};

// =============================================================================

/// A built in struct that conforms to the [ComponentList] trait.
#[derive(Debug)]
pub struct BasicComponentList<IDType, State>
where
    IDType: Eq + Hash + Copy,
    State: NamedComponent,
{
    name_map: HashMap<String, IDType>,
    components: HashMap<IDType, State>,
}

impl<IDType, State> BasicComponentList<IDType, State>
where
    IDType: Eq + Hash + Copy,
    State: NamedComponent,
{
    /// Find the name of a component by an ID
    pub fn find_name(&self, id: &IDType) -> Option<&String> {
        self.find(id)?.name()
    }

    /// Get a read-only copy of the ID -> State mapping
    pub fn component_map(&self) -> &HashMap<IDType, State> {
        &self.components
    }

    fn on_create(&mut self, id: IDType, state: State) {
        if let Some(n) = state.name() {
            self.name_map.insert(n.clone(), id);
        }

        self.components.insert(id, state);
    }

    fn on_delete(&mut self, id: IDType) {
        if let Some(n) = self.find_name(&id) {
            let n = n.clone();
            self.name_map.remove(&n);
        }

        self.components.remove(&id);
    }

    /// Find a component state by ID
    pub fn find(&self, id: &IDType) -> Option<&State> {
        self.components.get(id)
    }

    /// Find a component ID by a name
    pub fn get_id_by_name(&self, name: &str) -> Option<IDType> {
        self.name_map.get(name).cloned()
    }

    /// Get a component state by a name
    pub fn get_state_by_name(&self, name: &str) -> Option<&State> {
        self.find(self.name_map.get(name)?)
    }

    fn clear(&mut self) {
        self.name_map.clear();
        self.components.clear();
    }
}

impl<IDType, State> Default for BasicComponentList<IDType, State>
where
    IDType: Eq + Hash + Copy,
    State: NamedComponent,
{
    fn default() -> Self {
        Self {
            name_map: HashMap::new(),
            components: HashMap::new(),
        }
    }
}

impl<IDType, State> BasicComponentList<IDType, State>
where
    IDType: Eq + Hash + Copy,
    State: UpdatableWith + NamedComponent,
{
    fn on_update(&mut self, id: IDType, update: State::Substate) {
        if let Some(item) = self.components.get_mut(&id) {
            item.update(update);
        }
    }
}

// =============================================================================

/// Signals will be delivered along this channel type.
pub type SignalRecv = tokio::sync::broadcast::Receiver<Vec<Value>>;
type SignalSource = tokio::sync::broadcast::Sender<Vec<Value>>;
type SignalHash = HashMap<SignalID, SignalSource>;

fn fire_signal(id: SignalID, repo: &SignalHash, args: Vec<Value>) {
    if let Some(sender) = repo.get(&id) {
        sender.send(args).unwrap();
    }
}

// =============================================================================

/// A component list type that allows subscription to signals
#[derive(Debug, Default)]
pub struct SigModComponentList<IDType, State>
where
    IDType: Eq + Hash + Copy,
    State: UpdatableWith + NamedComponent + CommComponent,
{
    list: BasicComponentList<IDType, State>,
    signals: HashMap<IDType, SignalHash>,
}

impl<IDType, State> SigModComponentList<IDType, State>
where
    IDType: Eq + Hash + Copy,
    State: UpdatableWith + NamedComponent + CommComponent,
{
    /// Find the name of a component by an ID
    pub fn find_name(&self, id: &IDType) -> Option<&String> {
        self.list.find_name(id)
    }

    /// Get a read-only copy of the ID -> State mapping
    pub fn component_map(&self) -> &HashMap<IDType, State> {
        self.list.component_map()
    }

    fn on_create(&mut self, id: IDType, state: State) {
        self.list.on_create(id, state)
    }

    fn on_update(&mut self, id: IDType, update: State::Substate) {
        self.list.on_update(id, update)
    }

    fn on_delete(&mut self, id: IDType) {
        self.signals.remove(&id);
        self.list.on_delete(id)
    }

    /// Find a component state by ID
    pub fn find(&self, id: &IDType) -> Option<&State> {
        self.list.find(id)
    }

    /// Find a component ID by a name
    pub fn get_id_by_name(&self, name: &str) -> Option<IDType> {
        self.list.get_id_by_name(name)
    }

    /// Get a component state by a name
    pub fn get_state_by_name(&self, name: &str) -> Option<&State> {
        self.list.get_state_by_name(name)
    }

    fn clear(&mut self) {
        self.list.clear()
    }

    fn can_sub(&self, id: IDType, signal: SignalID) -> bool {
        if let Some(state) = self.list.components.get(&id) {
            if let Some(list) = state.signal_list() {
                return list.iter().any(|&f| f == signal);
            }
        }
        false
    }

    fn fire_signal(&self, sig_id: SignalID, id: IDType, args: Vec<Value>) {
        if let Some(hash) = self.signals.get(&id) {
            fire_signal(sig_id, hash, args)
        }
    }

    /// Subscribe to a signal emitted by a component
    ///
    /// Takes an ID to an existing component, and an ID to a signal that is attached to that component.
    ///
    /// # Returns
    ///
    /// Returns [None] if the subscription fails (non-existing component, etc). Otherwise returns a channel to be subscribed to.
    pub fn subscribe_signal(
        &mut self,
        id: IDType,
        signal: SignalID,
    ) -> Option<SignalRecv> {
        if self.can_sub(id, signal) {
            return Some(
                self.signals
                    .entry(id)
                    .or_default()
                    .entry(signal)
                    .or_insert_with(|| tokio::sync::broadcast::channel(16).0)
                    .subscribe(),
            );
        }
        None
    }

    /// Remove a subscription to a signal on a component.
    pub fn unsubscribe_signal(&mut self, id: IDType, signal: SignalID) {
        if let Some(h) = self.signals.get_mut(&id) {
            h.remove(&signal);
        }
    }
}

// =============================================================================

/// A callback that is invoked for every message from the server
pub type Callback = dyn Fn(&mut ClientState, &FromServer) + Send + Sync;

/// Current NOODLES client state.
///
/// Keeps up-to-date state on all components, signal subscriptions, method invocation, etc.
///
/// See module-level documentation to see how to use this struct.
pub struct ClientState {
    sender: tokio::sync::mpsc::Sender<OutgoingMessage>,

    callback: Arc<Callback>,

    ready_tx: Option<oneshot::Sender<()>>,
    ready_rx: Option<oneshot::Receiver<()>>,

    pub method_list: BasicComponentList<MethodID, ClientMethodState>,
    pub signal_list: BasicComponentList<SignalID, ClientSignalState>,

    pub buffer_list: BasicComponentList<BufferID, BufferState>,
    pub buffer_view_list:
        BasicComponentList<BufferViewID, ClientBufferViewState>,

    pub sampler_list: BasicComponentList<SamplerID, SamplerState>,
    pub image_list: BasicComponentList<ImageID, ClientImageState>,
    pub texture_list: BasicComponentList<TextureID, ClientTextureState>,

    pub material_list: BasicComponentList<MaterialID, ClientMaterialState>,
    pub geometry_list: BasicComponentList<GeometryID, ClientGeometryState>,

    pub light_list: BasicComponentList<LightID, LightState>,

    pub table_list: SigModComponentList<TableID, ClientTableState>,
    pub plot_list: SigModComponentList<PlotID, ClientPlotState>,
    pub entity_list: SigModComponentList<EntityID, ClientEntityState>,

    pub document_communication: ClientDocumentUpdate,

    signal_subs: SignalHash,

    method_subs:
        HashMap<uuid::Uuid, tokio::sync::oneshot::Sender<MessageMethodReply>>,
}

impl Debug for ClientState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientState")
            .field("sender", &self.sender)
            .field("method_list", &self.method_list)
            .field("signal_list", &self.signal_list)
            .field("buffer_list", &self.buffer_list)
            .field("buffer_view_list", &self.buffer_view_list)
            .field("sampler_list", &self.sampler_list)
            .field("image_list", &self.image_list)
            .field("texture_list", &self.texture_list)
            .field("material_list", &self.material_list)
            .field("geometry_list", &self.geometry_list)
            .field("light_list", &self.light_list)
            .field("table_list", &self.table_list)
            .field("plot_list", &self.plot_list)
            .field("entity_list", &self.entity_list)
            .field("document_communication", &self.document_communication)
            .field("signal_subs", &self.signal_subs)
            .field("method_subs", &self.method_subs)
            .finish()
    }
}

fn blank_callback(_: &mut ClientState, _: &FromServer) {}

impl ClientState {
    /// Create a new client state, using created channels.
    ///
    /// Note that the given [ClientChannels] struct should not be re-used.
    pub fn new(channels: &ClientChannels) -> Arc<Mutex<Self>> {
        Self::new_with_callback(channels, blank_callback)
    }

    /// Create a new client state, using previously created channels, and a callback.
    pub fn new_with_callback<C>(
        channels: &ClientChannels,
        cb: C,
    ) -> Arc<Mutex<Self>>
    where
        C: Fn(&mut ClientState, &FromServer) + Send + Sync + 'static,
    {
        let (ready_tx, ready_rx) = oneshot::channel();

        Arc::new(Mutex::new(Self {
            sender: channels.from_client_tx.clone(),
            callback: Arc::new(cb),
            ready_tx: Some(ready_tx),
            ready_rx: Some(ready_rx),
            method_list: Default::default(),
            signal_list: Default::default(),
            buffer_list: Default::default(),
            buffer_view_list: Default::default(),
            sampler_list: Default::default(),
            image_list: Default::default(),
            texture_list: Default::default(),
            material_list: Default::default(),
            geometry_list: Default::default(),
            light_list: Default::default(),
            table_list: Default::default(),
            plot_list: Default::default(),
            entity_list: Default::default(),
            document_communication: Default::default(),
            signal_subs: Default::default(),
            method_subs: Default::default(),
        }))
    }

    fn clear(&mut self) {
        self.method_list.clear();
        self.signal_list.clear();
        self.buffer_list.clear();
        self.buffer_view_list.clear();
        self.sampler_list.clear();
        self.image_list.clear();
        self.texture_list.clear();
        self.material_list.clear();
        self.geometry_list.clear();
        self.light_list.clear();
        self.table_list.clear();
        self.plot_list.clear();
        self.entity_list.clear();
        self.document_communication = Default::default();
        self.signal_subs.clear();
        self.method_subs.clear();
    }

    /// Update document method/signal handlers
    fn update_method_signals(&mut self) {
        if let Some(siglist) = &self.document_communication.signals_list {
            let siglist: Vec<_> = siglist
                .iter()
                .filter(|x| !self.signal_subs.contains_key(&x))
                .collect();

            for sid in siglist {
                self.signal_subs.remove(sid);
            }
        }
    }

    /// Subscribe to a signal on the document.
    ///
    /// # Returns
    ///
    /// Returns [None] if the subscription fails (non-existing component, etc). Otherwise returns a channel to be subscribed to.
    pub fn subscribe_signal(&mut self, signal: SignalID) -> Option<SignalRecv> {
        if let Some(list) = &mut self.document_communication.signals_list {
            log::debug!("Searching for signal {signal:?} in {list:?}");
            if list.iter().any(|&f| f == signal) {
                return Some(
                    self.signal_subs
                        .entry(signal)
                        .or_insert_with(|| {
                            tokio::sync::broadcast::channel(16).0
                        })
                        .subscribe(),
                );
            }
        }
        log::debug!("Unable to find requested signal: {signal:?}");
        None
    }

    /// Unsubscribe to a document signal
    pub fn unsubscribe_signal(&mut self, signal: SignalID) {
        self.signal_subs.remove(&signal);
    }
}

/// Async wait for the client to have connected and received the full document state.
pub async fn wait_for_start(state: Arc<Mutex<ClientState>>) {
    let rx = state.lock().unwrap().ready_rx.take();

    if let Some(rx) = rx {
        rx.await.unwrap();
    }
}

/// Async issue a shutdown for the client and wait for all client machinery to stop.
pub async fn shutdown(state: Arc<Mutex<ClientState>>) {
    let sender = state.lock().unwrap().sender.clone();

    sender.send(OutgoingMessage::Close).await.unwrap();
}

/// Context for a method invocation
pub enum InvokeContext {
    Document,
    Entity(EntityID),
    Table(TableID),
    Plot(PlotID),
}

/// Invoke a method on a component.
///
/// # Parameters
/// - state: The client state to invoke on
/// - method_id: The method id to invoke
/// - context: Component to invoke the method on
/// - args: A list of CBOR arguments to send to the server
///
/// # Return
/// If the method fails, returns an exception. If successful, the result of the invocation. Note that the method might return nothing (a void), thus it returns an optional [Value].
pub async fn invoke_method(
    state: Arc<Mutex<ClientState>>,
    method_id: MethodID,
    context: InvokeContext,
    args: Vec<Value>,
) -> Result<Option<Value>, MethodException> {
    let invoke_id = uuid::Uuid::new_v4();

    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    let content = OutgoingMessage::MethodInvoke(MethodInvokeMessage {
        method: method_id,
        context: match context {
            InvokeContext::Document => None,
            InvokeContext::Entity(id) => Some(InvokeIDType::Entity(id)),
            InvokeContext::Table(id) => Some(InvokeIDType::Table(id)),
            InvokeContext::Plot(id) => Some(InvokeIDType::Plot(id)),
        },
        invoke_id: Some(invoke_id.to_string()),
        args,
    });

    {
        // careful not to hold a lock across an await...

        let sender = {
            let mut lock = state.lock().unwrap();

            lock.method_subs.insert(invoke_id, res_tx);

            lock.sender.clone()
        };

        sender.send(content).await.map_err(|_| {
            MethodException::internal_error(Some(
                "Unable to send method invocation.",
            ))
        })?;
    }

    let result = res_rx.await.map_err(|_| {
        MethodException::internal_error(Some(
            "Invocation was dropped while in progress.",
        ))
    })?;

    if let Some(exp) = result.method_exception {
        return Err(exp);
    }

    Ok(result.result)
}

// =============================================================================

#[derive(Error, Debug)]
pub enum UserClientNext {
    #[error("Decode error")]
    DecodeError(String),
    #[error("Method error")]
    MethodError,
}

/// Execute the next message from a server on your client state
pub fn handle_next(
    state: Arc<Mutex<ClientState>>,
    message: Vec<u8>,
) -> Result<(), UserClientNext> {
    if log::log_enabled!(log::Level::Debug) {
        let v: ciborium::value::Value =
            ciborium::de::from_reader(message.as_slice()).unwrap();
        debug!("Content: {v:?}");
    }

    let root: ServerRootMessage =
        ciborium::de::from_reader(message.as_slice()).unwrap();

    debug!("Got {} messages", root.list.len());

    for msg in root.list {
        let mut state = state.lock().unwrap();
        let cb = state.callback.clone();
        (cb)(&mut state, &msg);
        handle_next_message(&mut state, msg)?;
    }

    Ok(())
}

fn handle_next_message(
    state: &mut ClientState,
    m: FromServer,
) -> Result<(), UserClientNext> {
    debug!("Handling next message...");

    match m {
        FromServer::Method(m) => match m {
            ModMethod::Create(x) => {
                state.method_list.on_create(x.id, x.content);
            }
            ModMethod::Delete(x) => state.method_list.on_delete(x.id),
        },
        //
        FromServer::Signal(s) => match s {
            ModSignal::Create(x) => {
                state.signal_list.on_create(x.id, x.content)
            }
            ModSignal::Delete(x) => state.signal_list.on_delete(x.id),
        },
        //
        FromServer::Entity(x) => match x {
            ModEntity::Create(x) => {
                state.entity_list.on_create(x.id, x.content)
            }
            ModEntity::Update(x) => {
                state.entity_list.on_update(x.id, x.content)
            }
            ModEntity::Delete(x) => state.entity_list.on_delete(x.id),
        },
        //
        FromServer::Plot(x) => match x {
            ModPlot::Create(x) => state.plot_list.on_create(x.id, x.content),
            ModPlot::Update(x) => state.plot_list.on_update(x.id, x.content),
            ModPlot::Delete(x) => state.plot_list.on_delete(x.id),
        },
        //
        FromServer::Buffer(s) => match s {
            ModBuffer::Create(x) => {
                state.buffer_list.on_create(x.id, x.content)
            }
            ModBuffer::Delete(x) => state.buffer_list.on_delete(x.id),
        },
        //
        FromServer::BufferView(s) => match s {
            ModBufferView::Create(x) => {
                state.buffer_view_list.on_create(x.id, x.content)
            }
            ModBufferView::Delete(x) => state.buffer_view_list.on_delete(x.id),
        },
        //
        FromServer::Material(s) => match s {
            ModMaterial::Create(x) => {
                state.material_list.on_create(x.id, x.content)
            }
            ModMaterial::Update(x) => {
                state.material_list.on_update(x.id, x.content)
            }
            ModMaterial::Delete(x) => state.material_list.on_delete(x.id),
        },
        //
        FromServer::Image(s) => match s {
            ModImage::Create(x) => state.image_list.on_create(x.id, x.content),
            ModImage::Delete(x) => state.image_list.on_delete(x.id),
        },
        //
        FromServer::Texture(s) => match s {
            ModTexture::Create(x) => {
                state.texture_list.on_create(x.id, x.content)
            }
            ModTexture::Delete(x) => state.texture_list.on_delete(x.id),
        },
        //
        FromServer::Sampler(s) => match s {
            ModSampler::Create(x) => {
                state.sampler_list.on_create(x.id, x.content)
            }
            ModSampler::Delete(x) => state.sampler_list.on_delete(x.id),
        },
        //
        FromServer::Light(s) => match s {
            ModLight::Create(x) => state.light_list.on_create(x.id, x.content),
            ModLight::Update(x) => state.light_list.on_update(x.id, x.content),
            ModLight::Delete(x) => state.light_list.on_delete(x.id),
        },
        //
        FromServer::Geometry(s) => match s {
            ModGeometry::Create(x) => {
                state.geometry_list.on_create(x.id, x.content)
            }
            ModGeometry::Delete(x) => state.geometry_list.on_delete(x.id),
        },
        //
        FromServer::Table(s) => match s {
            ModTable::Create(x) => state.table_list.on_create(x.id, x.content),
            ModTable::Update(x) => state.table_list.on_update(x.id, x.content),
            ModTable::Delete(x) => state.table_list.on_delete(x.id),
        },
        //
        FromServer::MsgDocumentUpdate(x) => {
            state.document_communication = x;
            state.update_method_signals();
        }
        FromServer::MsgDocumentReset(_) => {
            state.clear();
        }
        //
        FromServer::MsgSignalInvoke(x) => {
            let sig_id = SignalID(x.id);
            if let Some(id) = x.context {
                if let Some(entity) = id.entity {
                    state.entity_list.fire_signal(
                        sig_id,
                        entity,
                        x.signal_data,
                    );
                } else if let Some(plot) = id.plot {
                    state.plot_list.fire_signal(sig_id, plot, x.signal_data);
                } else if let Some(table) = id.table {
                    state.table_list.fire_signal(sig_id, table, x.signal_data);
                }
            } else {
                fire_signal(sig_id, &state.signal_subs, x.signal_data);
            }

            // log::debug!("Signal from server");
            // state
            //     .signal_list
            //     .find(&x.id)
            //     .and_then(|f| f.state.send(x.signal_data));
        }
        FromServer::MsgMethodReply(x) => {
            let invoke_id: uuid::Uuid = x
                .invoke_id
                .parse()
                .map_err(|_| UserClientNext::MethodError)?;

            if let Some(dest) = state.method_subs.remove(&invoke_id) {
                dest.send(x).map_err(|_| UserClientNext::MethodError)?;
            }
        }
        FromServer::MsgDocumentInitialized(_) => {
            //send read
            if let Some(tx) = state.ready_tx.take() {
                tx.send(()).unwrap()
            }
        }
    }

    debug!("Handling next message...Done");

    Ok(())
}

// =============================================================================

#[derive(Error, Debug)]
pub enum UserClientError {
    #[error("Invalid Host")]
    InvalidHost(String),

    #[error("Connection Error")]
    ConnectionError(tungstenite::Error),
}

/// Enumeration describing incoming messages to your client.
#[derive(Debug, Clone)]
pub enum IncomingMessage {
    /// Message from the server
    NetworkMessage(Vec<u8>),
    /// Socket shutdown from the server
    Closed,
}

/// Enumeration describing outgoing messages
#[derive(Debug)]
pub enum OutgoingMessage {
    /// Instruct client machinery to shut down
    Close,
    /// Invoke a message on the server
    MethodInvoke(MethodInvokeMessage),
}

// =============================================================================

/// Contains channels for incoming and outgoing messages for the client.
///
/// See the module level documentation to see how to use this class.
pub struct ClientChannels {
    to_client_tx: tokio::sync::mpsc::Sender<IncomingMessage>,
    to_client_rx: tokio::sync::mpsc::Receiver<IncomingMessage>,

    from_client_tx: tokio::sync::mpsc::Sender<OutgoingMessage>,
    from_client_rx: tokio::sync::mpsc::Receiver<OutgoingMessage>,
}

impl Default for ClientChannels {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientChannels {
    pub fn new() -> Self {
        let (to_client_tx, to_client_rx) = tokio::sync::mpsc::channel(16);
        let (from_client_tx, from_client_rx) = tokio::sync::mpsc::channel(16);

        Self {
            to_client_tx,
            to_client_rx,
            from_client_tx,
            from_client_rx,
        }
    }
}

/// Start running the client machinery.
///
/// # Parameters
/// - url: The host to connect to
/// - name: The name of the client to use during introduction to the server
/// - state: The client to use
/// - channels: Input/output channels
pub async fn start_client(
    url: String,
    name: String,
    state: Arc<Mutex<ClientState>>,
    channels: ClientChannels,
) -> Result<(), UserClientError> {
    // create streams to stop machinery
    let (stop_tx, mut stop_rx) = tokio::sync::broadcast::channel::<u8>(1);

    info!("Connecting to {url}...");

    // connect to a server...
    let conn_result = connect_async(&url)
        .await
        .map_err(UserClientError::ConnectionError)?;

    info!("Connected to {url}...");

    // Stream for messages going to client state
    let (to_client_thread_tx, to_client_thread_rx) =
        (channels.to_client_tx, channels.to_client_rx);
    // Stream for messages from client state
    let (_, from_client_thread_rx) =
        (channels.from_client_tx, channels.from_client_rx);

    let worker_stop_tx = stop_tx.clone();

    tokio::spawn(client_worker_thread(
        to_client_thread_rx,
        //from_client_thread_tx.clone(),
        worker_stop_tx,
        state.clone(),
    ));

    let (ws_stream, _) = conn_result;

    // Split out our server connection
    let (mut socket_tx, mut socket_rx) = ws_stream.split();

    // Send the initial introduction message
    {
        let content = (
            IntroductionMessage::message_id(),
            IntroductionMessage { client_name: name },
        );

        let mut buffer = Vec::<u8>::new();

        ciborium::ser::into_writer(&content, &mut buffer).unwrap();

        socket_tx.send(Message::Binary(buffer)).await.unwrap();
    }

    // spawn task that forwards messages from the client to the socket
    tokio::spawn(forward_task(
        from_client_thread_rx,
        socket_tx,
        stop_tx.clone(),
        stop_tx.subscribe(),
    ));

    debug!("Tasks launched");

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            msg = socket_rx.next() => {

                match msg.unwrap() {
                    Ok(x) => {
                        to_client_thread_tx
                        .send(IncomingMessage::NetworkMessage(x.into_data()))
                        .await
                        .unwrap();
                    },
                    Err(_) => {
                        to_client_thread_tx.send(IncomingMessage::Closed).await.unwrap();
                        break;
                    },
                }
            }
        }
    }

    debug!("Loop closed. Client system done.");

    Ok(())
}

/// Task that sends handles from the client.
async fn forward_task(
    mut input: tokio::sync::mpsc::Receiver<OutgoingMessage>,
    mut socket_out: SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        Message,
    >,
    stopper_tx: tokio::sync::broadcast::Sender<u8>,
    mut stopper: tokio::sync::broadcast::Receiver<u8>,
) {
    debug!("Starting thread forwarding task");

    loop {
        tokio::select! {
            _ = stopper.recv() => break,
            Some(msg) = input.recv() => {
                let mut buffer = Vec::<u8>::new();

                match msg {
                    // If the client wants to close things down...
                    OutgoingMessage::Close => {
                        // kill the output stream
                        socket_out.close().await.unwrap();

                        // tell everyone to stop
                        stopper_tx.send(1).unwrap();
                        break;
                    }
                    OutgoingMessage::MethodInvoke(x) => {
                        let tuple = (MethodInvokeMessage::message_id(), x);
                        ciborium::ser::into_writer(&tuple, &mut buffer).unwrap()
                    }
                }

                socket_out
                    .send(tokio_tungstenite::tungstenite::Message::Binary(buffer))
                    .await
                    .unwrap();
            }
        }
    }
    debug!("Ending thread forwarding task");
}

/// Run the client state in it's own thread
async fn client_worker_thread(
    mut input: tokio::sync::mpsc::Receiver<IncomingMessage>,
    //output: tokio::sync::mpsc::Sender<OutgoingMessage>,
    stopper: tokio::sync::broadcast::Sender<u8>,
    state: Arc<Mutex<ClientState>>,
) {
    debug!("Starting client worker thread");

    while let Some(x) = input.recv().await {
        match x {
            IncomingMessage::NetworkMessage(root) => {
                handle_next(state.clone(), root).unwrap();
            }
            IncomingMessage::Closed => {
                break;
            }
        }
    }
    debug!("Ending client worker thread");

    stopper.send(1).unwrap();

    {
        state.lock().unwrap().clear();
    }
}

//pub fn make_invoke_message
