//!
//! Reads a NOODLES message, and replays it
//!

mod replay_client;
mod replay_server;
mod replay_state;

use crate::replay_state::*;
use clap::Parser;
use colabrodo_server::{server::*, server_http::*};
use replay_server::setup_server;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct CLIArgs {
    /// Server hostname
    #[arg(default_value = "ws://localhost:50000")]
    url: url::Url,

    /// Session to replay
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    session_name: PathBuf,

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

    log::info!("Input: {}", cli_args.session_name.display());

    let asset_server = make_asset_server(AssetServerOptions::default());

    let noo_server_opts = ServerOptions::default();

    let state = ServerState::new();

    let replayer_server_state = ReplayerServerState::new(
        cli_args.session_name,
        state.clone(),
        asset_server,
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime
        .block_on(server_task(noo_server_opts, state, replayer_server_state))
        .unwrap();
}

async fn server_task(
    noo_server_opts: ServerOptions,
    state: ServerStatePtr,
    app: ReplayerStatePtr,
) -> anyhow::Result<()> {
    setup_server(state.clone(), app);

    server_main(noo_server_opts, state).await;

    Ok(())
}
