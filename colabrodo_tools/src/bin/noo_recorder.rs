use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use clap::Parser;

#[derive(Debug, Default)]
struct CLIState {}

enum Message {}

#[derive(Parser, Debug)]
struct CLIArgs {
    /// Server hostname
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

    std::fs::create_dir_all(&data_dir)
        .expect("Unable to create output directory");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_io()
        .build()
        .unwrap();

    runtime.block_on(cli_main(cli_args.url, data_dir));
}

async fn cli_main(server: url::Url, path: PathBuf) -> anyhow::Result<()> {
    //let (stdin_tx, stdin_rx) = tokio::sync::mpsc::unbounded_channel();

    let conn_result = tokio_tungstenite::connect_async(server).await?;

    let state = Arc::new(Mutex::new(CLIState::default()));

    //std::thread::spawn(read_stdin(stdin_tx));

    tokio::select! {
        ask_stop = tokio::signal::ctrl_c() => {
            println!("{:?}", ask_stop)
        }
    }

    Ok(())
}

// // docs recommend to use a separate thread with blocking IO on stdin.
// // So lets do that here.

// async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
//     let mut stdin = tokio::io::stdin();

//     let mut

//     //tx.unbounded_send(Message::binary(buf)).unwrap();
// }
