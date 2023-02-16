//! Tools to serve binary assets over http

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

use hyper::body::Bytes;
use hyper::Server;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Result, StatusCode,
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use tokio::sync::broadcast;
use tokio::sync::mpsc;

/// An asset to be served. Assets can either be in-memory, or on-disk.
#[derive(Debug)]
pub enum Asset {
    InMemory(Bytes),
    OnDisk(PathBuf),
}

/// Generate a new asset identifier.
pub fn create_asset_id() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// Storage for all known assets
#[derive(Default)]
struct AssetStore {
    asset_list: HashMap<uuid::Uuid, Arc<Asset>>,
}

impl AssetStore {
    /// Insert an asset into the store
    fn add_asset(&mut self, id: uuid::Uuid, asset: Asset) {
        self.asset_list.insert(id, Arc::new(asset));
    }

    /// Remove an asset from the store
    fn delete_asset(&mut self, id: &uuid::Uuid) {
        self.asset_list.remove(id);
    }

    /// Handle a request; find asset if it exists.
    fn on_request(&mut self, req: Request<Body>) -> Option<Arc<Asset>> {
        if req.method() != Method::GET {
            return None;
        }

        // method is a get. find the asset

        let id = req.uri().path().strip_prefix('/')?;

        let id = uuid::Uuid::parse_str(id).ok()?;

        self.asset_list.get(&id).cloned()
    }
}

/// Helper to create a page-not-found response code
fn make_not_found_code() -> Response<Body> {
    static NOTFOUND: &[u8] = b"Not Found";

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

/// Send bytes from an asset
async fn send_bytes(bytes: Bytes) -> Result<Response<Body>> {
    Ok(Response::new(Body::from(bytes)))
}

/// Send file bytes from an asset
async fn send_file(path: &Path) -> Result<Response<Body>> {
    if let Ok(file) = File::open(path).await {
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        return Ok(Response::new(body));
    }

    Ok(make_not_found_code())
}

/// Send bytes from an asset
async fn fetch_asset(asset: Arc<Asset>) -> Result<Response<Body>> {
    match asset.as_ref() {
        Asset::InMemory(data) => send_bytes(data.clone()).await,
        Asset::OnDisk(path) => send_file(path.as_path()).await,
    }
}

/// Handle a get request
async fn handle_request(
    req: Request<Body>,
    state: Arc<Mutex<AssetStore>>,
) -> Result<Response<Body>> {
    let asset = state.lock().unwrap().on_request(req);

    if let Some(asset) = asset {
        return fetch_asset(asset).await;
    }
    Ok(make_not_found_code())
}

/// Options for the asset server
pub struct AssetServerOptions {
    port: u16,
}

impl Default for AssetServerOptions {
    fn default() -> Self {
        Self { port: 50001 }
    }
}

/// Commands for the asset server
#[derive(Debug)]
pub enum AssetServerCommand {
    Register(uuid::Uuid, Asset),
    Unregister(uuid::Uuid),
    Shutdown,
}

/// Replies to commands
#[derive(Debug)]
pub enum AssetServerReply {
    Registered(uuid::Uuid),
    Unregistered(uuid::Uuid),
    Started,
}

/// Create a new asset server.
///
/// This constructs an asset server on the given port. The server takes a stream of commands for registering and removing assets from the store. Responses to those commands are sent back on the reply stream. Note that it is assumed that only one task will be sending and receiving commands.
///
/// The function will return on an error, or after the [AssetServerCommand::Shutdown] command has been sent.
pub async fn run_asset_server(
    options: AssetServerOptions,
    command_stream: mpsc::Receiver<AssetServerCommand>,
    command_replies: mpsc::Sender<AssetServerReply>,
) {
    let addr = ([127, 0, 0, 1], options.port).into();

    let state = Arc::new(Mutex::new(AssetStore::default()));

    let (bcast_tx, mut bcast_rx) = broadcast::channel(2);

    tokio::spawn(command_handler(
        command_stream,
        command_replies.clone(),
        bcast_tx.subscribe(),
        bcast_tx,
        state.clone(),
    ));

    let make_service = make_service_fn(move |_| {
        let c = state.clone();

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_request(req, c.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    log::info!("Serving binary assets on {addr}");

    command_replies
        .send(AssetServerReply::Started)
        .await
        .unwrap();

    tokio::select! {
        e = server => {
            if let Err(e) = e {
                log::error!("HTTP server error: {e}");
            }
        },
        _shutdown_c = bcast_rx.recv() => {
        }
    }
}

/// Task to handle commands
async fn command_handler(
    mut command_stream: mpsc::Receiver<AssetServerCommand>,
    command_replies: mpsc::Sender<AssetServerReply>,
    mut bcast_rx: broadcast::Receiver<u8>,
    bcast_tx: broadcast::Sender<u8>,
    state: Arc<Mutex<AssetStore>>,
) {
    loop {
        tokio::select! {
            msg = command_stream.recv() => {
                if let Some(msg) = msg {
                    match msg {
                        AssetServerCommand::Register(id, asset) => {
                            {
                                let mut st = state.lock().unwrap();
                                st.add_asset(id, asset);
                            }
                            command_replies
                                .send(AssetServerReply::Registered(id))
                                .await
                                .unwrap();
                        }
                        AssetServerCommand::Unregister(id) => {
                            {
                                let mut st = state.lock().unwrap();
                                st.delete_asset(&id);
                            }

                            command_replies
                                .send(AssetServerReply::Unregistered(id))
                                .await
                                .unwrap();
                        }
                        AssetServerCommand::Shutdown => {
                            bcast_tx.send(1).unwrap();
                        }
                    }
                }
            },
            _shutdown_c = bcast_rx.recv() => {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use hyper::{
        body::{Bytes, HttpBody},
        Client, Result, Uri,
    };
    use tokio::sync::mpsc;

    use super::*;

    async fn fetch_url(url: hyper::Uri) -> Result<Vec<u8>> {
        let client = Client::new();

        let mut res = client.get(url).await?;

        let mut ret = Vec::new();

        while let Some(next) = res.data().await {
            let chunk = next?;

            ret.extend_from_slice(&chunk);
        }

        Ok(ret)
    }

    async fn simple_structure_main() {
        let (tx_to_server, rx_to_server) = mpsc::channel(16);
        let (tx_from_server, mut rx_from_server) = mpsc::channel(16);

        let new_id = create_asset_id();

        tx_to_server
            .send(AssetServerCommand::Register(
                new_id,
                Asset::InMemory(Bytes::from(vec![10, 20, 30, 40])),
            ))
            .await
            .unwrap();

        tokio::spawn(async move {
            let first = rx_from_server.recv().await.unwrap();

            match first {
                AssetServerReply::Started => (),
                _ => {
                    panic!("Wrong first message");
                }
            }

            let second = rx_from_server.recv().await.unwrap();

            match second {
                AssetServerReply::Registered(id) => {
                    assert_eq!(id, new_id)
                }
                _ => {
                    panic!("Wrong second message");
                }
            }

            let target = format!("http://localhost:50001/{new_id}");

            let target: Uri = target.parse().unwrap();

            let content = fetch_url(target).await.unwrap();

            assert_eq!(content, vec![10, 20, 30, 40]);

            tx_to_server
                .send(AssetServerCommand::Shutdown)
                .await
                .unwrap();
        });

        run_asset_server(
            AssetServerOptions::default(),
            rx_to_server,
            tx_from_server,
        )
        .await;
    }

    #[test]
    fn basic_asset_publish() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(simple_structure_main())
    }
}
