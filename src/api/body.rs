use serde::{Deserialize, Serialize};

use crate::types::message::{ContentBlock, ImageSource, Message, Role};

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
    pub max_tokens_to_sample: u32,
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

impl<S> From<S> for Message
where
    S: Into<String>,
{
    /// Converts a string into a Message with assistant role
    ///
    /// # Arguments
    /// * `str` - The text content for the message
    ///
    /// # Returns
    /// * `Message` - A message with assistant role and text content

    fn from(str: S) -> Self {
        Message::new_blocks(
            Role::Assistant,
            vec![ContentBlock::Text { text: str.into() }],
        )
    }
}
