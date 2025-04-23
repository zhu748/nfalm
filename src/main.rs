use clewdr::{
    self, BANNER,
    config::{CLEWDR_CONFIG, CONFIG_NAME},
    cookie_manager::CookieManager,
    error::ClewdrError,
    state::ClientState,
    utils::{ARG_COOKIE_FILE, CLEWDR_DIR, LOG_DIR},
};
use colored::Colorize;
use tracing::warn;
use tracing_subscriber::{
    Registry,
    fmt::{self, time::ChronoLocal},
    layer::SubscriberExt,
};

/// Async main function using tokio runtime
#[tokio::main]
async fn main() -> Result<(), ClewdrError> {
    let _ = enable_ansi_support::enable_ansi_support();
    // setup dir
    let _ = *ARG_COOKIE_FILE;
    let _ = *CLEWDR_DIR;
    // set up logging time format
    let timer = ChronoLocal::new("%H:%M:%S%.3f".to_string());
    // set up logging
    let file_appender = tracing_appender::rolling::daily(LOG_DIR, "clewdr.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .with_writer(file_writer)
                .with_timer(timer.clone()),
        )
        .with(
            fmt::Layer::default()
                .with_writer(std::io::stdout)
                .with_timer(timer),
        );

    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    println!("{}", *BANNER);

    let updater = clewdr::update::ClewdrUpdater::new()?;
    if let Err(e) = updater.check_for_updates().await {
        warn!("Update check failed: {}", e);
    }

    // print the address
    let addr = format!("http://{}", CLEWDR_CONFIG.load().address());
    let api_addr = format!("{}/v1", addr);
    println!(
        "Config dir: {}",
        CLEWDR_DIR.join(CONFIG_NAME).display().to_string().blue()
    );
    println!("API address: {}", api_addr.green());
    println!("Web address: {}", addr.green());
    println!("{}", *CLEWDR_CONFIG);

    // initialize the application state
    let tx = CookieManager::start();
    let state = ClientState::new(tx);
    // build axum router
    // create a TCP listener
    let addr = CLEWDR_CONFIG.load().address().to_string();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let router = clewdr::router::RouterBuilder::new(state).build();
    // serve the application
    axum::serve(listener, router).await?;
    Ok(())
}
