use axum::body::Bytes;
use eventsource_stream::EventStream;
use futures::{Stream, StreamExt, pin_mut};
use itertools::Itertools;
use rand::{Rng, rng};
use serde::Deserialize;
use serde_json::Value;
use std::{fmt::Write, mem};
use tracing::warn;

use crate::{
    api::{
        ApiFormat,
        body::{Attachment, RequestBody},
    },
    config::CLEWDR_CONFIG,
    state::ClientState,
    types::message::{
        ContentBlock, CreateMessageParams, ImageSource, Message, MessageContent, Role,
    },
    utils::{TIME_ZONE, print_out_text},
};

/// Merged messages and images
#[derive(Default, Debug)]
struct Merged {
    pub paste: String,
    pub prompt: String,
    pub images: Vec<ImageSource>,
}

impl ClientState {
    pub fn transform_request(&self, mut value: CreateMessageParams) -> Option<RequestBody> {
        let (value, merged) = match self.api_format {
            ApiFormat::Claude => {
                let system = value.system.take();
                let msgs = mem::take(&mut value.messages);
                let system = merge_system(system.unwrap_or_default());
                let merged = self.merge_messages(msgs, system)?;
                (value, merged)
            }
            ApiFormat::OpenAI => {
                let mut msgs = mem::take(&mut value.messages);
                let mut role = msgs.first().map(|m| m.role)?;
                for msg in msgs.iter_mut() {
                    if msg.role != Role::System {
                        role = msg.role;
                    } else {
                        msg.role = role;
                    }
                }
                let merged = self.merge_messages(msgs, String::new())?;
                (value, merged)
            }
        };
        Some(RequestBody {
            max_tokens_to_sample: value.max_tokens,
            attachments: vec![Attachment::new(merged.paste)],
            files: vec![],
            model: if self.is_pro() {
                Some(value.model)
            } else {
                None
            },
            rendering_mode: if value.stream.unwrap_or_default() {
                "messages".to_string()
            } else {
                "raw".to_string()
            },
            prompt: merged.prompt,
            timezone: TIME_ZONE.to_string(),
            images: merged.images,
        })
    }

    /// Merges multiple messages into a single text prompt, handling system instructions
    /// and extracting any images from the messages
    ///
    /// # Arguments
    /// * `msgs` - Vector of messages to merge
    /// * `system` - System instructions to prepend
    ///
    /// # Returns
    /// * `Option<Merged>` - Merged prompt text, images, and additional metadata, or None if merging fails
    fn merge_messages(&self, msgs: Vec<Message>, system: String) -> Option<Merged> {
        if msgs.is_empty() {
            return None;
        }
        let h = CLEWDR_CONFIG
            .load()
            .custom_h
            .to_owned()
            .unwrap_or("Human".to_string());
        let a = CLEWDR_CONFIG
            .load()
            .custom_a
            .to_owned()
            .unwrap_or("Assistant".to_string());

        let user_real_roles = CLEWDR_CONFIG.load().use_real_roles;
        let line_breaks = if user_real_roles { "\n\n\x08" } else { "\n\n" };
        let system = system.trim().to_string();
        let size = size_of_val(&msgs);
        // preallocate string to avoid reallocations
        let mut w = String::with_capacity(size);
        // generate padding text
        if !CLEWDR_CONFIG.load().pad_tokens.is_empty() {
            let len = CLEWDR_CONFIG.load().padtxt_len;
            let padding = self.generate_padding(len);
            w.push_str(padding.as_str());
        }

        let mut imgs: Vec<ImageSource> = vec![];

        let chunks = msgs
            .into_iter()
            .map_while(|m| match m.content {
                MessageContent::Blocks { content } => {
                    // collect all text blocks, join them with new line
                    let blocks = content
                        .into_iter()
                        .map_while(|b| match b {
                            ContentBlock::Text { text } => Some(text.trim().to_string()),
                            ContentBlock::Image { source } => {
                                // push image to the list
                                imgs.push(source);
                                None
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if blocks.is_empty() {
                        None
                    } else {
                        Some((m.role, blocks))
                    }
                }
                MessageContent::Text { content } => {
                    // plain text
                    let content = content.trim().to_string();
                    if content.is_empty() {
                        None
                    } else {
                        Some((m.role, content))
                    }
                }
            })
            // chunk by role
            .chunk_by(|m| m.0);
        // join same role with new line
        let mut msgs = chunks.into_iter().map(|(role, grp)| {
            let txt = grp.into_iter().map(|m| m.1).collect::<Vec<_>>().join("\n");
            (role, txt)
        });
        // first message does not need prefix
        if !system.is_empty() {
            w += system.as_str();
        } else {
            let first = msgs.next()?;
            w += first.1.as_str();
        }
        for (role, text) in msgs {
            let prefix = match role {
                Role::System => {
                    warn!("System message should be merged into the first message");
                    continue;
                }
                Role::User => format!("{}: ", h),
                Role::Assistant => format!("{}: ", a),
            };
            write!(w, "{}{}{}", line_breaks, prefix, text).unwrap();
        }
        print_out_text(w.as_str(), "paste.txt");

        // prompt polyfill
        let p = CLEWDR_CONFIG.load().custom_prompt.to_owned();

        Some(Merged {
            paste: w,
            prompt: p,
            images: imgs,
        })
    }

    /// Generates random padding text of specified length
    /// Used to pad prompts with tokens to meet minimum length requirements
    ///
    /// # Arguments
    /// * `length` - The target length of padding in tokens
    ///
    /// # Returns
    /// A string containing the padding text
    fn generate_padding(&self, length: usize) -> String {
        if length == 0 {
            return String::new();
        }
        let conf = CLEWDR_CONFIG.load();
        let tokens = conf
            .pad_tokens
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        assert!(tokens.len() >= length, "Padding tokens too short");

        let mut result = String::with_capacity(length * 8);
        let mut rng = rng();
        let mut pushed = 0;
        loop {
            let slice_len = rng.random_range(16..64);
            let slice_start = rng.random_range(0..tokens.len() - slice_len);
            let slice = &tokens[slice_start..slice_start + slice_len];
            result.push_str(slice.join("").as_str());
            pushed += slice_len;
            result.push('\n');
            if rng.random_range(0..100) < 5 {
                result.push('\n');
            }
            if pushed > length {
                break;
            }
        }
        print_out_text(result.as_str(), "padding.txt");
        result.push_str("\n\n\n\n------------------------------------------------------------\n");
        result
    }
}

/// Merges system message content into a single string
/// Handles both string and array formats for system messages
///
/// # Arguments
/// * `sys` - System message content as a JSON Value
///
/// # Returns
/// Merged system message as a string
fn merge_system(sys: Value) -> String {
    if let Some(str) = sys.as_str() {
        return str.to_string();
    }
    let Some(arr) = sys.as_array() else {
        return String::new();
    };
    arr.iter()
        .map_while(|v| v["text"].as_str())
        .map(|v| v.trim())
        .to_owned()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Merges server-sent events (SSE) from a stream into a single string
/// Extracts and concatenates completion data from events
///
/// # Arguments
/// * `stream` - Event stream to process
///
/// # Returns
/// Combined completion text from all events
pub async fn merge_sse(
    stream: EventStream<impl Stream<Item = Result<Bytes, rquest::Error>>>,
) -> String {
    #[derive(Deserialize)]
    struct Data {
        completion: String,
    }
    pin_mut!(stream);
    let mut w = String::new();
    while let Some(event) = stream.next().await {
        let Ok(event) = event else {
            continue;
        };
        let data = event.data;
        let Ok(data) = serde_json::from_str::<Data>(&data) else {
            continue;
        };
        w += data.completion.as_str();
    }
    w
}
