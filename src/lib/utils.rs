use serde_json::Value;
use std::path::PathBuf;
use tracing::error;

use crate::{config::CONFIG_NAME, error::ClewdrError};

/// Get directory of the config file
pub fn config_dir() -> Result<PathBuf, ClewdrError> {
    let cwd = std::env::current_dir().map_err(|_| ClewdrError::PathNotFound("cwd".to_string()))?;
    let cwd_config = cwd.join(CONFIG_NAME);
    if cwd_config.exists() {
        return Ok(cwd);
    }
    let exec_path =
        std::env::current_exe().map_err(|_| ClewdrError::PathNotFound("exec".to_string()))?;
    let exec_dir = exec_path
        .parent()
        .ok_or_else(|| ClewdrError::PathNotFound("exec dir".to_string()))?
        .to_path_buf();
    let exec_config = exec_dir.join(CONFIG_NAME);
    if exec_config.exists() {
        return Ok(exec_dir);
    }
    Err(ClewdrError::PathNotFound(
        "No config found in cwd or exec dir".to_string(),
    ))
}

/// Helper function to print out json
pub fn print_out_json(json: &impl serde::ser::Serialize, file_name: &str) {
    let text = serde_json::to_string_pretty(json).unwrap_or_default();
    print_out_text(&text, file_name);
}

/// Helper function to print out text
pub fn print_out_text(text: &str, file_name: &str) {
    let Ok(dir) = config_dir() else {
        error!("No config found in cwd or exec dir");
        return;
    };
    let log_dir = dir.join("log");
    if !log_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            error!("Failed to create log dir: {}\n", e);
            return;
        }
    }
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

/// Mimic the behavior of JavaScript's truthiness
pub trait JsBool {
    fn js_bool(&self) -> bool;
}

impl JsBool for Option<&Value> {
    fn js_bool(&self) -> bool {
        match self {
            Some(v) => v.js_bool(),
            None => false,
        }
    }
}

impl JsBool for Value {
    fn js_bool(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Number(n) => {
                // '-0'/'0'/NaN => false
                // other numbers => true
                if let Some(num) = n.as_f64() {
                    if num == 0.0 || num.is_nan() {
                        return false;
                    }
                }
                true
            }
            Value::Bool(b) => *b,
            Value::String(s) => {
                // empty string => false
                // other strings => true
                if s.is_empty() {
                    return false;
                }
                true
            }
            _ => true,
        }
    }
}

/// Models for the API
pub const MODELS: [&str; 10] = [
    "claude-3-7-sonnet-20250219",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    "claude-2.1",
    "claude-2.0",
    "claude-1.3",
    "claude-instant-1.2",
    "claude-instant-1.1",
];

/// Endpoint for the API
pub const ENDPOINT: &str = "https://api.claude.ai";

/// Timezone for the API
pub const TIME_ZONE: &str = "America/New_York";
