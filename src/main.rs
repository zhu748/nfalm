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
    tracing_subscriber::fmt().with_timer(timer).pretty().init();

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
    let (req_tx, req_rx) = mpsc::channel(100);
    let (ret_tx, ret_rx) = mpsc::channel(100);
    let state = AppState::new(config, req_tx, ret_tx);
    let cm = CookieManager::new(state.config.clone(), req_rx, ret_rx);
    // build axum router
    let router = clewdr::router::RouterBuilder::new(state.clone()).build();
    // create a TCP listener
    let addr = state.config.address().to_string();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // serve the application
    spawn(cm.run());
    axum::serve(listener, router).await?;
    Ok(())
}
