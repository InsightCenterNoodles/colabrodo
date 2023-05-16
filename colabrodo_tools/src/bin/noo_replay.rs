//!
//! Reads a NOODLES message, and replays it
//!

use clap::Parser;
use colabrodo_common::recording::{parse_record, Packet};
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

pub struct ReplayerState {
    // we are going to just read everything in as is for now.
    packets: Vec<Packet>,
    marker_table: HashMap<String, u32>,

    last_packet: u64,
}

type ReplayerStatePtr = Arc<Mutex<ReplayerState>>;

impl ReplayerState {
    fn new(path: std::path::PathBuf) -> Arc<Mutex<Self>> {
        let in_file =
            std::fs::File::open(path).expect("Unable to open session file.");

        let in_stream = std::io::BufReader::new(in_file);

        // not the best, but for now...
        let raw_packet_vec: Vec<Value> = ciborium::de::from_reader(in_stream)
            .expect("Unable to decode record buffer");

        let packet_vec: Vec<_> = raw_packet_vec
            .into_iter()
            .filter_map(|f| parse_record(f))
            .collect();

        let mut table: HashMap<String, u32> = HashMap::new();

        for packet in &packet_vec {
            match packet {
                Packet::DropMarker(time, name) => {
                    table.insert(name.clone(), *time)
                }
                Packet::WriteCBOR(_, _) => todo!(),
                Packet::BufferLocation(_) => todo!(),
            };
        }

        Arc::new(Mutex::new(Self {
            packets: packet_vec,
            marker_table: table,
            last_packet: 0,
        }))
    }
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

    let in_file = ReplayerState::new(cli_args.session_name);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime.block_on(server_task(in_file)).unwrap();
}

async fn server_task(app: ReplayerStatePtr) -> anyhow::Result<()> {
    // let asset_server = make_asset_server(AssetServerOptions::default());

    // let noo_server_opts = ServerOptions::default();

    // let mut state = ServerState::new();

    // let (message_tx, mut message_rx) = tokio::sync::mpsc::unbounded_channel();

    // tokio::spawn(noodles_task(server, message_tx.clone()));

    // std::thread::spawn(|| stdin_task(message_tx));

    // let mut outfile = Outfile::new(&path);

    // while let Some(m) = message_rx.recv().await {
    //     outfile.write(m)
    // }

    Ok(())
}

make_method_function!(advance_time,
    ReplayerState,
    "advance_time",
    "Advance the replay by either a number of seconds, or to a marker",
    | to_time : Value : "Seconds or marker name" |,
    {
        match to_time {
            Value::Integer(x) => {
                advance_by_time( i128::from(x) as u64 , state, app)
            },
            Value::Text(x) => {
                advance_by_marker(x, state, app)
            }
            _ => ()
        }
        Ok(None)
    });

fn advance_by_time(
    seconds: u64,
    state: &mut ServerState,
    app: &mut ReplayerState,
) {
}

fn advance_by_marker(
    label: String,
    state: &mut ServerState,
    app: &mut ReplayerState,
) {
}

fn scan_all_markers(path: PathBuf) -> Option<HashMap<String, u64>> {
    let in_file = std::fs::File::open(path).ok()?;

    let in_stream = std::io::BufReader::new(in_file);

    let mut bytes = Vec::<u8>::new();

    None
}
