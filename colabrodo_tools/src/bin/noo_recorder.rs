//!
//! Writes NOODLES message to disk
//!
//! # Format
//!
//! // header
//! 1x u8: version (1)
//!
//!
//!
//! 1x u8: packet type
//!
//!

use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use clap::Parser;

#[derive(Debug, Default)]
struct CLIState {}

#[derive(Debug)]
enum Message {
    DropMarker(u64, String),
    WriteMessage(u64, Vec<u8>),
    Stop,
}

fn message_stamp(m: &Message) -> u8 {
    match m {
        Message::DropMarker(_, _) => 1,
        Message::WriteMessage(_, _) => 2,
        Message::Stop => u8::MAX,
    }
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
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel();

    let conn_result = tokio_tungstenite::connect_async(server).await?;

    let state = Arc::new(Mutex::new(CLIState::default()));

    std::thread::spawn(|| stdin_task(stdin_tx));

    let out_file = std::fs::File::create(path)
        .expect("unable to open destination file for writing");

    let mut out_buff = std::io::BufWriter::new(out_file);

    while let Some(m) = stdin_rx.recv().await {
        let message_id = [message_stamp(&m)];
        out_buff.write(&message_id);

        match m {
            Message::DropMarker(time, x) => {
                out_buff
                    .write(x.as_slice())
                    .expect("unable to write data to file");
            }
            Message::WriteMessage(time, x) => {
                out_buff
                    .write(x.as_slice())
                    .expect("unable to write data to file");
            }
            Message::Stop => {
                break;
            }
        }
    }

    Ok(())
}

fn stdin_task(sender: tokio::sync::mpsc::UnboundedSender<Message>) {
    let stdin = std::io::stdin();
    let mut line_buf = String::new();
    while let Ok(_) = stdin.read_line(&mut line_buf) {
        let line = line_buf.trim_end().to_string();
        sender.send(Message::DropMarker(line)).unwrap();
        line_buf.clear();
    }
}
