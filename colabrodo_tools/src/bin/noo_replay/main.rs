//!
//! Reads a NOODLES message, and replays it
//!

mod replay_client;
mod replay_server;
mod replay_state;

use crate::replay_state::*;
use clap::Parser;
use colabrodo_server::{
    server::ciborium::value::Value, server::*, server_bufferbuilder::*,
    server_http::*, server_messages::*,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

    let replayer_server_state = ReplayerServerState::new(cli_args.session_name);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime
        .block_on(server_task(replayer_server_state))
        .unwrap();
}

async fn server_task(app: ReplayerStatePtr) -> anyhow::Result<()> {
    let asset_server = make_asset_server(AssetServerOptions::default());

    let noo_server_opts = ServerOptions::default();

    let state = ServerState::new();

    server_main(noo_server_opts, state).await;

    return Ok(());
}
