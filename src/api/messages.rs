use axum::{Extension, extract::State, response::Response};
use colored::Colorize;
use scopeguard::defer;
use tracing::info;

use crate::{
    api::ApiFormat,
    error::ClewdrError,
    middleware::{FormatInfo, UnifiedRequestBody},
    state::ClientState,
    utils::{enabled, print_out_json},
};
/// Axum handler for the API messages
/// Main API endpoint for handling message requests to Claude
/// Processes messages, handles retries, and returns responses in stream or non-stream mode
///
/// # Arguments
/// * `XApiKey(_)` - API key authentication
/// * `state` - Application state containing client information
/// * `p` - Request body containing messages and configuration
///
/// # Returns
/// * `Response` - Stream or JSON response from Claude
pub async fn api_messages(
    State(mut state): State<ClientState>,
    UnifiedRequestBody(p, Extension(f)): UnifiedRequestBody,
) -> (Extension<FormatInfo>, Result<Response, ClewdrError>) {
    // Check if the request is a test message
    let stream = p.stream.unwrap_or_default();
    print_out_json(&p, "client_req.json");
    state.api_format = f.api_format;
    state.stream = stream;
    let format_display = match f.api_format {
        ApiFormat::Claude => f.api_format.to_string().green(),
        ApiFormat::OpenAI => f.api_format.to_string().yellow(),
    };
    info!(
        "[REQ] stream: {}, msgs: {}, model: {}, think: {}, format: {}",
        enabled(stream),
        p.messages.len().to_string().green(),
        p.model.green(),
        enabled(p.thinking.is_some()),
        format_display
    );
    let stopwatch = chrono::Utc::now();
    defer!(
        let elapsed = chrono::Utc::now().signed_duration_since(stopwatch);
        info!(
            "[FIN] elapsed: {}s",
            format!("{}", elapsed.num_milliseconds() as f64 / 1000.0).green()
        );
    );
    if let Some(r) = state.try_from_cache(p.to_owned()).await {
        return (Extension(f), Ok(r));
    }
    (Extension(f), state.try_chat(p).await)
}
