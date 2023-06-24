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
use crate::{
    client_state::*,
    delegate::{Delegate, DocumentDelegate},
    server_root_message::*,
};
use ciborium::value::Value;
pub use colabrodo_common::client_communication::MethodInvokeMessage;
pub use colabrodo_common::server_communication::MessageMethodReply;
use colabrodo_common::{
    client_communication::{ClientMessageID, IntroductionMessage},
    nooid::*,
};
pub use colabrodo_common::{components::UpdatableWith, nooid::NooID};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use log::{debug, info};
use std::fmt::Debug;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio_tungstenite::{tungstenite, MaybeTlsStream};

// =============================================================================

/// Signals will be delivered along this channel type.
pub type SignalRecv = tokio::sync::broadcast::Receiver<Vec<Value>>;
//type SignalSource = tokio::sync::broadcast::Sender<Vec<Value>>;
//pub(crate) type SignalHash = HashMap<SignalID, SignalSource>;

// fn fire_signal(id: SignalID, repo: &SignalHash, args: Vec<Value>) {
//     if let Some(sender) = repo.get(&id) {
//         sender.send(args).unwrap();
//     }
// }

// =============================================================================

/*
/// A component list type that allows subscription to signals
#[derive(Debug, Default)]
pub struct SigModComponentList<IDType, State, Del>
where
    IDType: Eq + Hash + Copy,
    State: UpdatableWith + NamedComponent + CommComponent,
{
    list: ComponentList<IDType, State, Del>,
    signals: HashMap<IDType, SignalHash>,
}

impl<IDType, State, Del> SigModComponentList<IDType, State, Del>
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
 */
// =============================================================================

/// Context for a method invocation
pub enum InvokeContext {
    Document,
    Entity(EntityID),
    Table(TableID),
    Plot(PlotID),
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
pub fn handle_next<Provider: DelegateProvider>(
    state: &mut ClientState<Provider>,
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
        handle_next_message(state, msg)?;
    }

    Ok(())
}

fn handle_next_message<Provider: DelegateProvider>(
    state: &mut ClientState<Provider>,
    m: FromServer,
) -> Result<(), UserClientNext> {
    debug!("Handling next message...");

    if log::log_enabled!(log::Level::Debug) {
        debug!("Message content: {:?}", m);
    }

    match m {
        FromServer::Method(m) => match m {
            ModMethod::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::MethodDelegate::on_new(x.id, x.content, state);
                state.method_list.on_create(x.id, Some(name), del);
            }
            ModMethod::Delete(x) => state.method_list.on_delete(x.id),
        },
        //
        FromServer::Signal(s) => match s {
            ModSignal::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::SignalDelegate::on_new(x.id, x.content, state);
                state.signal_list.on_create(x.id, Some(name), del)
            }
            ModSignal::Delete(x) => state.signal_list.on_delete(x.id),
        },
        //
        FromServer::Entity(x) => match x {
            ModEntity::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::EntityDelegate::on_new(x.id, x.content, state);
                state.entity_list.on_create(x.id, name, del)
            }
            ModEntity::Update(x) => {
                state.entity_list.on_update(x.id, x.content)
            }
            ModEntity::Delete(x) => state.entity_list.on_delete(x.id),
        },
        //
        FromServer::Plot(x) => match x {
            ModPlot::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::PlotDelegate::on_new(x.id, x.content, state);
                state.plot_list.on_create(x.id, name, del)
            }
            ModPlot::Update(x) => state.plot_list.on_update(x.id, x.content),
            ModPlot::Delete(x) => state.plot_list.on_delete(x.id),
        },
        //
        FromServer::Buffer(s) => match s {
            ModBuffer::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::BufferDelegate::on_new(x.id, x.content, state);
                state.buffer_list.on_create(x.id, name, del)
            }
            ModBuffer::Delete(x) => state.buffer_list.on_delete(x.id),
        },
        //
        FromServer::BufferView(s) => match s {
            ModBufferView::Create(x) => {
                let name = x.content.name.clone();
                let del = Provider::BufferViewDelegate::on_new(
                    x.id, x.content, state,
                );
                state.buffer_view_list.on_create(x.id, name, del)
            }
            ModBufferView::Delete(x) => state.buffer_view_list.on_delete(x.id),
        },
        //
        FromServer::Material(s) => match s {
            ModMaterial::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::MaterialDelegate::on_new(x.id, x.content, state);
                state.material_list.on_create(x.id, name, del)
            }
            ModMaterial::Update(x) => {
                state.material_list.on_update(x.id, x.content)
            }
            ModMaterial::Delete(x) => state.material_list.on_delete(x.id),
        },
        //
        FromServer::Image(s) => match s {
            ModImage::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::ImageDelegate::on_new(x.id, x.content, state);
                state.image_list.on_create(x.id, name, del)
            }
            ModImage::Delete(x) => state.image_list.on_delete(x.id),
        },
        //
        FromServer::Texture(s) => match s {
            ModTexture::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::TextureDelegate::on_new(x.id, x.content, state);
                state.texture_list.on_create(x.id, name, del)
            }
            ModTexture::Delete(x) => state.texture_list.on_delete(x.id),
        },
        //
        FromServer::Sampler(s) => match s {
            ModSampler::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::SamplerDelegate::on_new(x.id, x.content, state);
                state.sampler_list.on_create(x.id, name, del)
            }
            ModSampler::Delete(x) => state.sampler_list.on_delete(x.id),
        },
        //
        FromServer::Light(s) => match s {
            ModLight::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::LightDelegate::on_new(x.id, x.content, state);
                state.light_list.on_create(x.id, name, del)
            }
            ModLight::Update(x) => state.light_list.on_update(x.id, x.content),
            ModLight::Delete(x) => state.light_list.on_delete(x.id),
        },
        //
        FromServer::Geometry(s) => match s {
            ModGeometry::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::GeometryDelegate::on_new(x.id, x.content, state);
                state.geometry_list.on_create(x.id, name, del)
            }
            ModGeometry::Delete(x) => state.geometry_list.on_delete(x.id),
        },
        //
        FromServer::Table(s) => match s {
            ModTable::Create(x) => {
                let name = x.content.name.clone();
                let del =
                    Provider::TableDelegate::on_new(x.id, x.content, state);
                state.table_list.on_create(x.id, name, del)
            }
            ModTable::Update(x) => state.table_list.on_update(x.id, x.content),
            ModTable::Delete(x) => state.table_list.on_delete(x.id),
        },
        //
        FromServer::MsgDocumentUpdate(x) => {
            // this is a pretty rough hack
            // But there is no clean safe way to pass mutable self twice.
            let doc = std::mem::take(&mut state.document);

            if let Some(mut doc) = doc {
                doc.on_document_update(state, x);
                state.document = Some(doc);
            }
        }
        FromServer::MsgDocumentReset(_) => {
            state.clear();
        }
        //
        FromServer::MsgSignalInvoke(x) => {
            state.handle_signal(x);
        }
        FromServer::MsgMethodReply(x) => {
            let invoke_id: uuid::Uuid = x
                .invoke_id
                .parse()
                .map_err(|_| UserClientNext::MethodError)?;

            state.handle_method_reply(invoke_id, x);
        }
        FromServer::MsgDocumentInitialized(_) => {
            if let Some(mut local_doc) = state.document.take() {
                local_doc.on_ready(state);
                state.document = Some(local_doc);
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
    pub(crate) to_client_rx:
        tokio::sync::mpsc::UnboundedReceiver<IncomingMessage>,
    pub(crate) from_client_tx:
        tokio::sync::mpsc::UnboundedSender<OutgoingMessage>,
    pub(crate) stopper: tokio::sync::broadcast::Sender<u8>,
}

impl ClientChannels {
    pub fn get_stopper(&self) -> tokio::sync::broadcast::Receiver<u8> {
        self.stopper.subscribe()
    }
}

/// Start running the client machinery.
///
/// # Parameters
/// - url: The host to connect to
/// - name: The name of the client to use during introduction to the server
/// - channels: Input/output channels
pub async fn start_client_stream(
    url: String,
    name: String,
) -> Result<ClientChannels, UserClientError> {
    // create streams to stop machinery
    let (stop_tx, _) = tokio::sync::broadcast::channel::<u8>(1);

    let (to_client_tx, to_client_rx) = tokio::sync::mpsc::unbounded_channel();
    let (from_client_tx, from_client_rx) =
        tokio::sync::mpsc::unbounded_channel();

    info!("Connecting to {url}...");

    // connect to a server...
    let conn_result = connect_async(&url)
        .await
        .map_err(UserClientError::ConnectionError)?;

    info!("Connected to {url}...");

    //let worker_stop_tx = stop_tx.clone();

    let (ws_stream, _) = conn_result;

    // Split out our server connection
    let (mut socket_tx, socket_rx) = ws_stream.split();

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
        from_client_rx,
        socket_tx,
        stop_tx.clone(),
        stop_tx.subscribe(),
    ));

    debug!("Tasks launched");

    tokio::spawn(incoming_message_task(
        stop_tx.clone(),
        socket_rx,
        to_client_tx,
    ));

    //debug!("Loop closed. Client system done.");

    //#error we never get here

    Ok(ClientChannels {
        to_client_rx,
        from_client_tx,
        stopper: stop_tx.clone(),
    })
}

async fn incoming_message_task(
    stopper_tx: tokio::sync::broadcast::Sender<u8>,
    mut socket_rx: futures_util::stream::SplitStream<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
    >,
    to_client_tx: tokio::sync::mpsc::UnboundedSender<IncomingMessage>,
) {
    let mut stop_rx = stopper_tx.subscribe();
    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            msg = socket_rx.next() => {

                match msg.unwrap() {
                    Ok(x) => {
                        to_client_tx
                        .send(IncomingMessage::NetworkMessage(x.into_data()))
                        .unwrap();
                    },
                    Err(_) => {
                        to_client_tx.send(IncomingMessage::Closed).unwrap();
                        break;
                    },
                }
            }
        }
    }
}

/// Task that sends outgoing messages from the client to the socket.
async fn forward_task(
    mut input: tokio::sync::mpsc::UnboundedReceiver<OutgoingMessage>,
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

/// Consume messages and apply it to a given client state
/// This blocks and should be run in a thread
pub fn launch_client_worker_thread<Provider: DelegateProvider>(
    mut channels: ClientChannels,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        debug!("Starting client worker thread");

        let mut state = ClientState::<Provider>::new(&channels);

        while let Some(x) = channels.to_client_rx.blocking_recv() {
            match x {
                IncomingMessage::NetworkMessage(root) => {
                    handle_next(&mut state, root).unwrap();
                }
                IncomingMessage::Closed => {
                    break;
                }
            }
        }
        debug!("Ending client worker thread");

        let _ = channels.stopper.send(1);

        state.clear();
    })
}
