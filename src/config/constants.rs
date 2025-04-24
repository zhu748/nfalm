use arc_swap::ArcSwap;
use std::sync::LazyLock;

use crate::config::ClewdrConfig;

pub const CONFIG_NAME: &str = "clewdr.toml";
pub const ENDPOINT: &str = "https://claude.ai";
pub static CLEWDR_CONFIG: LazyLock<ArcSwap<ClewdrConfig>> = LazyLock::new(|| {
    let _ = *crate::utils::CLEWDR_DIR;
    let config = ClewdrConfig::new().unwrap_or_default();
    ArcSwap::from_pointee(config)
});

// Default functions
pub const fn default_max_retries() -> usize {
    5
}

pub fn default_ip() -> String {
    "127.0.0.1".to_string()
}

pub fn default_port() -> u16 {
    8484
}

pub const fn default_use_real_roles() -> bool {
    true
}

pub const fn default_padtxt_len() -> usize {
    4000
}

pub const fn default_check_update() -> bool {
    true
}

/// Default cookie value for testing purposes
pub const PLACEHOLDER_COOKIE: &str = "sk-ant-sid01----------------------------SET_YOUR_COOKIE_HERE----------------------------------------AAAAAAAA";
