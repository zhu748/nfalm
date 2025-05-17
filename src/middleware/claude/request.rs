use std::sync::LazyLock;

use axum::{
    Json,
    extract::{FromRequest, Request},
    response::IntoResponse,
};

use crate::{
    claude_state::{ClaudeApiFormat, ClaudeState},
    error::ClewdrError,
    types::claude_message::{ContentBlock, CreateMessageParams, Message, Role},
};

use super::to_oai;

/// A custom extractor that unifies different API formats
///
/// This extractor processes incoming requests, handling differences between
/// Claude and OpenAI API formats, and applies preprocessing to ensure consistent
/// handling throughout the application. It also detects and handles test messages
/// from client applications.
///
/// # Functionality
///
/// - Extracts and normalizes message parameters from different API formats
/// - Detects and processes "thinking mode" requests by modifying model names
/// - Identifies test messages and handles them appropriately
/// - Attempts to retrieve responses from cache before processing requests
/// - Provides format information via the FormatInfo extension
pub struct ClaudePreprocess(pub CreateMessageParams, pub ClaudeContext);

/// Contains information about the API format and streaming status
///
/// This structure is passed through the request pipeline to inform
/// handlers and response processors about the API format being used
/// and whether the response should be streamed.
#[derive(Debug, Clone)]
pub struct ClaudeContext {
    /// Whether the response should be streamed
    pub stream: bool,
    /// The API format being used (Claude or OpenAI)
    pub api_format: ClaudeApiFormat,
    /// The stop sequence used for the request
    pub stop_sequences: Vec<String>,
}

/// Predefined test message in Claude format for connection testing
///
/// This is a standard test message sent by clients like SillyTavern
/// to verify connectivity. The system detects these messages and
/// responds with a predefined test response to confirm service availability.
static TEST_MESSAGE_CLAUDE: LazyLock<Message> = LazyLock::new(|| {
    Message::new_blocks(
        Role::User,
        vec![ContentBlock::Text {
            text: "Hi".to_string(),
        }],
    )
});

/// Predefined test message in OpenAI format for connection testing
static TEST_MESSAGE_OAI: LazyLock<Message> = LazyLock::new(|| Message::new_text(Role::User, "Hi"));

impl FromRequest<ClaudeState> for ClaudePreprocess {
    type Rejection = ClewdrError;

    async fn from_request(req: Request, state: &ClaudeState) -> Result<Self, Self::Rejection> {
        let uri = req.uri().to_string();
        let Json(mut body) = Json::<CreateMessageParams>::from_request(req, &()).await?;

        // Handle thinking mode by modifying the model name
        if body.model.ends_with("-thinking") {
            body.model = body.model.trim_end_matches("-thinking").to_string();
            body.thinking = Some(Default::default());
        }

        // Check for test messages and respond appropriately
        if !body.stream.unwrap_or_default()
            && (body.messages == vec![TEST_MESSAGE_CLAUDE.to_owned()]
                || body.messages == vec![TEST_MESSAGE_OAI.to_owned()])
        {
            // Respond with a test message
            return Err(ClewdrError::TestMessage);
        }

        // Determine streaming status and API format
        let stream = body.stream.unwrap_or_default();
        let format = if uri.contains("chat/completions") {
            ClaudeApiFormat::OpenAI
        } else {
            ClaudeApiFormat::Claude
        };

        // Update state with format information
        let mut state = state.to_owned();
        state.api_format = format;
        state.stream = stream;
        let mut stop = body.stop_sequences.to_owned().unwrap_or_default();
        stop.extend_from_slice(body.stop.to_owned().unwrap_or_default().as_slice());
        stop.sort();
        stop.dedup();
        let info = ClaudeContext {
            stream,
            api_format: format,
            stop_sequences: stop,
        };

        // Try to retrieve from cache before processing
        if let Some(mut r) = state.try_from_cache(&body).await {
            r.extensions_mut().insert(info.to_owned());
            let r = to_oai(r).await.into_response();
            return Err(ClewdrError::CacheFound { res: Box::new(r) });
        }

        Ok(Self(body, info))
    }
}
