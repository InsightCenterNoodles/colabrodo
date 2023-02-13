//! A tokio powered server framework. Users can plug in a user server struct to this framework to obtain a coroutine powered NOODLES server.

use std::thread;

use futures_util::SinkExt;
use futures_util::StreamExt;
use log;
use tokio::runtime;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};
use tokio_tungstenite;

use crate::server_state;
use crate::server_state::UserServerState;

// We have a fun structure here.
// First there is a thread for handling the server state, which is controlled through queues.
// We then have a task that takes broadcast messages from the server thread and pumps it to clients through a tokio bcast queue. Using the bcast queue directly, as we get into issues of thread sync; the server is not thread safe and any attempt to use await in regards to the server might cause it to cross thread boundaries. Tokio lets you lock a tast to a thread, but it is extremely non-pleasant to structure this to do so.
// We have a task that listens for clients, that spawns a per client task.

/// Dump cbor to the terminal in both diagnostic and in byte form for debugging.
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

/// Public entry point to the server process.
///
/// Note that this function will spawn a number of threads.
///
/// # Example
/// Users should define a type that conforms to UserServerState and pass it in like:
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
///
/// colabrodo_core::server_tokio::server_main::<MyServer>();
///
/// ```
pub fn server_main<T>()
where
    T: UserServerState,
{
    let rt = runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    // channel for messages to be sent to all clients
    let (bcast_send, _) = broadcast::channel(16);

    // channel for server to recv messages
    let (to_server_send, to_server_recv) = std::sync::mpsc::channel();

    // channel for server to send messages to all clients
    let (from_server_send, from_server_recv) = std::sync::mpsc::channel();

    let listener = rt.block_on(listen());

    log::info!("Listening...");

    rt.spawn(client_connect_task(
        listener,
        bcast_send.clone(),
        to_server_send,
    ));

    let state_handle = thread::spawn(move || {
        server_state_loop::<T>(from_server_send, to_server_recv)
    });

    rt.block_on(server_message_pump(bcast_send, from_server_recv));

    state_handle.join().unwrap();
}

// Task to construct a listening socket
async fn listen() -> TcpListener {
    let addr = "localhost:50000".to_string();

    TcpListener::bind(&addr)
        .await
        .expect("Unable to bind to address")
}

// Task that waits for a new client to connect and spawns a new client
// handler task
async fn client_connect_task(
    listener: TcpListener,
    bcast_send: broadcast::Sender<Vec<u8>>,
    to_server_send: std::sync::mpsc::Sender<FromClientMessage>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(client_handler(
            stream,
            to_server_send.clone(),
            bcast_send.subscribe(),
        ));
    }
}

async fn server_message_pump(
    bcast_send: broadcast::Sender<Vec<u8>>,
    from_server_recv: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    while let Ok(msg) = from_server_recv.recv() {
        if bcast_send.receiver_count() == 0 {
            continue;
        }

        if bcast_send.send(msg).is_err() {
            println!("Internal error: Unable to broadcast message.")
        }
    }
}

/// handles server state; the state itself is not thread safe, so we isolate it
/// to this task.
fn server_state_loop<T>(
    tx: server_state::CallbackPtr,
    from_clients: std::sync::mpsc::Receiver<FromClientMessage>,
) where
    T: UserServerState,
{
    let mut server_state = T::new(tx);

    server_state.initialize_state();

    while let Ok(msg) = from_clients.recv() {
        // handle a message from a client, and write any replies
        // to the client's output queue
        print!("RECV:");
        debug_cbor(&msg.1);
        let result =
            server_state::handle_next(&mut server_state, msg.1, |out| {
                print!("SEND CLIENT:");
                debug_cbor(&out);
                msg.0.blocking_send(out).unwrap();
            });

        if let Err(x) = result {
            log::warn!("Unable to handle message from client: {x:?}");
        }
    }
}

/// Task for each client that has joined up
async fn client_handler(
    stream: TcpStream,
    to_server_send: std::sync::mpsc::Sender<FromClientMessage>,
    mut bcast_recv: broadcast::Receiver<Vec<u8>>,
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
        let message = message.unwrap();

        if message.is_binary() {
            to_server_send
                .send(FromClientMessage(out_tx.clone(), message.into_data()))
                .unwrap();
        }
    }

    println!("Client closed.");

    Ok(())
}
