use axum::body::Bytes;
use eventsource_stream::EventStream;
use futures::Stream;
use futures::StreamExt;
use futures::pin_mut;
use itertools::Itertools;
use rand::{Rng, rng};
use serde_json::Value;
use std::fmt::Write;
use tracing::error;
use tracing::warn;

use crate::config::CLEWDR_CONFIG;
use crate::{
    api::body::{Attachment, ClientRequestBody, RequestBody},
    state::ClientState,
    types::message::{ContentBlock, ImageSource, Message, MessageContent, Role},
    utils::{TIME_ZONE, print_out_text},
};

/// Merged messages and images
#[derive(Default, Debug)]
pub struct Merged {
    pub paste: String,
    pub prompt: String,
    pub images: Vec<ImageSource>,
}

impl ClientState {
    /// Transform the request body from Claude API to Claude web
    pub fn transform_anthropic(&self, value: ClientRequestBody) -> Option<RequestBody> {
        let system = merge_system(value.system);
        let merged = self.merge_messages(value.messages, system)?;
        Some(RequestBody {
            max_tokens_to_sample: value.max_tokens,
            attachments: vec![Attachment::new(merged.paste)],
            files: vec![],
            model: if self.is_pro() {
                Some(value.model)
            } else {
                None
            },
            rendering_mode: if value.stream {
                "messages".to_string()
            } else {
                "raw".to_string()
            },
            prompt: merged.prompt,
            timezone: TIME_ZONE.to_string(),
            images: merged.images,
        })
    }

    /// Transform the request body from Claude web to OAI API
    pub fn transform_oai(&self, mut value: ClientRequestBody) -> Option<RequestBody> {
        let mut role = value.messages.first().map(|m| m.role)?;
        for msg in value.messages.iter_mut() {
            if msg.role != Role::System {
                role = msg.role;
            } else {
                msg.role = role;
            }
        }
        let merged = self.merge_messages(value.messages, String::new())?;
        Some(RequestBody {
            max_tokens_to_sample: value.max_tokens,
            attachments: vec![Attachment::new(merged.paste)],
            files: vec![],
            model: if self.is_pro() {
                Some(value.model)
            } else {
                None
            },
            rendering_mode: "raw".to_string(),
            prompt: merged.prompt,
            timezone: TIME_ZONE.to_string(),
            images: merged.images,
        })
    }

    /// Merge messages into strings and extract images
    fn merge_messages(&self, msgs: Vec<Message>, system: String) -> Option<Merged> {
        if msgs.is_empty() {
            return None;
        }
        let h = CLEWDR_CONFIG
            .load()
            .custom_h
            .clone()
            .unwrap_or("Human".to_string());
        let a = CLEWDR_CONFIG
            .load()
            .custom_a
            .clone()
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
        let p = CLEWDR_CONFIG.load().custom_prompt.clone();

        Some(Merged {
            paste: w,
            prompt: p,
            images: imgs,
        })
    }

    /// Generate padding text
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

/// Merge system message into a string
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

pub async fn merge_sse(
    stream: EventStream<impl Stream<Item = Result<Bytes, rquest::Error>>>,
) -> String {
    pin_mut!(stream);
    let mut w = String::new();
    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                if event.event != "completion" {
                    continue;
                }
                let data = event.data;
                let Ok(json) = serde_json::from_str::<Value>(&data) else {
                    error!("Failed to parse JSON: {}", data);
                    continue;
                };
                let Some(completion) = json["completion"].as_str() else {
                    error!("Failed to get completion from JSON: {}", json);
                    continue;
                };
                w += completion;
            }
            Err(e) => error!("Stream Error: {}", e),
        }
    }
    w
}
