use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response, Sse},
};
use axum_auth::AuthBearer;
use eventsource_stream::Eventsource;

use crate::{
    api::{
        ApiFormat,
        body::{ClientRequestBody, Thinking},
        openai::stream::transform,
    },
    config::CLEWDR_CONFIG,
    error::ClewdrError,
    state::ClientState,
    utils::{print_out_text, text::merge_sse},
};

use super::stream::NonStreamEventData;

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
    if p.model.contains("-thinking") {
        p.model = p.model.trim_end_matches("-thinking").to_string();
    }
    p.thinking = Some(Thinking::default());
    state.try_chat(p).await
}

impl ClientState {
    /// Sends a completion request to the Claude API in OpenAI-compatible format
    /// Creates a new conversation, processes the request, and returns formatted response
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - OpenAI-formatted response or error
    pub async fn try_completion(&mut self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        let stream = p.stream;
        let api_res = self.send_chat(p).await?;

        if !stream {
            let stream = api_res.bytes_stream().eventsource();
            let text = merge_sse(stream).await;
            print_out_text(&text, "non_stream.txt");
            return Ok(Json(NonStreamEventData::new(text)).into_response());
        }
        // stream the response
        let input_stream = api_res.bytes_stream().eventsource();
        let output = transform(input_stream);

        Ok(Sse::new(output).into_response())
    }
}
