pub use crate::table::{ManagedTable, ResolvedTableIDs};
use crate::{
    components::*,
    server_root_message::{FromServer, ServerRootMessage},
};
pub use colabrodo_common::client_communication::ClientInvokeMessage;
pub use colabrodo_common::server_communication::MessageMethodReply;
use colabrodo_common::{
    client_communication::{ClientIntroductionMessage, ClientMessageID},
    components::LightState,
};
pub use colabrodo_common::{components::UpdatableWith, nooid::NooID};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, info};
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio_tungstenite::{tungstenite, MaybeTlsStream};

/// A trait to interact with lists of immutable components
pub trait ComponentList<State> {
    fn on_create(&mut self, id: NooID, state: State);
    fn on_delete(&mut self, id: NooID);
    fn find(&self, id: &NooID) -> Option<&State>;

    fn get_id_by_name(&self, name: &str) -> Option<&NooID>;
    fn get_state_by_name(&self, name: &str) -> Option<&State> {
        self.find(self.get_id_by_name(name)?)
    }
}

/// A trait to describe how to interact with mutable components
pub trait UpdatableComponentList<State>: ComponentList<State>
where
    State: UpdatableWith,
{
    fn on_update(&mut self, id: NooID, update: State::Substate);
}

// =============================================================================

/// A built in struct that conforms to the [ComponentList] trait.
#[derive(Debug)]
pub struct BasicComponentList<State>
where
    State: NamedComponent,
{
    name_map: HashMap<String, NooID>,
    components: HashMap<NooID, State>,
}

impl<State> BasicComponentList<State>
where
    State: NamedComponent,
{
    pub fn find_name(&self, id: &NooID) -> Option<&String> {
        self.find(id)?.name()
    }

    pub fn component_map(&self) -> &HashMap<NooID, State> {
        &self.components
    }
}

impl<State> Default for BasicComponentList<State>
where
    State: NamedComponent,
{
    fn default() -> Self {
        Self {
            name_map: HashMap::new(),
            components: HashMap::new(),
        }
    }
}

impl<State> ComponentList<State> for BasicComponentList<State>
where
    State: NamedComponent,
{
    fn on_create(&mut self, id: NooID, state: State) {
        if let Some(n) = state.name() {
            self.name_map.insert(n.clone(), id);
        }

        self.components.insert(id, state);
    }

    fn on_delete(&mut self, id: NooID) {
        if let Some(n) = self.find_name(&id) {
            let n = n.clone();
            self.name_map.remove(&n);
        }

        self.components.remove(&id);
    }

    fn find(&self, id: &NooID) -> Option<&State> {
        self.components.get(id)
    }

    fn get_id_by_name(&self, name: &str) -> Option<&NooID> {
        self.name_map.get(name)
    }

    fn get_state_by_name(&self, name: &str) -> Option<&State> {
        self.find(self.name_map.get(name)?)
    }
}

impl<State> UpdatableComponentList<State> for BasicComponentList<State>
where
    State: UpdatableWith + NamedComponent,
{
    fn on_update(&mut self, id: NooID, update: State::Substate) {
        if let Some(item) = self.components.get_mut(&id) {
            item.update(update);
        }
    }
}

// =============================================================================

/// A trait to describe how the client library interacts with client state.
pub trait UserClientState: Debug {
    /// The type holding methods
    type MethodL: ComponentList<ClientMethodState>;
    /// The type holding signals
    type SignalL: ComponentList<SignalState>;

    /// The type holding buffers
    type BufferL: ComponentList<BufferState>;
    /// The type holding buffer lists    
    type BufferViewL: ComponentList<ClientBufferViewState>;

    /// The type holding samplers
    type SamplerL: ComponentList<SamplerState>;
    /// The type holding images
    type ImageL: ComponentList<ClientImageState>;
    /// The type holding texture
    type TextureL: ComponentList<ClientTextureState>;

    /// The type holding materials
    type MaterialL: UpdatableComponentList<ClientMaterialState>;
    /// The type holding geometry
    type GeometryL: ComponentList<ClientGeometryState>;

    /// The type holding lights
    type LightL: UpdatableComponentList<LightState>;

    /// The type holding tables
    type TableL: UpdatableComponentList<ClientTableState>;
    /// The type holding plots
    type PlotL: UpdatableComponentList<ClientPlotState>;

    /// The type holding entities
    type EntityL: UpdatableComponentList<ClientEntityState>;

    /// The type for commands passed to the client
    type CommandType;
    /// The type holding arguments for your client
    type ArgumentType;

    fn new(
        a: Self::ArgumentType,
        to_server: tokio::sync::mpsc::Sender<OutgoingMessage>,
    ) -> Self;

    fn method_list(&mut self) -> &mut Self::MethodL;
    fn signal_list(&mut self) -> &mut Self::SignalL;

    fn buffer_list(&mut self) -> &mut Self::BufferL;
    fn buffer_view_list(&mut self) -> &mut Self::BufferViewL;

    fn sampler_list(&mut self) -> &mut Self::SamplerL;
    fn image_list(&mut self) -> &mut Self::ImageL;
    fn texture_list(&mut self) -> &mut Self::TextureL;

    fn material_list(&mut self) -> &mut Self::MaterialL;
    fn geometry_list(&mut self) -> &mut Self::GeometryL;

    fn light_list(&mut self) -> &mut Self::LightL;

    fn table_list(&mut self) -> &mut Self::TableL;
    fn plot_list(&mut self) -> &mut Self::PlotL;

    fn entity_list(&mut self) -> &mut Self::EntityL;

    fn document_update(&mut self, update: ClientDocumentUpdate);

    fn document_reset(&mut self) {}

    #[allow(unused_variables)]
    fn on_signal_invoke(&mut self, signal: ClientMessageSignalInvoke) {}
    #[allow(unused_variables)]
    fn on_method_reply(&mut self, method_reply: MessageMethodReply) {}

    fn on_document_ready(&mut self) {}

    fn on_command(&mut self, _c: Self::CommandType) {}
}

#[derive(Error, Debug)]
pub enum UserClientNext {
    #[error("Decode error")]
    DecodeError(String),
}

/// Execute the next message from a server on your client state
pub fn handle_next<U: UserClientState>(
    state: &mut U,
    message: &[u8],
) -> Result<(), UserClientNext> {
    debug!("Handling next message array:");

    if log::log_enabled!(log::Level::Debug) {
        let v: ciborium::value::Value =
            ciborium::de::from_reader(message).unwrap();
        debug!("Content: {v:?}");
    }

    let root: ServerRootMessage = ciborium::de::from_reader(message)
        .map_err(|x| UserClientNext::DecodeError(x.to_string()))?;

    debug!("Got {} messages", root.list.len());

    for msg in root.list {
        handle_next_message(state, msg)?;
    }

    Ok(())
}

fn handle_next_message<U: UserClientState>(
    state: &mut U,
    m: FromServer,
) -> Result<(), UserClientNext> {
    debug!("Handling next message...");
    match m {
        FromServer::MsgMethodCreate(x) => {
            state.method_list().on_create(x.id, x.content);
        }
        FromServer::MsgMethodDelete(x) => state.method_list().on_delete(x.id),
        //
        FromServer::MsgSignalCreate(x) => {
            state.signal_list().on_create(x.id, x.content)
        }
        FromServer::MsgSignalDelete(x) => state.signal_list().on_delete(x.id),
        //
        FromServer::MsgEntityCreate(x) => {
            state.entity_list().on_create(x.id, x.content)
        }
        FromServer::MsgEntityUpdate(x) => {
            state.entity_list().on_update(x.id, x.content)
        }
        FromServer::MsgEntityDelete(x) => state.entity_list().on_delete(x.id),
        //
        FromServer::MsgPlotCreate(x) => {
            state.plot_list().on_create(x.id, x.content)
        }
        FromServer::MsgPlotUpdate(x) => {
            state.plot_list().on_update(x.id, x.content)
        }
        FromServer::MsgPlotDelete(x) => state.plot_list().on_delete(x.id),
        //
        FromServer::MsgBufferCreate(x) => {
            state.buffer_list().on_create(x.id, x.content)
        }
        FromServer::MsgBufferDelete(x) => state.buffer_list().on_delete(x.id),
        //
        FromServer::MsgBufferViewCreate(x) => {
            state.buffer_view_list().on_create(x.id, x.content)
        }
        FromServer::MsgBufferViewDelete(x) => {
            state.buffer_view_list().on_delete(x.id)
        }
        //
        FromServer::MsgMaterialCreate(x) => {
            state.material_list().on_create(x.id, x.content)
        }
        FromServer::MsgMaterialUpdate(x) => {
            state.material_list().on_update(x.id, x.content)
        }
        FromServer::MsgMaterialDelete(x) => {
            state.material_list().on_delete(x.id)
        }
        //
        FromServer::MsgImageCreate(x) => {
            state.image_list().on_create(x.id, x.content)
        }
        FromServer::MsgImageDelete(x) => state.image_list().on_delete(x.id),
        //
        FromServer::MsgTextureCreate(x) => {
            state.texture_list().on_create(x.id, x.content)
        }
        FromServer::MsgTextureDelete(x) => state.texture_list().on_delete(x.id),
        //
        FromServer::MsgSamplerCreate(x) => {
            state.sampler_list().on_create(x.id, x.content)
        }
        FromServer::MsgSamplerDelete(x) => state.sampler_list().on_delete(x.id),
        //
        FromServer::MsgLightCreate(x) => {
            state.light_list().on_create(x.id, x.content)
        }
        FromServer::MsgLightUpdate(x) => {
            state.light_list().on_update(x.id, x.content)
        }
        FromServer::MsgLightDelete(x) => state.light_list().on_delete(x.id),
        //
        FromServer::MsgGeometryCreate(x) => {
            state.geometry_list().on_create(x.id, x.content)
        }
        FromServer::MsgGeometryDelete(x) => {
            state.geometry_list().on_delete(x.id)
        }
        //
        FromServer::MsgTableCreate(x) => {
            state.table_list().on_create(x.id, x.content)
        }
        FromServer::MsgTableUpdate(x) => {
            state.table_list().on_update(x.id, x.content)
        }
        FromServer::MsgTableDelete(x) => state.table_list().on_delete(x.id),
        //
        FromServer::MsgDocumentUpdate(x) => state.document_update(x),
        FromServer::MsgDocumentReset(_) => state.document_reset(),
        //
        FromServer::MsgSignalInvoke(x) => {
            log::debug!("Signal from server");
            state.on_signal_invoke(x)
        }
        FromServer::MsgMethodReply(x) => state.on_method_reply(x),
        FromServer::MsgDocumentInitialized(_) => state.on_document_ready(),
    }
    debug!("Handling next message...Done");

    Ok(())
}

#[derive(Error, Debug)]
pub enum UserClientError {
    #[error("Invalid Host")]
    InvalidHost(String),

    #[error("Connection Error")]
    ConnectionError(tungstenite::Error),
}

/// Enumeration describing incoming messages to your client.
#[derive(Debug)]
pub enum IncomingMessage<T: UserClientState> {
    /// Message from the server
    NetworkMessage(Vec<u8>),
    /// Socket shutdown from the server
    Closed,
    /// Message from an out of band command stream
    Command(T::CommandType),
}

/// Enumeration describing outgoing messages
#[derive(Debug)]
pub enum OutgoingMessage {
    /// Instruct client machinery to shut down
    Close,
    /// Invoke a message on the server
    MethodInvoke(ClientInvokeMessage),
}

/// Start running the client machinery.
///
/// Will create the given user client state type when needed
pub async fn start_client<T>(
    url: String,
    name: String,
    a: T::ArgumentType,
) -> Result<(), UserClientError>
where
    T: UserClientState + 'static + std::fmt::Debug,
    T::CommandType: std::marker::Send + std::fmt::Debug,
    T::ArgumentType: std::marker::Send,
{
    // create streams to stop machinery
    let (stop_tx, mut stop_rx) = tokio::sync::broadcast::channel::<u8>(1);

    info!("Connecting to {url}...");

    // connect to a server...
    let conn_result = connect_async(&url)
        .await
        .map_err(UserClientError::ConnectionError)?;

    info!("Connecting to {url}...");

    // Stream for messages going to client state
    let (to_client_thread_tx, to_client_thread_rx) = std::sync::mpsc::channel();
    // Stream for messages from client state
    let (from_client_thread_tx, from_client_thread_rx) =
        tokio::sync::mpsc::channel(16);

    // Stream to go from async world to the sync std stream
    let (inter_channel_tx, inter_channel_rx) = tokio::sync::mpsc::channel(16);

    let worker_stop_tx = stop_tx.clone();

    let h1 = std::thread::spawn(move || {
        debug!("Creating client worker thread...");
        client_worker_thread::<T>(
            to_client_thread_rx,
            from_client_thread_tx.clone(),
            worker_stop_tx,
            a,
        )
    });

    let to_c_handle = tokio::spawn(to_client_task::<T>(
        inter_channel_rx,
        to_client_thread_tx,
        stop_tx.subscribe(),
    ));

    let (ws_stream, _) = conn_result;

    // Split out our server connection
    let (mut socket_tx, mut socket_rx) = ws_stream.split();

    // Send the initial introduction message
    {
        let content = (
            ClientIntroductionMessage::message_id(),
            ClientIntroductionMessage { client_name: name },
        );

        let mut buffer = Vec::<u8>::new();

        ciborium::ser::into_writer(&content, &mut buffer).unwrap();

        socket_tx.send(Message::Binary(buffer)).await.unwrap();
    }

    // spawn task that forwards messages from the client to the socket
    let fhandle = tokio::spawn(forward_task(
        from_client_thread_rx,
        socket_tx,
        stop_tx.clone(),
        stop_tx.subscribe(),
    ));

    // Now handle all incoming messages
    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            msg = socket_rx.next() => {

                match msg.unwrap() {
                    Ok(x) => {
                        inter_channel_tx
                        .send(IncomingMessage::NetworkMessage(x.into_data()))
                        .await
                        .unwrap()
                    },
                    Err(_) => {
                        inter_channel_tx.send(IncomingMessage::Closed).await.unwrap();
                        break;
                    },
                }
            }
        }
    }

    info!("Closing client to {url}...");

    let join_res = tokio::join!(fhandle, to_c_handle);

    join_res.0.unwrap();
    join_res.1.unwrap();

    h1.join().unwrap();

    Ok(())
}

/// Task that consumes messages from a tokio stream and outputs it to a std stream
async fn to_client_task<T>(
    mut input: tokio::sync::mpsc::Receiver<IncomingMessage<T>>,
    output: std::sync::mpsc::Sender<IncomingMessage<T>>,
    mut stopper: tokio::sync::broadcast::Receiver<u8>,
) where
    T: UserClientState + 'static,
{
    debug!("Starting to-client task");
    loop {
        tokio::select! {
            _ = stopper.recv() => break,
            Some(msg) = input.recv() => output.send(msg).unwrap()
        }
    }
    debug!("Ending to-client task");
}

/// Task that sends handles from the client.
async fn forward_task(
    mut input: tokio::sync::mpsc::Receiver<OutgoingMessage>,
    mut output: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
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
                        output.close().await.unwrap();

                        // tell everyone to stop
                        stopper_tx.send(1).unwrap();
                        break;
                    }
                    OutgoingMessage::MethodInvoke(x) => {
                        let tuple = (ClientInvokeMessage::message_id(), x);
                        ciborium::ser::into_writer(&tuple, &mut buffer).unwrap()
                    }
                }

                output
                    .send(tokio_tungstenite::tungstenite::Message::Binary(buffer))
                    .await
                    .unwrap();
            }
        }
    }
    debug!("Ending thread forwarding task");
}

/// Run the client state in it's own thread
fn client_worker_thread<T>(
    input: std::sync::mpsc::Receiver<IncomingMessage<T>>,
    output: tokio::sync::mpsc::Sender<OutgoingMessage>,
    stopper: tokio::sync::broadcast::Sender<u8>,
    a: T::ArgumentType,
) where
    T: UserClientState + 'static,
    T::CommandType: std::marker::Send,
    T::ArgumentType: std::marker::Send,
{
    debug!("Starting client worker thread");
    let mut t = T::new(a, output);

    while let Ok(x) = input.recv() {
        match x {
            IncomingMessage::NetworkMessage(bytes) => {
                handle_next(&mut t, bytes.as_slice()).unwrap();
            }
            IncomingMessage::Closed => {
                break;
            }
            IncomingMessage::Command(c) => t.on_command(c),
        }
    }
    debug!("Ending client worker thread");

    stopper.send(1).unwrap();
}

//pub fn make_invoke_message
