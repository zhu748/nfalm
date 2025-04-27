use clap::Parser;
use std::{fs, path::PathBuf, str::FromStr, sync::LazyLock};
use tracing::error;

use crate::{IS_DEV, config::CONFIG_NAME, error::ClewdrError};

pub mod text;

pub static ARG_COOKIE_FILE: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let args = crate::Args::parse();
    if let Some(cookie_file) = args.file {
        // canonicalize the path
        if !cookie_file.exists() {
            error!("No cookie file found at: {}", cookie_file.display());
            return None;
        }
        cookie_file.canonicalize().ok()
    } else {
        None
    }
});

pub static ARG_CONFIG_FILE: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let args = crate::Args::parse();
    if let Some(config_file) = args.config {
        // canonicalize the path
        config_file.canonicalize().ok()
    } else {
        None
    }
});

pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    if let Some(path) = ARG_CONFIG_FILE.as_ref() {
        path.to_owned()
    } else {
        CLEWDR_DIR.join(CONFIG_NAME)
    }
});

pub static CLEWDR_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| set_clewdr_dir().expect("Failed to get dir"));
pub const LOG_DIR: &str = "log";

/// Gets and sets up the configuration directory for the application
///
/// In dev, uses the current working directory
/// In production, uses the directory of the executable
/// Also creates the log directory if it doesn't exist
///
/// # Returns
/// * `Result<PathBuf, ClewdrError>` - The path to the configuration directory on success, or an error
fn set_clewdr_dir() -> Result<PathBuf, ClewdrError> {
    let dir = if *IS_DEV {
        // In development use cargo dir
        let cargo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        cargo_dir.canonicalize()?
    } else {
        // In production use the directory of the executable
        std::env::current_exe()?
            .parent()
            .ok_or_else(|| ClewdrError::PathNotFound("exec dir".to_string()))?
            .canonicalize()?
            .to_path_buf()
    };
    std::env::set_current_dir(&dir)?;
    // create log dir
    #[cfg(feature = "no_fs")]
    {
        return Ok(dir);
    }

    let log_dir = dir.join(LOG_DIR);
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
    }
    Ok(dir)
}

/// Helper function to print out JSON to a file in the log directory
///
/// # Arguments
/// * `json` - The JSON object to serialize and output
/// * `file_name` - The name of the file to write in the log directory
pub fn print_out_json(json: &impl serde::ser::Serialize, file_name: &str) {
    #[cfg(feature = "no_fs")]
    {
        return;
    }
    let text = serde_json::to_string_pretty(json).unwrap_or_default();
    print_out_text(&text, file_name);
}

/// Helper function to print out text to a file in the log directory
///
/// # Arguments
/// * `text` - The text content to write
/// * `file_name` - The name of the file to write in the log directory
pub fn print_out_text(text: &str, file_name: &str) {
    #[cfg(feature = "no_fs")]
    {
        return;
    }
    let Ok(log_dir) = PathBuf::from_str(LOG_DIR);
    let file_name = log_dir.join(file_name);
    let Ok(mut file) = std::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_name)
    else {
        error!("Failed to open file: {}", file_name.display());
        return;
    };
    if let Err(e) = std::io::Write::write_all(&mut file, text.as_bytes()) {
        error!("Failed to write to file: {}\n", e);
    }
}

/// Timezone for the API
pub const TIME_ZONE: &str = "America/New_York";
