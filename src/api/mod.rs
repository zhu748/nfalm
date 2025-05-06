/// API module for handling all external HTTP endpoints and request/response transformations
///
/// This module serves as the main entry point for all API requests, providing endpoints
/// for configuration management, message handling, authentication, and OpenAI-compatible
/// interfaces. It also implements response transformation between different API formats.
mod config;
mod claude;
mod misc;

/// Configuration related endpoints for retrieving and updating Clewdr settings
pub use config::{api_get_config, api_post_config};
/// Message handling endpoints for creating and managing chat conversations
pub use claude::api_claude;
/// Miscellaneous endpoints for authentication, cookies, and version information
pub use misc::{api_auth, api_delete_cookie, api_get_cookies, api_post_cookie, api_version};
use strum::Display;

/// Represents the format of the API response
///
/// This enum defines the available API response formats that Clewdr can use
/// when communicating with clients. It supports both Claude's native format
/// and an OpenAI-compatible format for broader compatibility with existing tools.
#[derive(Display, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiFormat {
    /// Claude native format
    Claude,
    /// OpenAI compatible format
    OpenAI,
}
