use std::sync::{Arc, Mutex, Weak};

use clap::Parser;
use colabrodo_client::client::{
    launch_client_worker_thread, start_client_stream,
};
use colabrodo_server::server::*;
use colabrodo_server::server_state::{ServerState, ServerStatePtr};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct CLIArgs {
    /// Server hostnames
    urls: Vec<url::Url>,

    /// Host the server on which address?
    #[arg(short, long)]
    server_url: Option<url::Url>,

    /// Debug mode
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let cli_args = CLIArgs::parse();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    if cli_args.debug {
        std::env::set_var("RUST_LOG", "debug")
    }

    env_logger::init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime.block_on(server_task(cli_args)).unwrap();
}

async fn server_task(args: CLIArgs) -> anyhow::Result<()> {
    let noo_server_opts = args
        .server_url
        .as_ref()
        .map(|f| ServerOptions { host: f.clone() })
        .unwrap_or_else(ServerOptions::default);

    let state = ServerState::new();

    let _ = MergerServerState::new(args, state.clone()).await;

    server_main(noo_server_opts, state).await;

    Ok(())
}

// =============================================================================

struct MergerServerState {}

async fn make_client(
    url: url::Url,
    server: Weak<Mutex<ServerState>>,
) -> Option<std::thread::JoinHandle<()>> {
    let channels = start_client_stream(url, "NOODLES Merge Client".into())
        .await
        .ok()?;
    let maker = replay::ReplayDelegateMaker {
        server_link: server,
    };
    Some(launch_client_worker_thread(channels, maker))
}

impl MergerServerState {
    async fn new(
        args: CLIArgs,
        server_state: ServerStatePtr,
    ) -> Arc<Mutex<Self>> {
        let mut clients = Vec::new();

        for u in args.urls {
            clients.push(
                make_client(
                    u,
                    Arc::<std::sync::Mutex<ServerState>>::downgrade(
                        &server_state,
                    ),
                )
                .await,
            )
        }

        Arc::new(Mutex::new(Self {}))
    }
}
