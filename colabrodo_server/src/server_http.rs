//! Tools to serve binary assets over HTTP.
//!
//! See the cube_http example to see how to use this module.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

use colabrodo_common::network::determine_ip_address;
use hyper::body::Bytes;
use hyper::http::HeaderValue;
use hyper::Server;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Result, StatusCode,
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use tokio::sync::broadcast;

pub use uuid;

use crate::server::ServerOptions;

/// An asset to be served. Assets can either be in-memory, or on-disk.
#[derive(Debug)]
pub enum Asset {
    InMemory(Bytes),
    OnDisk(PathBuf),
}

impl Asset {
    /// Create a new in-memory asset from a byte slice
    pub fn new_from_slice(bytes: &[u8]) -> Self {
        Self::InMemory(Bytes::copy_from_slice(bytes))
    }
}

/// Generate a new asset identifier.
pub fn create_asset_id() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// Storage for assets to be served
pub struct AssetStore {
    stop_tx: broadcast::Sender<u8>,
    url_prefix: String,
    asset_list: HashMap<uuid::Uuid, Arc<Asset>>,
}

impl AssetStore {
    fn new(options: &AssetServerOptions) -> Self {
        let hname = options
            .external_host
            .clone()
            .or(determine_ip_address())
            .unwrap();

        let hname =
            format!("http://{hname}:{}/", options.url.port().unwrap_or(50001));

        let (stop_tx, _) = broadcast::channel(2);

        Self {
            stop_tx,
            url_prefix: hname,
            asset_list: HashMap::new(),
        }
    }

    fn hostname(&self) -> &String {
        &self.url_prefix
    }

    /// Insert an asset into the store
    fn add_asset(&mut self, id: uuid::Uuid, asset: Asset) -> String {
        log::info!("Adding web asset {id}");
        self.asset_list.insert(id, Arc::new(asset));
        format!("{}{}", self.url_prefix, id)
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

/// Shared pointer to an asset store
pub type AssetStorePtr = Arc<Mutex<AssetStore>>;

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

/// Add CORS header
fn update_headers<T>(res: &mut Response<T>) {
    res.headers_mut().insert(
        hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str("*").unwrap(),
    );
}

/// Handle a get request
async fn handle_request(
    req: Request<Body>,
    state: Arc<Mutex<AssetStore>>,
) -> Result<Response<Body>> {
    let asset = state.lock().unwrap().on_request(req);

    if let Some(asset) = asset {
        let ret = fetch_asset(asset).await.map(|mut f| {
            update_headers(&mut f);
            f
        });
        return ret;
    }
    Ok(make_not_found_code())
}

/// Options for the asset server
pub struct AssetServerOptions {
    pub url: url::Url,
    pub external_host: Option<String>,
}

impl AssetServerOptions {
    pub fn new(opts: &ServerOptions) -> Self {
        let mut asset_url = opts.host.clone();
        asset_url
            .set_port(Some(asset_url.port().unwrap_or(50000) + 1))
            .unwrap();

        Self {
            url: asset_url,
            external_host: None,
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
async fn run_asset_server(state: AssetStorePtr, addr: SocketAddr) {
    let mut stop_rx = {
        let state = state.lock().unwrap();
        log::info!("Serving binary assets on {}", state.hostname());
        state.stop_tx.subscribe()
    };

    let make_service = make_service_fn(move |_| {
        let c = state.clone();

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_request(req, c.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    // command_replies.send(AssetServerReply::Started).unwrap();

    tokio::select! {
        e = server => {
            if let Err(e) = e {
                log::error!("HTTP server error: {e}");
            }
        },
        _shutdown_c = stop_rx.recv() => {
        }
    }

    log::debug!("Asset server shut down.");
}

/// Add an asset to a given store
pub fn add_asset(store: AssetStorePtr, id: uuid::Uuid, asset: Asset) -> String {
    let mut st = store.lock().unwrap();
    st.add_asset(id, asset)
}

/// Remove an asset from a given store
pub fn remove_asset(store: AssetStorePtr, id: uuid::Uuid) {
    let mut st = store.lock().unwrap();
    st.delete_asset(&id);
}

/// Shutdown a given store
pub fn shutdown(store: AssetStorePtr) {
    let st = store.lock().unwrap();
    st.stop_tx.send(1).unwrap();
}

/// Make an asset server, and launch the web handler
pub fn make_asset_server(options: AssetServerOptions) -> AssetStorePtr {
    let addr = SocketAddr::new(
        options.url.host_str().unwrap().parse().unwrap(),
        options.url.port().unwrap_or(50001),
    );

    let state = Arc::new(Mutex::new(AssetStore::new(&options)));

    tokio::spawn(run_asset_server(state.clone(), addr));

    state
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
        let asset_server = make_asset_server(AssetServerOptions {
            url: "http://127.0.0.1:50001/".parse().unwrap(),
            external_host: Some("127.0.0.1".to_string()),
        });

        let new_id = create_asset_id();

        let url = add_asset(
            asset_server.clone(),
            new_id,
            Asset::InMemory(Bytes::from(vec![10, 20, 30, 40])),
        );

        println!("Server should have asset at {url}");

        let mut counter = 0;

        let content = loop {
            if let Ok(content) = fetch_url(url.parse().unwrap()).await {
                break content;
            }

            tokio::time::sleep(Duration::from_millis(2000)).await;

            counter += 1;

            if counter == 5 {
                panic!("Unable to fetch from server!")
            }

            println!("Unable to fetch, trying again...");
        };

        assert_eq!(content, vec![10, 20, 30, 40]);

        shutdown(asset_server);
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
