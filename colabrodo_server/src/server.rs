//! A tokio powered server framework. Users can plug in a user server struct to this framework to obtain a coroutine powered NOODLES server.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;

use futures_util::SinkExt;
use futures_util::StreamExt;
use log;
use log::log_enabled;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message as WSMessage;

use crate::server_state;
use crate::server_state::Output;
pub use crate::server_state::{
    CallbackPtr, InvokeObj, ServerState, UserServerState,
};

pub use tokio;

pub use ciborium;

pub use uuid;

pub use crate::server_messages::{ComponentReference, MethodState};
pub use crate::server_state::MethodResult;

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
enum ToServerMessage<CommandType> {
    ClientConnected(ClientRecord),
    Client(FromClientMessage),
    ClientClosed(uuid::Uuid),
    //Shutdown,
    Command(CommandType),
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

#[derive(Debug)]
/// A placeholder if the user server doesn't support commands
pub struct DefaultCommand {}

#[derive(Debug)]
/// A placeholder if the user server doesn't support initialization arguments
pub struct NoInit {}

/// Trait for user servers to implement
pub trait AsyncServer: UserServerState {
    /// Type to use for the handle command callback
    type CommandType;

    /// Type to use for additional initialization
    type InitType;

    /// Called when our framework needs to create the user state
    fn new(tx: server_state::CallbackPtr, init: Self::InitType) -> Self;

    /// Called after the user state has been created. Extra setup can happen here.
    fn initialize_state(&mut self) {}

    /// Called when a new client connects. This is before any introduction is received
    #[allow(unused_variables)]
    fn client_connected(&mut self, record: ClientRecord) {}

    /// Called when a client disconnects.
    #[allow(unused_variables)]
    fn client_disconnected(&mut self, client_id: uuid::Uuid) {}

    /// Called when an async command is received
    #[allow(unused_variables)]
    fn handle_command(&mut self, command: Self::CommandType) {}
}

/// Public entry point to the server process.
///
/// Note that this function will spawn a number of threads.
///
/// # Example
/// Users should define a type that conforms to UserServerState + AsyncServer and pass it in like:
/// ```rust,ignore
/// struct MyServer {
///     state: ServerState,
///     // your state
///     // .. etc
/// }
///
/// impl UserServerState for MyServer {
///     // ...
/// }
///
/// impl AsyncServer for MyServer {
///     // ...
/// }
///
/// let opts = ServerOptions::default();
/// colabrodo_common::server_tokio::server_main::<MyServer>(opts);
///
/// ```
pub async fn server_main<T>(opts: ServerOptions, init: T::InitType)
where
    T: AsyncServer + 'static,
    T::CommandType: Debug + std::marker::Send,
    <T as AsyncServer>::InitType: std::marker::Send,
{
    server_main_with_command_queue::<T>(opts, init, None).await;
}

pub async fn server_main_with_command_queue<T>(
    opts: ServerOptions,
    init: T::InitType,
    command_queue: Option<mpsc::Receiver<T::CommandType>>,
) where
    T: AsyncServer + 'static,
    T::CommandType: Debug + std::marker::Send,
    <T as AsyncServer>::InitType: std::marker::Send,
{
    // channel for task control
    let (stop_tx, _stop_rx) = tokio::sync::broadcast::channel::<u8>(1);

    // channel for messages to be sent to all clients
    let (bcast_send, bcast_recv) = broadcast::channel(16);

    // channel for server to recv messages
    let (to_server_send, to_server_recv) = tokio::sync::mpsc::channel(16);

    // // channel for server to send messages to all clients
    // let (from_server_send, from_server_recv) = tokio::sync::mpsc::channel(16);

    let listener = listen(&opts).await;

    let local_addy =
        listener.local_addr().expect("Unable to find local address");

    log::info!("NOODLES server accepting clients @ {local_addy}");

    let stop_watch_task =
        tokio::spawn(shutdown_watcher(bcast_recv, stop_tx.clone()));

    // move the listener off to start accepting clients
    // it needs a handle for the broadcast channel to hand to new clients
    // and a handle to the to_server stream.
    let h1 = tokio::spawn(client_connect_task::<T>(
        listener,
        bcast_send.clone(),
        to_server_send.clone(),
        stop_tx.clone(),
        stop_tx.subscribe(),
    ));

    if let Some(q) = command_queue {
        tokio::spawn(command_sender::<T>(q, to_server_send.clone()));
    }

    // let state_handle = thread::spawn(move || {

    // });

    server_state_loop::<T>(
        bcast_send.clone(),
        init,
        to_server_recv,
        stop_tx.subscribe(),
    )
    .await;

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
        if let Output::Shutdown = msg {
            log::debug!("Server is asking to stop, broadcasting stop bit");
            stop_tx.send(1).unwrap();
            return;
        }
    }
}

async fn command_sender<T>(
    mut command_queue: mpsc::Receiver<T::CommandType>,
    to_server_send: tokio::sync::mpsc::Sender<ToServerMessage<T::CommandType>>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send + Debug + 'static,
{
    log::debug!("Launching command sender");
    while let Some(msg) = command_queue.recv().await {
        to_server_send
            .send(ToServerMessage::Command(msg))
            .await
            .unwrap();
    }
    log::debug!("Done with command sender");
}

// Task to construct a listening socket
async fn listen(opts: &ServerOptions) -> TcpListener {
    TcpListener::bind(&opts.host)
        .await
        .expect("Unable to bind to address")
}

// Task that waits for a new client to connect and spawns a new client
// handler task
async fn client_connect_task<T>(
    listener: TcpListener,
    bcast_send: broadcast::Sender<Output>,
    to_server_send: tokio::sync::mpsc::Sender<ToServerMessage<T::CommandType>>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
    mut stop_rx: tokio::sync::broadcast::Receiver<u8>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send + Debug + 'static,
{
    log::debug!("Starting client connect task");

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            acc = listener.accept() => {
                if let Ok((stream, _)) = acc {
                    tokio::spawn(client_handler::<T>(
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
async fn server_state_loop<T>(
    tx: server_state::CallbackPtr,
    init: T::InitType,
    mut from_world: tokio::sync::mpsc::Receiver<
        ToServerMessage<T::CommandType>,
    >,
    mut stop_rx: tokio::sync::broadcast::Receiver<u8>,
) where
    T: AsyncServer,
    T::CommandType: std::marker::Send,
{
    log::debug!("Starting server state thread");
    let server_state = Arc::new(Mutex::new(T::new(tx, init)));

    server_state.lock().unwrap().initialize_state();

    let mut client_map = HashMap::<uuid::Uuid, ClientRecord>::new();

    loop {
        tokio::select! {
            _ = stop_rx.recv() => break,
            Some(msg) = from_world.recv() => {
                match msg {
                    ToServerMessage::ClientConnected(cr) => {
                        client_map.insert(cr.id, cr.clone());
                        server_state.lock().unwrap().client_connected(cr)
                    }
                    ToServerMessage::Client(client_msg) => {
                        // handle a message from a client, and write any replies
                        // to the client's output queue
                        if log_enabled!(log::Level::Debug) {
                            log::debug!("RECV:");
                            debug_cbor(&client_msg.1);
                        }

                        let client = client_map.get(&client_msg.0).unwrap();

                        let result = {
                            server_state::handle_next(
                                Arc::clone(&server_state),
                                client_msg.1,
                                client_msg.0,
                                |out| {
                                    if log_enabled!(log::Level::Debug) {
                                        log::debug!("SEND TO CLIENT:");
                                        debug_cbor(&out);
                                    }
                                    client.sender.send(out).unwrap();
                                },
                            )
                        };

                        if let Err(x) = result {
                            log::warn!("Unable to handle message from client: {x:?}");
                        }
                    }
                    ToServerMessage::ClientClosed(id) => {
                        server_state.lock().unwrap().client_disconnected(id);
                        client_map.remove(&id);
                    }
                    // ToServerMessage::Shutdown => {
                    //     stop_tx.send(1).unwrap();
                    //     break;
                    // }
                    ToServerMessage::Command(comm_msg) => {
                        server_state.lock().unwrap().handle_command(comm_msg);
                    }
                }
            }
        }
    }

    log::debug!("Ending server state thread");
}

/// Task for each client that has joined up
async fn client_handler<'a, T>(
    stream: TcpStream,
    to_server_send: tokio::sync::mpsc::Sender<ToServerMessage<T::CommandType>>,
    mut bcast_recv: broadcast::Receiver<Output>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
) -> Result<(), ()>
where
    T: AsyncServer,
    T::CommandType: std::marker::Send + Debug + 'a,
{
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
    let (out_tx, mut out_rx) = mpsc::unbounded_channel();

    let mut socket_stopper = stop_tx.subscribe();
    let h1 = tokio::spawn(async move {
        // task that just sends data to the socket
        loop {
            tokio::select! {
                _ = socket_stopper.recv() => break,
                data = out_rx.recv() => {
                    if let Some(data) = data {
                        // for each message, just send it along
                        tx.send(tokio_tungstenite::tungstenite::Message::Binary(data))
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
        tokio::spawn(async move {
            loop {
                tokio::select! {
                        _ = bcast_stopper.recv() => break,
                        bcast = bcast_recv.recv() => {
                // take each message from the broadcast channel and add it to the
                // queue
                            if let Ok(Output::Broadcast(bcast)) = bcast {
                                this_tx.send(bcast).unwrap();
                            }
                        }
                    }
            }

            log::debug!("Ending per-client broadcast-forwarder");
        })
    };

    to_server_send
        .send(ToServerMessage::ClientConnected(ClientRecord {
            id: client_id,
            sender: out_tx.clone(),
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
                        _ => break
                    }
                }
            }
        }
    }

    //stop_tx.send(1).unwrap();

    to_server_send
        .send(ToServerMessage::ClientClosed(client_id))
        .await
        .unwrap();

    log::info!("Closing client, waiting for tasks...");

    h1.await.unwrap();
    h2.await.unwrap();

    log::info!("Client closed.");

    Ok(())
}
