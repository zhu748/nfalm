use arc_swap::ArcSwap;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::LazyLock,
};

use crate::config::ClewdrConfig;

pub const CONFIG_NAME: &str = "clewdr.toml";
pub const ENDPOINT: &str = "https://claude.ai";
pub static CLEWDR_CONFIG: LazyLock<ArcSwap<ClewdrConfig>> = LazyLock::new(|| {
    let _ = *crate::utils::CLEWDR_DIR;
    let config = ClewdrConfig::new().unwrap_or_default();
    ArcSwap::from_pointee(config)
});

// Default functions
/// Default number of maximum retries for API requests
///
/// # Returns
/// * `usize` - The default value of 5
pub const fn default_max_retries() -> usize {
    5
}

/// Default IP address for the server to bind to
///
/// # Returns
/// * `String` - The default localhost IP "127.0.0.1"
pub fn default_ip() -> IpAddr {
    Ipv4Addr::new(127, 0, 0, 1).into()
}

/// Default port for the server to listen on
///
/// # Returns
/// * `u16` - The default port number 8484
pub fn default_port() -> u16 {
    8484
}

/// Default setting for using real roles in conversations
///
/// # Returns
/// * `bool` - The default value of true
pub const fn default_use_real_roles() -> bool {
    true
}

/// Default length of padding text
///
/// # Returns
/// * `usize` - The default value of 4000 tokens
pub const fn default_padtxt_len() -> usize {
    4000
}

/// Default setting for checking updates on startup
///
/// # Returns
/// * `bool` - The default value of true
pub const fn default_check_update() -> bool {
    true
}

/// Default cookie value for testing purposes
pub const PLACEHOLDER_COOKIE: &str = "sk-ant-sid01----------------------------SET_YOUR_COOKIE_HERE----------------------------------------AAAAAAAA";
