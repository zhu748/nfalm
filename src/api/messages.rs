use std::sync::LazyLock;

use axum::{
    Extension, Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    api::body::non_stream_message,
    error::ClewdrError,
    middleware::{FormatInfo, UnifiedRequestBody},
    state::ClientState,
    types::message::{ContentBlock, Message, Role},
    utils::print_out_json,
};

/// Exact test message send by SillyTavern
static TEST_MESSAGE_CLAUDE: LazyLock<Message> = LazyLock::new(|| {
    Message::new_blocks(
        Role::User,
        vec![ContentBlock::Text {
            text: "Hi".to_string(),
        }],
    )
});

static TEST_MESSAGE_OAI: LazyLock<Message> = LazyLock::new(|| Message::new_text(Role::User, "Hi"));

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
    if !stream
        && (p.messages == vec![TEST_MESSAGE_CLAUDE.to_owned()]
            || p.messages == vec![TEST_MESSAGE_OAI.to_owned()])
    {
        // respond with a test message
        return (
            Extension(f),
            Ok(Json(non_stream_message(
                "Claude Reverse Proxy is working, please send a real message.".to_string(),
            ))
            .into_response()),
        );
    }
    print_out_json(&p, "client_req.json");
    state.api_format = f.api_format;
    state.stream = stream;
    if let Some(r) = state.try_from_cache(p.to_owned()).await {
        return (Extension(f), Ok(r));
    }
    (Extension(f), state.try_chat(p).await)
}
