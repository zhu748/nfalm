/// API module for handling all external HTTP endpoints and request/response transformations
///
/// This module serves as the main entry point for all API requests, providing endpoints
/// for configuration management, message handling, authentication, and OpenAI-compatible
/// interfaces. It also implements response transformation between different API formats.
mod claude;
mod config;
mod gemini;
mod misc;

/// Message handling endpoints for creating and managing chat conversations
pub use claude::api_claude;
/// Configuration related endpoints for retrieving and updating Clewdr settings
pub use config::{api_get_config, api_post_config};
pub use gemini::api_post_gemini;
/// Miscellaneous endpoints for authentication, cookies, and version information
pub use misc::{api_auth, api_delete_cookie, api_get_cookies, api_post_cookie, api_version};
