use std::sync::LazyLock;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    api::body::non_stream_message,
    error::ClewdrError,
    state::ClientState,
    types::message::{ContentBlock, Message, Role},
};

use super::{
    ApiFormat,
    body::{ClientRequestBody, XApiKey},
};

/// Exact test message send by SillyTavern
pub static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| {
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
    XApiKey(_): XApiKey,
    State(mut state): State<ClientState>,
    Json(p): Json<ClientRequestBody>,
) -> Result<Response, ClewdrError> {
    // Check if the request is a test message
    if !p.stream && p.messages == vec![TEST_MESSAGE.clone()] {
        // respond with a test message
        return Ok(Json(non_stream_message(
            "Claude Reverse Proxy is working, please send a real message.".to_string(),
        ))
        .into_response());
    }
    state.api_format = ApiFormat::Claude;
    state.stream = p.stream;
    state.try_chat(p).await
}
