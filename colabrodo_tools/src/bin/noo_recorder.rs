//!
//! Writes NOODLES message to disk
//!
//! # Format
//!
//! All in LE.
//!
//! // header
//! 3x u8: 'noo'
//! 1x u8: version (1)
//! 1x u64: unix timestamp
//!
//! // packet
//! 1x u8:  packet type
//! 1x u24: seconds from start
//! 1x u32: size
//! size x u8 : content
//!
//! Type 1:
//! CBOR server message content
//!
//! Type 2:
//! Timestamp w/ utf8 string identifier
//!
//! Type 255:
//! EOF (not required)
//!

use std::{io::Write, path::PathBuf, time::SystemTime};

use clap::Parser;
use colabrodo_client::mapped_client::ciborium::ser;
use colabrodo_common::client_communication::{
    ClientMessageID, IntroductionMessage,
};
use futures_util::{SinkExt, StreamExt};

#[derive(Debug)]
enum Message {
    DropMarker(String),
    WriteCBOR(Vec<u8>),
    Stop,
}

fn message_stamp(m: &Message) -> u8 {
    match m {
        Message::DropMarker(_) => 1,
        Message::WriteCBOR(_) => 2,
        Message::Stop => u8::MAX,
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct Header {
    magic: [u8; 3],
    version: u8,
    timestamp: [u8; 8],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct PacketHeader {
    packet_type: u8,
    time_delta: [u8; 3],
    packet_size: u32,
}

#[derive(Parser, Debug)]
struct CLIArgs {
    /// Server hostname
    #[arg(default_value = "ws://localhost:50000")]
    url: url::Url,

    /// Session recording destination directory
    #[arg(short, long)]
    output_directory: Option<PathBuf>,

    /// Session name
    #[arg(short, long)]
    session_name: Option<String>,

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

    let output_dir = cli_args.output_directory.unwrap_or_else(|| {
        std::env::current_dir().expect("Default directory (the current working directory) is not available")
    });

    log::info!("Output to: {}", output_dir.display());

    let destination_folder = cli_args.session_name.unwrap_or_else(|| {
        format!(
            "{}_{:?}",
            cli_args
                .url
                .host()
                .map(|f| f.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            chrono::offset::Local::now()
        )
    });

    log::info!("Session name: {destination_folder}");

    let data_dir = output_dir.join(destination_folder);

    //std::fs::create_dir_all(&data_dir)
    //    .expect("Unable to create output directory");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime.block_on(cli_main(cli_args.url, data_dir)).unwrap();
}

async fn cli_main(server: url::Url, path: PathBuf) -> anyhow::Result<()> {
    let (message_tx, mut message_rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::spawn(noodles_task(server, message_tx.clone()));

    std::thread::spawn(|| stdin_task(message_tx));

    let mut outfile = Outfile::new(&path);

    while let Some(m) = message_rx.recv().await {
        outfile.write(m)
    }

    Ok(())
}

async fn noodles_task(
    server: url::Url,
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
) -> anyhow::Result<()> {
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(server).await?;

    // send introduction
    {
        let introduction = (
            IntroductionMessage::message_id(),
            IntroductionMessage {
                client_name: "Rusty CLI".to_string(),
            },
        );

        let mut intro_bytes: Vec<u8> = Vec::new();

        ser::into_writer(&introduction, &mut intro_bytes)
            .expect("Unable to serialize introduction message!");

        ws_stream
            .send(tokio_tungstenite::tungstenite::protocol::Message::Binary(
                intro_bytes,
            ))
            .await
            .unwrap();
    }

    let (_write, read) = ws_stream.split();

    read.for_each(|message| async {
        let data = message
            .expect("unable to read message from server")
            .into_data();

        // we can, for now, just assume that this message is valid cbor.

        sender
            .send(Message::WriteCBOR(data))
            .expect("internal error");
    })
    .await;

    Ok(())
}

fn stdin_task(sender: tokio::sync::mpsc::UnboundedSender<Message>) {
    let stdin = std::io::stdin();
    let mut line_buf = String::new();
    while stdin.read_line(&mut line_buf).is_ok() {
        let line = line_buf.trim_end().to_string();
        sender.send(Message::DropMarker(line)).unwrap();
        line_buf.clear();
    }
}

struct Outfile {
    out_stream: std::io::BufWriter<std::fs::File>,
    timestamp: u64,
}

fn trim_u32(value: u32) -> [u8; 3] {
    let bytes = value.to_le_bytes();

    [bytes[1], bytes[2], bytes[3]]
}

fn get_time_delta(base_time: u64) -> [u8; 3] {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before Unix Epoch")
        .as_secs();

    let delta = now - base_time;

    let delta: u32 = delta.try_into().expect("massive delta");

    const INT_24_LIMIT: u32 = 2_u32.pow(24);

    if delta >= INT_24_LIMIT {
        panic!("delta is larger than 194 days. I did not expect this.");
    }

    trim_u32(delta)
}

impl Drop for Outfile {
    fn drop(&mut self) {
        self.write(Message::Stop);
    }
}

impl Outfile {
    fn new(path: &std::path::Path) -> Self {
        let out_file = std::fs::File::create(path)
            .expect("unable to open destination file for writing");

        let out_stream = std::io::BufWriter::new(out_file);

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock before Unix Epoch")
            .as_secs();

        Self {
            out_stream,
            timestamp,
        }
    }

    fn write(&mut self, message: Message) {
        let delta = get_time_delta(self.timestamp);

        let packet_size: usize = match &message {
            Message::DropMarker(x) => x.len(),
            Message::WriteCBOR(x) => x.len(),
            Message::Stop => 0_usize,
        };

        let packet_size: u32 =
            packet_size.try_into().expect("oversized packet");

        let packet = PacketHeader {
            packet_type: message_stamp(&message),
            time_delta: delta,
            packet_size,
        };

        self.write_bytes(bytemuck::bytes_of(&packet));

        match message {
            Message::DropMarker(x) => self.write_bytes(x.as_bytes()),
            Message::WriteCBOR(x) => self.write_bytes(x.as_slice()),
            _ => (),
        };
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.out_stream
            .write_all(bytes)
            .expect("unable to write packet into file");
    }
}
