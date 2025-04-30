use std::sync::LazyLock;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};
use axum_auth::AuthBearer;

use crate::{
    api::{ApiFormat, openai::stream::NonStreamEventData},
    config::CLEWDR_CONFIG,
    error::ClewdrError,
    state::ClientState,
    types::message::{CreateMessageParams, Message, Role, Thinking},
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
    AuthBearer(token): AuthBearer,
    State(mut state): State<ClientState>,
    Json(mut p): Json<CreateMessageParams>,
) -> Result<Response, ClewdrError> {
    if !CLEWDR_CONFIG.load().v1_auth(&token) {
        return Err(ClewdrError::IncorrectKey);
    }
    let stream = p.stream.unwrap_or_default();

    // Check if the request is a test message
    if !stream && p.messages == vec![TEST_MESSAGE.to_owned()] {
        // respond with a test message
        return Ok(Json(NonStreamEventData::new(
            "Claude Reverse Proxy is working, please send a real message.".to_string(),
        ))
        .into_response());
    }
    state.api_format = ApiFormat::OpenAI;
    state.stream = stream;
    if p.model.contains("-thinking") {
        p.model = p.model.trim_end_matches("-thinking").to_string();
    }
    p.thinking = Some(Thinking::default());
    state.try_chat(p).await
}
