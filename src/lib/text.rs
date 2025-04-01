use itertools::Itertools;
use std::fmt::Write;

use crate::{
    messages::{Attachment, ClientRequestBody, RequestBody},
    state::AppState,
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

impl AppState {
    /// Transform the request body from Claude API to Claude web
    pub fn transform(&self, value: ClientRequestBody) -> Option<RequestBody> {
        let merged = self.merge_messages(value.messages, value.system)?;
        Some(RequestBody {
            max_tokens_to_sample: value.max_tokens,
            attachments: vec![Attachment::new(merged.paste)],
            files: vec![],
            model: value.model,
            rendering_mode: "messages".to_string(),
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
        let h = self
            .config
            .read()
            .custom_h
            .clone()
            .unwrap_or("Human".to_string());
        let a = self
            .config
            .read()
            .custom_a
            .clone()
            .unwrap_or("Assistant".to_string());
        let custom_prompt = self.config.read().custom_prompt.clone();
        let user_real_roles = self.config.read().user_real_roles;
        let line_breaks = if user_real_roles { "\n\n\x08" } else { "\n\n" };
        let system = system.trim().to_string();
        let size = size_of_val(&msgs);
        // preallocate string to avoid reallocations
        let mut w = String::with_capacity(size);
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
            .chunk_by(|m| m.0.clone());
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
                Role::User => format!("{}: ", h),
                Role::Assistant => format!("{}: ", a),
            };
            write!(w, "{}{}{}", line_breaks, prefix, text).unwrap();
        }
        print_out_text(w.as_str(), "paste.txt");

        Some(Merged {
            paste: w,
            prompt: custom_prompt,
            images: imgs,
        })
    }
}
