use clewdr::{
    self, BANNER,
    config::{CLEWDR_CONFIG, CONFIG_NAME},
    error::ClewdrError,
    services::cookie_manager::CookieManager,
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

/// Application entry point
/// Sets up logging, checks for updates, initializes the application state,
/// creates the router, and starts the server
///
/// # Returns
/// Result indicating success or failure of the application execution
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

    let updater = clewdr::services::update::ClewdrUpdater::new()?;
    if let Err(e) = updater.check_for_updates().await {
        warn!("Update check failed: {}", e);
    }

    // print info
    println!(
        "Config dir: {}",
        CLEWDR_DIR.join(CONFIG_NAME).display().to_string().blue()
    );
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
