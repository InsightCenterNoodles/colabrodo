//! Tools to serve binary assets over http

use std::net::SocketAddr;
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

impl Asset {
    pub fn new_from_slice(bytes: &[u8]) -> Self {
        Self::InMemory(Bytes::copy_from_slice(bytes))
    }
}

/// Generate a new asset identifier.
pub fn create_asset_id() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// Storage for all known assets
struct AssetStore {
    host_name: String,
    asset_list: HashMap<uuid::Uuid, Arc<Asset>>,
}

impl AssetStore {
    fn new(p: u16) -> Self {
        let hname = hostname::get().unwrap();

        let hname = hname.to_string_lossy();

        let hname = format!("http://{hname}:{p}/");

        Self {
            host_name: hname,
            asset_list: HashMap::new(),
        }
    }

    fn hostname(&self) -> &String {
        &self.host_name
    }

    /// Insert an asset into the store
    fn add_asset(&mut self, id: uuid::Uuid, asset: Asset) -> String {
        log::info!("Adding web asset {id}");
        self.asset_list.insert(id, Arc::new(asset));
        format!("{}{}", self.host_name, id)
    }

    /// Remove an asset from the store
    fn delete_asset(&mut self, id: &uuid::Uuid) {
        log::info!("Removing web asset {id}");
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
    host: String,
}

impl Default for AssetServerOptions {
    fn default() -> Self {
        Self {
            host: "0.0.0.0:50001".to_string(),
        }
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
    Registered(uuid::Uuid, String),
    Unregistered(uuid::Uuid),
    ShuttingDown,
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
    let addr: SocketAddr = options.host.parse().unwrap();

    let state = Arc::new(Mutex::new(AssetStore::new(addr.port())));

    {
        let state = state.lock().unwrap();
        log::info!("Serving binary assets on {}", state.hostname());
    }

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
                            let url;
                            {
                                let mut st = state.lock().unwrap();
                                url = st.add_asset(id, asset);
                            }
                            command_replies
                                .send(AssetServerReply::Registered(id, url))
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
                            command_replies
                                .send(AssetServerReply::ShuttingDown)
                                .await
                                .unwrap();
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

pub struct AssetServerLink {
    tx_to_server: mpsc::Sender<AssetServerCommand>,
    rx_from_server: mpsc::Receiver<AssetServerReply>,
}

impl AssetServerLink {
    pub async fn wait_for_start(&mut self) {
        let get_http_startup = self.rx_from_server.recv().await.unwrap();

        match get_http_startup {
            AssetServerReply::Started => (),
            _ => {
                panic!("HTTP Server could not start.");
            }
        }
    }

    pub fn send_command(
        &mut self,
        command: AssetServerCommand,
    ) -> AssetServerReply {
        self.tx_to_server.blocking_send(command).unwrap();

        self.rx_from_server.blocking_recv().unwrap()
    }

    pub async fn send_command_async(
        &mut self,
        command: AssetServerCommand,
    ) -> AssetServerReply {
        self.tx_to_server.send(command).await.unwrap();

        // Now the smart thing would be to figure out if responses are correlated to replies, but as this is a single sender, single recv situation, we can skip that for the moment
        // TODO: robustify request and recv.

        self.rx_from_server.recv().await.unwrap()
    }

    pub fn add_asset(&mut self, id: uuid::Uuid, asset: Asset) -> String {
        let reply = self.send_command(AssetServerCommand::Register(id, asset));

        match reply {
            AssetServerReply::Registered(rid, url) => {
                assert_eq!(rid, id);
                return url;
            }
            _ => {
                panic!("Unexpected message!");
            }
        }
    }

    pub async fn add_asset_async(
        &mut self,
        id: uuid::Uuid,
        asset: Asset,
    ) -> String {
        let reply = self
            .send_command_async(AssetServerCommand::Register(id, asset))
            .await;

        match reply {
            AssetServerReply::Registered(rid, url) => {
                assert_eq!(rid, id);
                return url;
            }
            _ => {
                panic!("Unexpected message!");
            }
        }
    }

    pub fn remove_asset(&mut self, id: uuid::Uuid) {
        let reply = self.send_command(AssetServerCommand::Unregister(id));

        match reply {
            AssetServerReply::Unregistered(rid) => {
                assert_eq!(rid, id);
            }
            _ => {
                panic!("Unexpected message!");
            }
        }
    }

    pub fn shutdown(&mut self) {
        let reply = self.send_command(AssetServerCommand::Shutdown);

        match reply {
            AssetServerReply::ShuttingDown => {
                return;
            }
            _ => {
                panic!("Unexpected message!");
            }
        }
    }

    pub async fn shutdown_async(&mut self) {
        let reply = self.send_command_async(AssetServerCommand::Shutdown).await;

        match reply {
            AssetServerReply::ShuttingDown => {
                return;
            }
            _ => {
                panic!("Unexpected message!");
            }
        }
    }
}

/// Make an asset server with some channels already set up.
pub fn make_asset_server(
    options: AssetServerOptions,
) -> (impl futures_util::Future<Output = ()>, AssetServerLink) {
    let (tx_to_server, rx_to_server) = mpsc::channel(16);
    let (tx_from_server, rx_from_server) = mpsc::channel(16);

    (
        run_asset_server(options, rx_to_server, tx_from_server),
        AssetServerLink {
            tx_to_server,
            rx_from_server,
        },
    )
}

#[cfg(test)]
mod tests {
    use hyper::{
        body::{Bytes, HttpBody},
        Client, Result,
    };

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
        let (asset_server, mut link) = make_asset_server(AssetServerOptions {
            host: "127.0.0.1".to_string(),
            ..Default::default()
        });

        let new_id = create_asset_id();

        tokio::spawn(async move {
            link.wait_for_start().await;

            let url = link
                .add_asset_async(
                    new_id,
                    Asset::InMemory(Bytes::from(vec![10, 20, 30, 40])),
                )
                .await;

            let content = fetch_url(url.parse().unwrap()).await.unwrap();

            assert_eq!(content, vec![10, 20, 30, 40]);

            link.shutdown_async().await;
        });

        asset_server.await;
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
