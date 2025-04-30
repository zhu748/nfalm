use std::sync::LazyLock;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    api::body::non_stream_message,
    error::ClewdrError,
    middleware::UnifiedRequestBody,
    state::ClientState,
    types::message::{ContentBlock, Message, Role},
};

use super::ApiFormat;

/// Exact test message send by SillyTavern
static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| {
    Message::new_blocks(
        Role::User,
        vec![ContentBlock::Text {
            text: "Hi".to_string(),
        }],
    )
});

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
    UnifiedRequestBody(p): UnifiedRequestBody,
) -> Result<Response, ClewdrError> {
    // Check if the request is a test message
    let stream = p.stream.unwrap_or_default();
    if !stream && p.messages == vec![TEST_MESSAGE.to_owned()] {
        // respond with a test message
        return Ok(Json(non_stream_message(
            "Claude Reverse Proxy is working, please send a real message.".to_string(),
        ))
        .into_response());
    }
    let key = p.get_hash();
    if let Some(r) = state.try_from_cache(p.to_owned(), key).await {
        return Ok(r);
    }
    state.api_format = ApiFormat::Claude;
    state.stream = stream;
    state.try_chat(p).await
}
