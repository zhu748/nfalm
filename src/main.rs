use std::fs::OpenOptions;

use clap::Parser;
use clewdr::{
    self, BANNER, config::Config, cookie::CookieManager, error::ClewdrError, state::AppState,
};
use colored::Colorize;
use const_format::formatc;
use tokio::{spawn, sync::mpsc};
use tracing_subscriber::fmt::time::ChronoLocal;

/// Async main function using tokio runtime
#[tokio::main]
async fn main() -> Result<(), ClewdrError> {
    // parse command line arguments
    clewdr::Args::parse();
    // set up logging time format
    let timer = ChronoLocal::new("%H:%M:%S%.3f".to_string());
    // set up logging
    // create log directory if it doesn't exist
    if !std::path::Path::new("log").exists() {
        std::fs::create_dir_all("log")?;
    }
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("log/clewdr.log")?;
    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_writer(std::io::stdout)
        .with_writer(log_file)
        .pretty()
        .init();

    println!("{}", *BANNER);
    // load config from file
    let config = Config::load()?;
    // TODO: load config from env

    // print the title and address
    const TITLE: &str = formatc!(
        "ClewdR v{} by {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );
    println!("{}", TITLE.blue());
    println!("Listening on {}", config.address().green());
    println!("{}", config);

    // initialize the application state
    let (req_tx, req_rx) = mpsc::channel(config.max_connections);
    let (ret_tx, ret_rx) = mpsc::channel(config.max_connections);
    let (submit_tx, submit_rx) = mpsc::channel(config.max_connections);
    let state = AppState::new(config.clone(), req_tx, ret_tx, submit_tx);
    let cm = CookieManager::new(config, req_rx, ret_rx, submit_rx);
    // build axum router
    // create a TCP listener
    let addr = state.config.address().to_string();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let router = clewdr::router::RouterBuilder::new(state).build();
    // serve the application
    spawn(cm.run());
    axum::serve(listener, router).await?;
    Ok(())
}
