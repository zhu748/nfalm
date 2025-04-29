use std::sync::LazyLock;

use axum::{
    Json,
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use eventsource_stream::Eventsource;

use crate::{
    api::body::non_stream_message,
    error::ClewdrError,
    state::ClientState,
    types::message::{ContentBlock, Message, Role},
    utils::print_out_text,
    utils::text::merge_sse,
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
    state.try_chat(p).await
}

impl ClientState {
    /// Tries to send a message to the Claude API
    /// Creates a new conversation, processes the request, and returns the response
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - Response from Claude or error
    pub async fn try_message(&mut self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        let stream = p.stream;
        let api_res = self.send_chat(p).await?;

        // if not streaming, return the response
        if !stream {
            let stream = api_res.bytes_stream().eventsource();
            let text = merge_sse(stream).await;
            print_out_text(&text, "non_stream.txt");
            return Ok(Json(non_stream_message(text)).into_response());
        }

        // stream the response
        let input_stream = api_res.bytes_stream();
        Ok(Body::from_stream(input_stream).into_response())
    }
}
