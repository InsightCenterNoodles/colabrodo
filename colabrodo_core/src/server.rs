//! A tokio powered server framework. Users can plug in a user server struct to this framework to obtain a coroutine powered NOODLES server.

use std::thread;

use futures_util::SinkExt;
use futures_util::StreamExt;
use log;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};
use tokio_tungstenite;

use crate::server_state;
use crate::server_state::UserServerState;

pub use tokio;

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
    Client(FromClientMessage),
    Command(T::CommandType),
}

pub struct ServerOptions {
    pub host: String,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            host: "localhost:50000".to_string(),
        }
    }
}

pub struct DefaultCommand {}

pub trait AsyncServer: UserServerState {
    type CommandType;
    fn new(tx: server_state::CallbackPtr) -> Self;
    fn initialize_state(&mut self);

    fn handle_command(&mut self, command: Self::CommandType);
}

/// Public entry point to the server process.
///
/// Note that this function will spawn a number of threads.
///
/// # Example
/// Users should define a type that conforms to UserServerState + AsyncServer and pass it in like:
/// ```
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
/// colabrodo_core::server_tokio::server_main::<MyServer>(opts);
///
/// ```
pub async fn server_main<T>(opts: ServerOptions)
where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send,
{
    server_main_with_command_queue::<T>(opts, None).await;
}

pub async fn server_main_with_command_queue<T>(
    opts: ServerOptions,
    command_queue: Option<mpsc::Receiver<T::CommandType>>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send,
{
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
    tokio::spawn(client_connect_task(
        listener,
        bcast_send.clone(),
        to_server_send.clone(),
    ));

    if let Some(q) = command_queue {
        tokio::spawn(command_sender(q, to_server_send.clone()));
    }

    let state_handle = thread::spawn(move || {
        server_state_loop::<T>(from_server_send, to_server_recv)
    });

    server_message_pump(bcast_send, from_server_recv).await;

    state_handle.join().unwrap();
}

async fn command_sender<T>(
    mut command_queue: mpsc::Receiver<T::CommandType>,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send + 'static,
{
    while let Some(msg) = command_queue.recv().await {
        to_server_send.send(ToServerMessage::Command(msg)).unwrap();
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
async fn client_connect_task<T>(
    listener: TcpListener,
    bcast_send: broadcast::Sender<Vec<u8>>,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
) where
    T: AsyncServer + 'static,
    T::CommandType: std::marker::Send + 'static,
{
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(client_handler(
            stream,
            to_server_send.clone(),
            bcast_send.subscribe(),
        ));
    }
}

/// Broadcast a message from the server outgoing queue to all clients.
async fn server_message_pump(
    bcast_send: broadcast::Sender<Vec<u8>>,
    from_server_recv: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    while let Ok(msg) = from_server_recv.recv() {
        if bcast_send.receiver_count() == 0 {
            continue;
        }

        if bcast_send.send(msg).is_err() {
            log::error!("Internal error: Unable to broadcast message.")
        }
    }
}

/// handles server state; the state itself is not thread safe, so we isolate it
/// to this task.
fn server_state_loop<T>(
    tx: server_state::CallbackPtr,
    from_world: std::sync::mpsc::Receiver<ToServerMessage<T>>,
) where
    T: AsyncServer,
    T::CommandType: std::marker::Send,
{
    let mut server_state = T::new(tx);

    server_state.initialize_state();

    while let Ok(msg) = from_world.recv() {
        match msg {
            ToServerMessage::Client(client_msg) => {
                // handle a message from a client, and write any replies
                // to the client's output queue
                // print!("RECV:");
                //debug_cbor(&msg.1);
                let result = server_state::handle_next(
                    &mut server_state,
                    client_msg.1,
                    |out| {
                        //print!("SEND CLIENT:");
                        //debug_cbor(&out);
                        client_msg.0.blocking_send(out).unwrap();
                    },
                );

                if let Err(x) = result {
                    log::warn!("Unable to handle message from client: {x:?}");
                }
            }
            ToServerMessage::Command(comm_msg) => {
                server_state.handle_command(comm_msg);
            }
        }
    }
}

/// Task for each client that has joined up
async fn client_handler<'a, T>(
    stream: TcpStream,
    to_server_send: std::sync::mpsc::Sender<ToServerMessage<T>>,
    mut bcast_recv: broadcast::Receiver<Vec<u8>>,
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

    tokio::spawn(async move {
        // task that just sends data to the socket
        while let Some(data) = out_rx.recv().await {
            // for each message, just send it along
            tx.send(tokio_tungstenite::tungstenite::Message::Binary(data))
                .await
                .unwrap();
        }
    });

    // task that takes broadcast information and sends it to the out queue
    {
        let this_tx = out_tx.clone();
        tokio::spawn(async move {
            // take each message from the broadcast channel and add it to the
            // queue
            while let Ok(bcast) = bcast_recv.recv().await {
                this_tx.send(bcast).await.unwrap();
            }
        });
    }

    // handle recv of any data, and forward on to the server
    while let Some(message) = rx.next().await {
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

    log::info!("Client closed.");

    Ok(())
}
