use std::path::{Path, PathBuf};
use tracing::error;
use std::fs;
use walkdir::WalkDir;

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
    // cd to the exec dir
    std::env::set_current_dir(&exec_dir)
        .map_err(|_| ClewdrError::PathNotFound("exec dir".to_string()))?;
    Ok(exec_dir)
}

/// Recursively copies all files and subdirectories from `src` to `dst`
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), ClewdrError> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    
    if !src.exists() {
        return Err(ClewdrError::PathNotFound(format!(
            "Source directory not found: {}", 
            src.display()
        )));
    }
    
    fs::create_dir_all(dst)?;
    
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry?;
        
        let path = entry.path();
        let relative_path = path.strip_prefix(src)
            .map_err(|_| ClewdrError::PathNotFound(format!("Failed to strip prefix from path: {}", path.display())))?;
        let target_path = dst.join(relative_path);
        
        if path.is_dir() {
            fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::copy(path, &target_path)?;
        }
    }
    
    Ok(())
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

/// Timezone for the API
pub const TIME_ZONE: &str = "America/New_York";
