use std::sync::Arc;

use anyhow::Result;
use clewdr::{self, config::Config, utils::BANNER};

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", *BANNER);
    let config = Config::load()?.trim();
    // TODO: load config from env

    let router = clewdr::api::RouterBuilder::new(config).build();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();

    Ok(())
}
