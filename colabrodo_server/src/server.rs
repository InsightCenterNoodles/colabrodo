//! A tokio powered server framework. Users can plug in a user server struct to this framework to obtain a coroutine powered NOODLES server.

use std::thread;

use futures_util::SinkExt;
use futures_util::StreamExt;
use log;
use log::log_enabled;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};
use tokio_tungstenite;

use crate::server_state;
use crate::server_state::Output;
use crate::server_state::UserServerState;

pub use tokio;

pub use ciborium;

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

/// Models a message from a client. As the server thread doesn't know who sent it, we pass along a lightweight handle to the client-specific queue so the server can send specific messages along
#[derive(Debug)]
struct FromClientMessage(mpsc::Sender<Vec<u8>>, Vec<u8>);

enum ToServerMessage<T>
where
    T: AsyncServer,
{
    ClientConnected,
    Client(FromClientMessage),
    ClientClosed,
    Shutdown,
    Command(T::CommandType),
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

/// A placeholder if the user server doesn't support commands
pub struct DefaultCommand {}

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
    fn initialize_state(&mut self);

    /// Called when a new client connects. This is before any introduction is received
    fn client_connected(&mut self) {}

    /// Called when a client disconnects. This is purely informative.
    fn client_disconnected(&mut self) {}

    /// Called when an async command is received
    fn handle_command(&mut self, command: Self::CommandType);
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
    T::CommandType: std::marker::Send,
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
    T::CommandType: std::marker::Send,
    <T as AsyncServer>::InitType: std::marker::Send,
{
    // channel for task control
    let (stop_tx, _stop_rx) = tokio::sync::broadcast::channel::<u8>(1);

    // channel for messages to be sent to all clients
    let (bcast_send, _) = broadcast::channel(16);

    // channel for server to recv messages
    let (to_server_send, to_server_recv) = std::sync::mpsc::channel();

    // channel for server to send messages to all clients
    let (from_server_send, from_server_recv) = std::sync::mpsc::channel();

    let listener = listen(&opts).await;

    let local_addy =
        listener.local_addr().expect("Unable to find local address");

    log::info!("NOODLES server accepting clients @ {local_addy}");

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

    if let Some(q) = command_queue {
        tokio::spawn(command_sender(q, to_server_send.clone()));
    }

    let state_handle = thread::spawn(move || {
        server_state_loop::<T>(from_server_send, init, to_server_recv, stop_tx)
    });

    server_message_pump(bcast_send, from_server_recv, to_server_send.clone())
        .await;

    h1.await.unwrap();

    state_handle.join().unwrap();
}

async fn command_sender<T>(
    mut command_queue: mpsc::Receiver<T::CommandType>,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send + 'static,
{
    log::debug!("Launching command sender");
    while let Some(msg) = command_queue.recv().await {
        to_server_send.send(ToServerMessage::Command(msg)).unwrap();
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
    bcast_send: broadcast::Sender<Vec<u8>>,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
    mut stop_rx: tokio::sync::broadcast::Receiver<u8>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send + 'static,
{
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

/// Broadcast a message from the server outgoing queue to all clients.
async fn server_message_pump<T>(
    bcast_send: broadcast::Sender<Vec<u8>>,
    from_server_recv: std::sync::mpsc::Receiver<Output>,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
) where
    T: AsyncServer + 'static,
{
    log::debug!("Starting server message pump");
    while let Ok(msg) = from_server_recv.recv() {
        if bcast_send.receiver_count() == 0 {
            continue;
        }

        match msg {
            Output::Broadcast(msg) => {
                if bcast_send.send(msg).is_err() {
                    log::error!("Internal error: Unable to broadcast message.")
                }
            }
            Output::Shutdown => {
                log::debug!("Server sent a shutdown, broadcasting stop.");
                to_server_send.send(ToServerMessage::Shutdown).unwrap();
                break;
            }
        }
    }
    log::debug!("Stopping server message pump");
}

/// handles server state; the state itself is not thread safe, so we isolate it
/// to this task.
fn server_state_loop<T>(
    tx: server_state::CallbackPtr,
    init: T::InitType,
    from_world: std::sync::mpsc::Receiver<ToServerMessage<T>>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
) where
    T: AsyncServer,
    T::CommandType: std::marker::Send,
{
    log::debug!("Starting server state thread");
    let mut server_state = T::new(tx, init);

    server_state.initialize_state();

    while let Ok(msg) = from_world.recv() {
        match msg {
            ToServerMessage::ClientConnected => server_state.client_connected(),
            ToServerMessage::Client(client_msg) => {
                // handle a message from a client, and write any replies
                // to the client's output queue
                if log_enabled!(log::Level::Debug) {
                    log::debug!("RECV:");
                    debug_cbor(&client_msg.1);
                }
                let result = server_state::handle_next(
                    &mut server_state,
                    client_msg.1,
                    |out| {
                        if log_enabled!(log::Level::Debug) {
                            log::debug!("SEND TO CLIENT:");
                            debug_cbor(&out);
                        }
                        client_msg.0.blocking_send(out).unwrap();
                    },
                );

                if let Err(x) = result {
                    log::warn!("Unable to handle message from client: {x:?}");
                }
            }
            ToServerMessage::ClientClosed => {
                server_state.client_disconnected();
            }
            ToServerMessage::Shutdown => {
                stop_tx.send(1).unwrap();
                break;
            }
            ToServerMessage::Command(comm_msg) => {
                server_state.handle_command(comm_msg);
            }
        }
    }

    log::debug!("Ending server state thread");
}

/// Task for each client that has joined up
async fn client_handler<'a, T>(
    stream: TcpStream,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
    mut bcast_recv: broadcast::Receiver<Vec<u8>>,
    stop_tx: tokio::sync::broadcast::Sender<u8>,
) -> Result<(), ()>
where
    T: AsyncServer,
    T::CommandType: std::marker::Send + 'a,
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

    // tx an rx are the core websocket channels
    let (mut tx, mut rx) = websocket.split();

    // set up a queue for messages to be sent to the socket
    // probably more elegant ways of doing this, but this seems to be
    // an appropriate way to merge messages from a broadcast and possible
    // single-client replies
    let (out_tx, mut out_rx) = mpsc::channel(16);

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
                            if let Ok(bcast) = bcast {
                                this_tx.send(bcast).await.unwrap();
                            }
                        }
                    }
            }

            log::debug!("Ending per-client broadcast-forwarder");
        })
    };

    to_server_send
        .send(ToServerMessage::ClientConnected)
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
                            return Ok(());
                        }
                    };

                    if message.is_binary() {
                        to_server_send
                            .send(ToServerMessage::Client(FromClientMessage(
                                out_tx.clone(),
                                message.into_data(),
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }

    to_server_send.send(ToServerMessage::ClientClosed).unwrap();

    log::info!("Closing client, waiting for tasks...");

    h1.await.unwrap();
    h2.await.unwrap();

    log::info!("Client closed.");

    Ok(())
}
