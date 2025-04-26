use std::{path::PathBuf, sync::LazyLock};

use clap::Parser;
use figlet_rs::FIGfont;

pub mod api;
pub mod bootstrap;
pub mod config;
pub mod error;
pub mod router;
pub mod services;
pub mod state;
pub mod types;
pub mod utils;

pub const IS_DEBUG: bool = cfg!(debug_assertions);
pub static IS_DEV: LazyLock<bool> = LazyLock::new(|| std::env::var("CARGO_MANIFEST_DIR").is_ok());

pub static VERSION_INFO: LazyLock<String> = LazyLock::new(|| {
    format!(
        "v{} by {}\n| profile: {}\n| mode: {}\n| no_fs: {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        if IS_DEBUG { "debug" } else { "release" },
        if *IS_DEV { "dev" } else { "prod" },
        if cfg!(feature = "no_fs") {
            "true"
        } else {
            "false"
        }
    )
});

/// Header for the application
pub static BANNER: LazyLock<String> = LazyLock::new(|| {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("ClewdR");
    let banner = figure.unwrap().to_string();
    format!("{}\n{}", banner, *VERSION_INFO)
});

/// Command line arguments for the application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    /// Force update of the application
    pub update: bool,
    #[arg(short, long)]
    /// load cookie from file
    pub file: Option<PathBuf>,
    /// Alternative config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}
