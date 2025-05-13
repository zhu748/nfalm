use std::str::FromStr;

use clewdr::{
    self, BANNER, IS_DEV,
    config::{ARG_CONFIG_FILE, ARG_COOKIE_FILE, CLEWDR_CONFIG, CLEWDR_DIR, CONFIG_PATH, LOG_DIR},
    error::ClewdrError,
};
use colored::Colorize;
use tracing::warn;
use tracing_subscriber::{
    Layer, Registry,
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
    let _ = *ARG_CONFIG_FILE;
    let _ = *CLEWDR_DIR;
    // set up logging time format
    let timer = ChronoLocal::new("%H:%M:%S%.3f".to_string());
    // set up logging
    let console_layer = console_subscriber::spawn();
    let filter = if *IS_DEV {
        tracing_subscriber::filter::LevelFilter::DEBUG
    } else {
        tracing_subscriber::filter::LevelFilter::INFO
    };
    let tokio_console_filter =
        tracing_subscriber::filter::Targets::from_str("tokio=trace,runtime=trace")
            .expect("Failed to parse filter");
    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .with_writer(std::io::stdout)
                .with_timer(timer.to_owned())
                .with_filter(filter),
        )
        .with(console_layer.with_filter(tokio_console_filter));
    #[cfg(not(feature = "no_fs"))]
    let (subscriber, _guard) = {
        let file_appender = tracing_appender::rolling::daily(LOG_DIR, "clewdr.log");
        let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

        (
            subscriber.with(
                fmt::Layer::default()
                    .with_writer(file_writer)
                    .with_timer(timer)
                    .with_filter(filter),
            ),
            _guard,
        )
    };
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    println!("{}", *BANNER);

    let updater = clewdr::services::update::ClewdrUpdater::new()?;
    if let Err(e) = updater.check_for_updates().await {
        warn!("Update check failed: {}", e);
    }

    // print info
    println!("Config dir: {}", CONFIG_PATH.display().to_string().blue());
    println!("{}", *CLEWDR_CONFIG);

    // build axum router
    // create a TCP listener
    let addr = CLEWDR_CONFIG.load().address();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let router = clewdr::router::RouterBuilder::new()
        .with_default_setup()
        .build();
    // serve the application
    Ok(axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl-C handler");
        })
        .await?)
}
