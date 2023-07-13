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
use crate::{client_state::*, server_root_message::*};
pub use ciborium::value::Value;
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

macro_rules! do_item_update {
    ($list:ident, $state:expr, $info:expr) => {{
        let delegate = $state.delegate_lists.$list.take(&($info).id);
        if let Some(mut delegate) = delegate {
            delegate.on_update(&mut $state.delegate_lists, $info.content);
            $state.delegate_lists.$list.replace(&$info.id, delegate);
        }
    }};
}

/// Execute the next message from a server on your client state
pub fn handle_next(
    state: &mut ClientState,
    message: &[u8],
) -> Result<(), UserClientNext> {
    if log::log_enabled!(log::Level::Debug) {
        let v: ciborium::value::Value =
            ciborium::de::from_reader(message).unwrap();
        debug!("Content: {v:?}");
    }

    let root: ServerRootMessage = ciborium::de::from_reader(message).unwrap();

    debug!("Got {} messages", root.list.len());

    for msg in root.list {
        handle_next_message(state, msg)?;
    }

    Ok(())
}

fn handle_next_message(
    state: &mut ClientState,
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
                let del = state.maker.make_method(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.method_list.on_create(
                    x.id,
                    Some(name),
                    del,
                );
            }
            ModMethod::Delete(x) => {
                state.delegate_lists.method_list.on_delete(x.id)
            }
        },
        //
        FromServer::Signal(s) => match s {
            ModSignal::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_signal(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.signal_list.on_create(
                    x.id,
                    Some(name),
                    del,
                )
            }
            ModSignal::Delete(x) => {
                state.delegate_lists.signal_list.on_delete(x.id)
            }
        },
        //
        FromServer::Entity(x) => match x {
            ModEntity::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_entity(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.entity_list.on_create(x.id, name, del)
            }
            ModEntity::Update(x) => {
                do_item_update!(entity_list, state, x);
            }
            ModEntity::Delete(x) => {
                state.delegate_lists.entity_list.on_delete(x.id)
            }
        },
        //
        FromServer::Plot(x) => match x {
            ModPlot::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_plot(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.plot_list.on_create(x.id, name, del)
            }
            ModPlot::Update(x) => {
                do_item_update!(plot_list, state, x);
            }
            ModPlot::Delete(x) => {
                state.delegate_lists.plot_list.on_delete(x.id)
            }
        },
        //
        FromServer::Buffer(s) => match s {
            ModBuffer::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_buffer(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.buffer_list.on_create(x.id, name, del)
            }
            ModBuffer::Delete(x) => {
                state.delegate_lists.buffer_list.on_delete(x.id)
            }
        },
        //
        FromServer::BufferView(s) => match s {
            ModBufferView::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_buffer_view(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state
                    .delegate_lists
                    .buffer_view_list
                    .on_create(x.id, name, del)
            }
            ModBufferView::Delete(x) => {
                state.delegate_lists.buffer_view_list.on_delete(x.id)
            }
        },
        //
        FromServer::Material(s) => match s {
            ModMaterial::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_material(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state
                    .delegate_lists
                    .material_list
                    .on_create(x.id, name, del)
            }
            ModMaterial::Update(x) => {
                do_item_update!(material_list, state, x);
            }
            ModMaterial::Delete(x) => {
                state.delegate_lists.material_list.on_delete(x.id)
            }
        },
        //
        FromServer::Image(s) => match s {
            ModImage::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_image(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.image_list.on_create(x.id, name, del)
            }
            ModImage::Delete(x) => {
                state.delegate_lists.image_list.on_delete(x.id)
            }
        },
        //
        FromServer::Texture(s) => match s {
            ModTexture::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_texture(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.texture_list.on_create(x.id, name, del)
            }
            ModTexture::Delete(x) => {
                state.delegate_lists.texture_list.on_delete(x.id)
            }
        },
        //
        FromServer::Sampler(s) => match s {
            ModSampler::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_sampler(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.sampler_list.on_create(x.id, name, del)
            }
            ModSampler::Delete(x) => {
                state.delegate_lists.sampler_list.on_delete(x.id)
            }
        },
        //
        FromServer::Light(s) => match s {
            ModLight::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_light(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.light_list.on_create(x.id, name, del)
            }
            ModLight::Update(x) => {
                do_item_update!(light_list, state, x);
            }
            ModLight::Delete(x) => {
                state.delegate_lists.light_list.on_delete(x.id)
            }
        },
        //
        FromServer::Geometry(s) => match s {
            ModGeometry::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_geometry(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state
                    .delegate_lists
                    .geometry_list
                    .on_create(x.id, name, del)
            }
            ModGeometry::Delete(x) => {
                state.delegate_lists.geometry_list.on_delete(x.id)
            }
        },
        //
        FromServer::Table(s) => match s {
            ModTable::Create(x) => {
                let name = x.content.name.clone();
                let del = state.maker.make_table(
                    x.id,
                    x.content,
                    &mut state.delegate_lists,
                );
                state.delegate_lists.table_list.on_create(x.id, name, del)
            }
            ModTable::Update(x) => {
                do_item_update!(table_list, state, x);
            }
            ModTable::Delete(x) => {
                state.delegate_lists.table_list.on_delete(x.id)
            }
        },
        //
        FromServer::MsgDocumentUpdate(x) => {
            // this is a pretty rough hack
            // But there is no clean safe way to pass mutable self twice.
            let doc = std::mem::take(&mut state.delegate_lists.document);

            if let Some(mut doc) = doc {
                doc.on_document_update(state, x);
                state.delegate_lists.document = Some(doc);
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
            if let Some(mut local_doc) = state.delegate_lists.document.take() {
                local_doc.on_ready(state);
                state.delegate_lists.document = Some(local_doc);
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

pub fn start_blank_stream() -> ClientChannels {
    let (stop_tx, _) = tokio::sync::broadcast::channel::<u8>(1);

    let (_, to_client_rx) = tokio::sync::mpsc::unbounded_channel();
    let (from_client_tx, _) = tokio::sync::mpsc::unbounded_channel();

    ClientChannels {
        to_client_rx,
        from_client_tx,
        stopper: stop_tx,
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
pub fn launch_client_worker_thread<Maker>(
    mut channels: ClientChannels,
    maker: Maker,
) -> std::thread::JoinHandle<()>
where
    Maker: DelegateMaker + Send + 'static,
{
    std::thread::spawn(move || {
        debug!("Starting client worker thread");

        let mut state = ClientState::new(&channels, maker);

        while let Some(x) = channels.to_client_rx.blocking_recv() {
            match x {
                IncomingMessage::NetworkMessage(root) => {
                    handle_next(&mut state, root.as_slice()).unwrap();
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
