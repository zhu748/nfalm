use axum::extract::FromRequestParts;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use crate::{
    config::CLEWDR_CONFIG,
    error::ClewdrError,
    types::message::{ContentBlock, ImageSource, Message, Role},
};

/// Claude.ai attachment
#[derive(Deserialize, Serialize, Debug)]
pub struct Attachment {
    extracted_content: String,
    file_name: String,
    file_type: String,
    file_size: u64,
}

impl Attachment {
    /// Creates a new Attachment with the given content
    ///
    /// # Arguments
    /// * `content` - The text content for the attachment
    ///
    /// # Returns
    /// A new Attachment instance configured as a text file
    pub fn new(content: String) -> Self {
        Attachment {
            file_size: content.len() as u64,
            extracted_content: content,
            file_name: "paste.txt".to_string(),
            file_type: "txt".to_string(),
        }
    }
}

/// Request body to be sent to the Claude.ai
#[derive(Deserialize, Serialize, Debug)]
pub struct RequestBody {
    pub max_tokens_to_sample: u64,
    pub attachments: Vec<Attachment>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub rendering_mode: String,
    pub prompt: String,
    pub timezone: String,
    #[serde(skip)]
    pub images: Vec<ImageSource>,
}

/// Default function for maximum tokens
///
/// # Returns
/// * `u64` - The default value of 4096 tokens
fn max_tokens() -> u64 {
    4096
}

/// Request body sent from the client
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientRequestBody {
    #[serde(default = "max_tokens")]
    pub max_tokens: u64,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    pub model: String,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub thinking: Option<Thinking>,
    #[serde(default)]
    pub system: Value,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default)]
    pub top_p: f32,
    #[serde(default)]
    pub top_k: u64,
}

/// Thinking mode in Claude API Request
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Thinking {
    budget_tokens: u64,
    r#type: String,
}

impl Default for Thinking {
    fn default() -> Self {
        Thinking {
            budget_tokens: 0,
            r#type: "none".to_string(),
        }
    }
}

pub struct XApiKey(pub String);

impl<S> FromRequestParts<S> for XApiKey
where
    S: Sync,
{
    type Rejection = ClewdrError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !CLEWDR_CONFIG.load().v1_auth(key) {
            warn!("Invalid password: {}", key);
            return Err(ClewdrError::IncorrectKey);
        }
        Ok(XApiKey(key.to_string()))
    }
}

/// Transforms a string to a message with assistant role
/// Used to create response messages for non-streaming API calls
///
/// # Arguments
/// * `str` - The text content for the message
///
/// # Returns
/// * `Message` - A message with assistant role and text content
pub fn non_stream_message(str: String) -> Message {
    Message::new_blocks(Role::Assistant, vec![ContentBlock::Text { text: str }])
}
