//! A tokio powered server framework. Users can plug in a user server struct to this framework to obtain a coroutine powered NOODLES server.

use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;

use crate::server_messages::AsyncMethodContent;
use crate::server_messages::MethodReference;
use crate::server_messages::Recorder;
pub use crate::server_messages::{
    ComponentReference, MethodHandlerSlot, ServerDocumentUpdate,
    ServerMethodState,
};
pub use crate::server_state::MethodResult;
use crate::server_state::Output;
pub use crate::server_state::{InvokeObj, ServerState, ServerStatePtr};
pub use ciborium;
pub use colabrodo_common::client_communication::InvokeIDType;
use colabrodo_common::client_communication::{
    AllClientMessages, ClientRootMessage, MethodInvokeMessage,
};
use colabrodo_common::common::ServerMessageIDs;
pub use colabrodo_common::server_communication::*;
pub use colabrodo_common::value_tools::*;
pub use colabrodo_macros::make_method_function;
use futures_util::SinkExt;
use futures_util::StreamExt;
use log;
use thiserror::Error;
pub use tokio;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message as WSMessage;
pub use uuid;

// We have a fun structure here.
// First there is a thread for handling the server state, which is controlled through queues.
// We then have a task that takes broadcast messages from the server thread and pumps it to clients through a tokio bcast queue. Using the bcast queue directly, as we get into issues of thread sync; the server is not thread safe and any attempt to use await in regards to the server might cause it to cross thread boundaries. Tokio lets you lock a tast to a thread, but it is extremely non-pleasant to structure this to do so.
// We have a task that listens for clients, that spawns a per client task.

/// Dump cbor to the terminal in both diagnostic and in byte form for debugging.
#[allow(dead_code)]
fn debug_cbor(data: &Vec<u8>) {
    let output: ciborium::value::Value =
        match ciborium::de::from_reader(data.as_slice()) {
            Err(x) => {
                println!("Unable to decode: {x:?}");
                return;
            }
            Ok(v) => v,
        };
    println!("Decoded {output:?} | {data:02X?}");
}

#[derive(Debug, Clone)]
pub struct ClientRecord {
    pub id: uuid::Uuid,
    pub sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

/// Models a message from a client. As the server thread doesn't know who sent it, we pass along a lightweight handle to the client-specific queue so the server can send specific messages along
#[derive(Debug)]
struct FromClientMessage(uuid::Uuid, Vec<u8>);

#[derive(Debug)]
enum ToServerMessage {
    ClientConnected(ClientRecord),
    Client(FromClientMessage),
    ClientClosed(uuid::Uuid),
    //Shutdown,
    //Command(CommandType),
}

pub struct ServerOptions {
    pub host: String,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            host: "0.0.0.0:50000".to_string(),
        }
    }
}

/// Public entry point to the server process.
///
/// Note that this function will spawn a number of threads.
///
/// # Example
///
/// A simple example to create a server:
///
/// ```rust,ignore
/// let opts = ServerOptions::default();
/// let mut state = ServerState::new();
///
/// //setup(&mut state);
///
/// server_main(opts, state).await;
///
/// ```
pub async fn server_main(opts: ServerOptions, state: Arc<Mutex<ServerState>>) {
    let bcast_send = state.lock().unwrap().new_broadcast_send();

    // channel for task control
    let (stop_tx, _stop_rx) = tokio::sync::broadcast::channel::<u8>(1);

    // channel for server to recv messages
    let (to_server_send, to_server_recv) = tokio::sync::mpsc::channel(16);

    // // channel for server to send messages to all clients
    // let (from_server_send, from_server_recv) = tokio::sync::mpsc::channel(16);

    let listener = listen(&opts).await;

    let local_addy =
        listener.local_addr().expect("Unable to find local address");

    log::info!("NOODLES server accepting clients @ {local_addy}");

    let stop_watch_task =
        tokio::spawn(shutdown_watcher(bcast_send.subscribe(), stop_tx.clone()));

    // move the listener off to start accepting clients
    // it needs a handle for the broadcast channel to hand to new clients
    // and a handle to the to_server stream.
    let h1 = tokio::spawn(client_connect_task(
        listener,
        bcast_send.clone(),
        to_server_send.clone(),
        stop_tx.clone(),
        stop_tx.subscribe(),
    ));

    // if let Some(q) = command_queue {
    //     tokio::spawn(command_sender::<T>(q, to_server_send.clone()));
    // }

    // let state_handle = thread::spawn(move || {

    // });

    server_state_loop(state, to_server_recv, stop_tx.subscribe()).await;

    //server_message_pump(bcast_send, from_server_recv, to_server_send.clone())
    //    .await;

    log::debug!("Server is closing down, waiting for stopwatch task...");

    stop_watch_task.await.unwrap();

    log::debug!("Server is closing down, waiting for client connect task...");

    h1.await.unwrap();

    //state_handle.join().unwrap();

    log::debug!("Server is done.");
}

async fn shutdown_watcher(
    mut server_bcast: tokio::sync::broadcast::Receiver<Output>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
) {
    while let Ok(msg) = server_bcast.recv().await {
        log::debug!("WATCHER GOT A MESSAGE");
        if let Output::Shutdown = msg {
            log::debug!("Server is asking to stop, broadcasting stop bit");
            stop_tx.send(1).unwrap();
            return;
        }
    }
}

// Task to construct a listening socket
async fn listen(opts: &ServerOptions) -> TcpListener {
    TcpListener::bind(&opts.host)
        .await
        .expect("Unable to bind to address")
}

// Task that waits for a new client to connect and spawns a new client
// handler task
async fn client_connect_task(
    listener: TcpListener,
    bcast_send: broadcast::Sender<Output>,
    to_server_send: tokio::sync::mpsc::Sender<ToServerMessage>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
    mut stop_rx: tokio::sync::broadcast::Receiver<u8>,
) {
    log::debug!("Starting client connect task");

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            acc = listener.accept() => {
                if let Ok((stream, _)) = acc {
                    tokio::spawn(client_handler(
                        stream,
                        to_server_send.clone(),
                        bcast_send.subscribe(),
                        stop_tx.clone(),
                    ));
                }
            }
        }
    }

    log::debug!("Stopping client connect task");
}

/// handles server state; the state itself is not thread safe, so we isolate it
/// to this task.
async fn server_state_loop(
    server_state: Arc<Mutex<ServerState>>,
    mut from_world: tokio::sync::mpsc::Receiver<ToServerMessage>,
    mut stop_rx: tokio::sync::broadcast::Receiver<u8>,
) {
    log::debug!("Starting server state thread");

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            Some(msg) = from_world.recv() => {
                match msg {
                    ToServerMessage::ClientConnected(cr) => {
                        let mut lock = server_state.lock().unwrap();

                        lock.active_client_info.insert(cr.id, cr.clone());
                    }
                    ToServerMessage::Client(client_msg) => {
                        // handle a message from a client, and write any replies
                        // to the client's output queue
                        if log::log_enabled!(log::Level::Debug) {
                            log::debug!("RECV:");
                            debug_cbor(&client_msg.1);
                        }

                        let client = {
                            let lock = server_state.lock().unwrap();

                            lock.active_client_info.get(&client_msg.0).cloned().unwrap()
                        };

                        let result = handle_next(
                                &server_state,
                                client_msg.1,
                                client_msg.0,
                                |out| {
                                    if log::log_enabled!(log::Level::Debug) {
                                        log::debug!("SEND TO CLIENT:");
                                        debug_cbor(&out);
                                    }
                                    // clients could already be gone, so don't
                                    // unwrap.
                                    let _ = client.sender.send(out);
                                },
                            ).await;

                        if let Err(x) = result {
                            log::warn!("Unable to handle message from client: {x:?}");
                        }
                    }
                    ToServerMessage::ClientClosed(id) => {
                        let mut lock = server_state.lock().unwrap();

                        lock.active_client_info.remove(&id);
                    }
                    // ToServerMessage::Shutdown => {
                    //     stop_tx.send(1).unwrap();
                    //     break;
                    // }
                    //ToServerMessage::Command(comm_msg) => {
                        //user_state.handle_command(comm_msg);
                    //}
                }
            }
        }
    }

    log::debug!("Ending server state thread");
}

/// Task for each client that has joined up
async fn client_handler(
    stream: TcpStream,
    to_server_send: tokio::sync::mpsc::Sender<ToServerMessage>,
    mut bcast_recv: broadcast::Receiver<Output>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
) -> Result<(), ()> {
    let addr = stream
        .peer_addr()
        .expect("Connected stream missing peer address");

    log::info!("Peer address: {addr}");

    // set up the websocket
    let websocket = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during handshake");

    log::info!("Websocket ready: {addr}");

    let client_id = uuid::Uuid::new_v4();

    log::info!("Identifying client as: {client_id}");

    // tx an rx are the core websocket channels
    let (mut tx, mut rx) = websocket.split();

    // set up a queue for messages to be sent to the socket
    // probably more elegant ways of doing this, but this seems to be
    // an appropriate way to merge messages from a broadcast and possible
    // single-client replies
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<tokio_tungstenite::tungstenite::Message>();

    let (client_stop_tx, mut client_stop_rx) = broadcast::channel(1);

    let mut socket_stopper = stop_tx.subscribe();
    let h1 = tokio::spawn(async move {
        // task that just sends data to the socket
        loop {
            tokio::select! {
                _ = socket_stopper.recv() => break,
                _ = client_stop_rx.recv() => break,
                data = out_rx.recv() => {
                    if let Some(data) = data {
                        // for each message, just send it along
                        tx.send(data)
                        .await
                        .unwrap();
                    }
                }
            }
        }
        log::debug!("Ending per-client data-forwarder");
    });

    let mut bcast_stopper = stop_tx.subscribe();
    // task that takes broadcast information and sends it to the out queue
    let h2 = {
        let this_tx = out_tx.clone();
        let mut this_client_stopper = client_stop_tx.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                        _ = bcast_stopper.recv() => break,
                        _ = this_client_stopper.recv() => break,
                        bcast = bcast_recv.recv() => {
                // take each message from the broadcast channel and add it to the
                // queue
                            if let Ok(Output::Broadcast(bcast)) = bcast {
                                this_tx.send(tokio_tungstenite::tungstenite::Message::Binary(bcast)).unwrap();
                            }
                        }
                    }
            }

            log::debug!("Ending per-client broadcast-forwarder");
        })
    };

    let (out_raw_tx, mut out_raw_rx) =  mpsc::unbounded_channel();

    let out_clone = out_tx.clone();

    tokio::spawn(async move {
        
        loop {
            while let Some(x) = out_raw_rx.recv().await {
                out_clone.send(tokio_tungstenite::tungstenite::Message::Binary(x)).unwrap();
            }
        }
    });

    to_server_send
        .send(ToServerMessage::ClientConnected(ClientRecord {
            id: client_id,
            sender: out_raw_tx.clone(),
        }))
        .await
        .unwrap();

    let mut stop_rx = stop_tx.subscribe();

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,

            message = rx.next() => match message {
                None => break,
                // handle recv of any data, and forward on to the server
                Some(message) => {
                    let message = match message {
                        Ok(x) => x,
                        Err(error) => {
                            log::warn!("Client disconnected: {error:?}");
                            break;
                        }
                    };

                    match message {
                        WSMessage::Binary(x) => {
                            to_server_send
                            .send(ToServerMessage::Client(FromClientMessage(
                                client_id,
                                x,
                            ))).await
                            .unwrap();
                        }
                        WSMessage::Text(_) => {
                            log::warn!("Client {} sent text, which is not supported at this time. Closing.", client_id);
                        }
                        WSMessage::Pong(_) => {
                            log::debug!("Pong from client {}", client_id);
                        }
                        WSMessage::Ping(x) => {
                            out_tx.send(tokio_tungstenite::tungstenite::Message::Pong(x))
                            .unwrap();
                        }
                        WSMessage::Close(_) => {
                            log::debug!("Client {} sent close...", client_id);
                        }                        
                        _ => {
                            log::debug!("Unknown message from client {}, closing connection", client_id);
                            break;
                        }
                    }
                }
            }
        }
    }

    //stop_tx.send(1).unwrap();

    let _ = to_server_send
        .send(ToServerMessage::ClientClosed(client_id))
        .await;

    log::info!("Closing client, waiting for tasks...");

    let _ = client_stop_tx.send(1);

    h1.await.unwrap();
    h2.await.unwrap();

    log::info!("Client closed.");

    Ok(())
}

// =============================================================================

/// Helper function to determine if a method is indeed attached to a given target.
fn find_method_in_state(
    method: &MethodReference,
    state: &Option<Vec<MethodReference>>,
) -> bool {
    match state {
        None => false,
        Some(x) => {
            for m in x {
                if m.id() == method.id() {
                    return true;
                }
            }
            false
        }
    }
}

/// Helper function to actually invoke a method
///
/// Determines if the method exists, can be invoked on the target, etc, and if so, dispatches to the user server
async fn invoke_helper(
    state: &Arc<Mutex<ServerState>>,
    client_id: uuid::Uuid,
    invoke: MethodInvokeMessage,
) -> MethodResult {
    let signal = {
        let lock = state.lock().unwrap();

        let method = lock.methods.resolve(invoke.method).ok_or_else(|| {
            MethodException {
                code: ExceptionCodes::MethodNotFound as i32,
                ..Default::default()
            }
        })?;

        // get context

        let context = match invoke.context {
            None => Some(InvokeObj::Document),
            Some(id) => match id {
                InvokeIDType::Entity(eid) => {
                    lock.entities.resolve(eid).map(InvokeObj::Entity)
                }
                InvokeIDType::Table(eid) => {
                    lock.tables.resolve(eid).map(InvokeObj::Table)
                }
                InvokeIDType::Plot(eid) => {
                    lock.plots.resolve(eid).map(InvokeObj::Plot)
                }
            },
        };

        let context = context.ok_or_else(|| MethodException {
            code: ExceptionCodes::MethodNotFound as i32,
            ..Default::default()
        })?;

        // make sure the object has the method attached

        let has_method = match &context {
            InvokeObj::Document => {
                find_method_in_state(&method, &lock.comm.methods_list)
            }
            InvokeObj::Entity(x) => {
                x.0.inspect(|t| {
                    find_method_in_state(&method, &t.mutable.methods_list)
                })
                .unwrap_or(false)
            }
            InvokeObj::Plot(x) => {
                x.0.inspect(|t| {
                    find_method_in_state(&method, &t.mutable.methods_list)
                })
                .unwrap_or(false)
            }
            InvokeObj::Table(x) => {
                x.0.inspect(|t| {
                    find_method_in_state(&method, &t.mutable.methods_list)
                })
                .unwrap_or(false)
            }
        };

        if !has_method {
            return Err(MethodException {
                code: ExceptionCodes::MethodNotFound as i32,
                ..Default::default()
            });
        }

        // send it along
        lock.methods
            .inspect(method.id(), |m| m.state.clone())
            .unwrap()
    };

    let msg = AsyncMethodContent {
        state: state.clone(),
        context: invoke.context,
        args: invoke.args,
        from: client_id,
    };

    if let Some(s) = signal.channels {
        let mut func = s.lock().unwrap();

        if let Some(rep) = func.activate(msg).await {
            return rep.result;
        }
    }

    Err(MethodException {
        code: ExceptionCodes::InternalError as i32,
        ..Default::default()
    })
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Decode error")]
    DecodeError(String),
    #[error("Root message is not valid")]
    InvalidRootMessage(String),
}

/// Drive state changes by handling the next message to the server.
///
/// This function takes a user server state, a message, and a writeback function. The message is assumed to be encoded in CBOR. If a specific message is needed to be sent back, the `write` function argument will be called with the content; it is up to user code to determine how to send that to the client.
pub async fn handle_next<F>(
    state: &Arc<Mutex<ServerState>>,
    msg: Vec<u8>,
    client_id: uuid::Uuid,
    write: F,
) -> Result<(), ServerError>
where
    F: Fn(Vec<u8>),
{
    log::debug!("Handling next message...");
    // extract a typed message from the input stream
    let root_message = ciborium::de::from_reader(msg.as_slice());

    let root_message: ClientRootMessage = root_message
        .map_err(|_| ServerError::InvalidRootMessage("Unable to extract root client message; this should just be a CBOR array.".to_string()))?;

    for message in root_message.list {
        match message {
            AllClientMessages::Intro(_) => {
                // dump current state to the client. The serde handler should
                // do the right thing here and make one big list of messages
                // to send back
                log::debug!("Client joined, providing initial state");
                let mut recorder = Vec::<u8>::new();
                {
                    let lock = state.lock().unwrap();
                    ciborium::ser::into_writer(&*lock, &mut recorder).unwrap();
                }
                write(recorder);
            }
            AllClientMessages::Invoke(invoke) => {
                log::debug!("Next message is invoke...");
                // copy the reply ident
                let reply_id = invoke.invoke_id.clone();

                // invoke the method and get the result or error
                let result = invoke_helper(state, client_id, invoke).await;

                // if we have a reply id, then we can ship a response. Otherwise, we just skip this step.
                if let Some(resp) = reply_id {
                    // Format a reply object
                    let mut reply = MessageMethodReply {
                        invoke_id: resp,
                        ..Default::default()
                    };

                    // only fill in a certain field if a reply or exception...
                    match result {
                        Err(x) => {
                            reply.method_exception = Some(x);
                        }
                        Ok(result) => {
                            reply.result = result;
                        }
                    }

                    // now send it back
                    let recorder = Recorder::record(
                        ServerMessageIDs::MsgMethodReply as u32,
                        &reply,
                    );

                    write(recorder.data);
                }
            }
        }
    }

    Ok(())
}
