use axum::{Json, extract::State, response::Response};
use axum_auth::AuthBearer;

use crate::{
    api::{
        ApiFormat,
        body::{ClientRequestBody, Thinking},
    },
    config::CLEWDR_CONFIG,
    error::ClewdrError,
    state::ClientState,
};

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
    Json(mut p): Json<ClientRequestBody>,
) -> Result<Response, ClewdrError> {
    if !CLEWDR_CONFIG.load().v1_auth(&token) {
        return Err(ClewdrError::IncorrectKey);
    }
    // TODO: Check if the request is a test message
    state.api_format = ApiFormat::OpenAI;
    state.stream = p.stream;
    if p.model.contains("-thinking") {
        p.model = p.model.trim_end_matches("-thinking").to_string();
    }
    p.thinking = Some(Thinking::default());
    state.try_chat(p).await
}
