use std::sync::LazyLock;

use crate::{
    api::{ApiFormat, openai::stream::NonStreamEventData},
    error::ClewdrError,
    middleware::UnifiedRequestBody,
    state::ClientState,
    types::message::{Message, Role},
};
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| Message::new_text(Role::User, "Hi"));

/// OpenAI-compatible API endpoint for chat completions
/// Handles authentication, processes messages, and supports both streaming and non-streaming responses
///
/// # Arguments
/// * `token` - Bearer token for API authentication
/// * `state` - Application state containing client information
/// * `p` - Request body containing messages and configuration
///
/// # Returns
/// * `Response` - JSON or stream response in OpenAI format
pub async fn api_completion(
    State(mut state): State<ClientState>,
    UnifiedRequestBody(p): UnifiedRequestBody,
) -> Result<Response, ClewdrError> {
    let stream = p.stream.unwrap_or_default();
    // Check if the request is a test message
    if !stream && p.messages == vec![TEST_MESSAGE.to_owned()] {
        // respond with a test message
        return Ok(Json(NonStreamEventData::new(
            "Claude Reverse Proxy is working, please send a real message.".to_string(),
        ))
        .into_response());
    }
    let key = p.get_hash();
    if let Some(r) = state.try_from_cache(p.to_owned(), key).await {
        return Ok(r);
    }
    state.api_format = ApiFormat::OpenAI;
    state.stream = stream;
    state.try_chat(p).await
}
