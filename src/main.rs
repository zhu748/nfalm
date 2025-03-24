use std::sync::Arc;

use anyhow::Result;
use clewdr::{self, config::Config, utils::BANNER};

#[tokio::main]
async fn main() -> Result<()> {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)?;
    println!("{}", *BANNER);
    let config = Config::load()?.validate();
    // TODO: load config from env

    let router = clewdr::api::RouterBuilder::new(config).build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();

    Ok(())
}
