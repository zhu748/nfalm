pub mod body;
mod config;
mod messages;
mod misc;
mod openai;

// Re-exports

use bytes::Bytes;
// Configuration related endpoints
pub use config::{api_get_config, api_post_config};
// Message handling endpoints
pub use messages::api_messages;
// Miscellaneous endpoints
pub use misc::{api_auth, api_delete_cookie, api_get_cookies, api_post_cookie, api_version};
// OpenAI compatibility endpoints
pub use openai::api_completion;

// Internal imports from OpenAI module
use openai::{NonStreamEventData, transform_stream};

use std::{
    mem,
    sync::{Arc, Mutex},
};

use axum::{
    Json,
    body::Body,
    response::{IntoResponse, Sse},
};
use body::non_stream_message;
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures::{Stream, TryFutureExt};
use rquest::{Method, Response, header::ACCEPT};
use scopeguard::defer;
use serde_json::json;
use strum::Display;
use tokio::spawn;
use tracing::{debug, error, info, warn};

use crate::{
    config::CLEWDR_CONFIG,
    error::{CheckResErr, ClewdrError},
    services::cache::{CACHE, CachedResponse, stream_to_vec, vec_to_stream},
    state::ClientState,
    types::message::CreateMessageParams,
    utils::{enabled, print_out_json, print_out_text, text::merge_sse},
};

/// Represents the format of the API response
#[derive(Display, Clone, Copy)]
pub enum ApiFormat {
    /// Claude native format
    Claude,
    /// OpenAI compatible format
    OpenAI,
}

impl ClientState {
    pub async fn try_from_cache(
        &mut self,
        p: CreateMessageParams,
        key: u64,
    ) -> Option<axum::response::Response> {
        if let Some(value) = CACHE.get(&key).await {
            let (vec, empty) = {
                let mut value = value.lock().unwrap();
                (value.pop(), value.is_empty())
            };
            if empty {
                debug!("Cache is empty for key: {}", key);
                // remove the cache entry
                CACHE.invalidate(&key).await;
            }
            if let Some(vec) = vec {
                info!("Cache hit for key: {}", key);
                let byte_stream = vec_to_stream(vec);
                return self
                    .transform_response(byte_stream, self.api_format, self.stream)
                    .await
                    .ok();
            }
        }
        for id in 0..CLEWDR_CONFIG.load().max_cache {
            let mut state = self.to_owned();
            state.key = Some((key, id));
            let p = p.to_owned();
            spawn(async move { state.try_chat(p).await });
        }
        None
    }

    async fn cache_response(
        &self,
        stream: impl Stream<Item = Result<Bytes, rquest::Error>>,
        key: u64,
        id: usize,
    ) {
        let vec = stream_to_vec(stream).await;
        let value = CACHE
            .get_with(key, async {
                Arc::new(Mutex::new(CachedResponse::default()))
            })
            .await;
        let mut value = value.lock().unwrap();
        if value.len() >= CLEWDR_CONFIG.load().max_cache {
            debug!("Cache is full, skipping cache for key: {}", key);
            return;
        }
        info!("[CACHE {}] cache response for key: {}", id, key);
        value.push(vec);
    }
    /// Converts the response from the Claude Web into Claude API or OpenAI API format
    ///
    /// # Arguments
    /// * `input` - The response from the Claude Web
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - Response from Claude or error
    pub async fn transform_response(
        &self,
        input: impl Stream<Item = Result<Bytes, rquest::Error>> + Send + 'static,
        api_format: ApiFormat,
        stream: bool,
    ) -> Result<axum::response::Response, ClewdrError> {
        if let Some((key, id)) = self.key {
            self.cache_response(input, key, id).await;
            return Ok(Body::empty().into_response());
        }
        // if not streaming, return the response
        if !stream {
            let stream = input.eventsource();
            let text = merge_sse(stream).await;
            print_out_text(&text, "non_stream.txt");
            match api_format {
                ApiFormat::Claude => return Ok(Json(non_stream_message(text)).into_response()),
                ApiFormat::OpenAI => return Ok(Json(NonStreamEventData::new(text)).into_response()),
            }
        }

        // stream the response
        match api_format {
            ApiFormat::Claude => Ok(Body::from_stream(input).into_response()),
            ApiFormat::OpenAI => {
                let input_stream = input.eventsource();
                let output = transform_stream(input_stream);
                Ok(Sse::new(output).into_response())
            }
        }
    }

    /// Attempts to send a chat message to Claude API with retry mechanism
    ///
    /// This method handles the complete chat flow including:
    /// - Request preparation and logging
    /// - Cookie management for authentication
    /// - Executing the chat request with automatic retries on failure
    /// - Response transformation according to the specified API format
    /// - Error handling and cleanup
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<axum::response::Response, ClewdrError>` - Formatted response or error
    pub(self) async fn try_chat(
        &mut self,
        p: CreateMessageParams,
    ) -> Result<axum::response::Response, ClewdrError> {
        let api_format = self.api_format;
        let stream = p.stream.unwrap_or_default();
        let format_display = match api_format {
            ApiFormat::Claude => api_format.to_string().green(),
            ApiFormat::OpenAI => api_format.to_string().yellow(),
        };
        info!(
            "[REQ] stream: {}, msgs: {}, model: {}, think: {}, format: {}",
            enabled(stream),
            p.messages.len().to_string().green(),
            p.model.green(),
            enabled(p.thinking.is_some()),
            format_display
        );
        let stopwatch = chrono::Utc::now();
        defer!(
            let elapsed = chrono::Utc::now().signed_duration_since(stopwatch);
            info!(
                "[FIN] elapsed: {}s",
                format!("{}", elapsed.num_milliseconds() as f64 / 1000.0).green()
            );
        );
        for i in 0..CLEWDR_CONFIG.load().max_retries {
            if i > 0 {
                info!("[RETRY] attempt: {}", (i + 1).to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();

            state.request_cookie().await?;

            let mut defer_clone = state.to_owned();
            defer! {
                // ensure the cookie is returned
                spawn(async move {
                    defer_clone.return_cookie(None).await;
                });
            }
            // check if request is successful
            let web_res = async { state.bootstrap().await.and(state.send_chat(p).await) };
            match web_res
                .and_then(|r| self.transform_response(r.bytes_stream(), api_format, stream))
                .await
            {
                Ok(b) => {
                    if let Err(e) = state.clean_chat().await {
                        warn!("Failed to clean chat: {}", e);
                    }
                    return Ok(b);
                }
                Err(e) => {
                    // delete chat after an error
                    if let Err(e) = state.clean_chat().await {
                        warn!("Failed to clean chat: {}", e);
                    }
                    error!("{}", e);
                    // 429 error
                    if let ClewdrError::InvalidCookie(ref r) = e {
                        state.return_cookie(Some(r.to_owned())).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        error!("Max retries exceeded");
        Err(ClewdrError::TooManyRetries)
    }

    /// Sends a message to the Claude API by creating a new conversation and processing the request
    ///
    /// This method performs several key operations:
    /// - Creates a new conversation with a unique UUID
    /// - Configures thinking mode if applicable
    /// - Transforms the client request to the Claude API format
    /// - Handles image uploads if present
    /// - Sends the request to the Claude API endpoint
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - Response from Claude or error
    async fn send_chat(&mut self, p: CreateMessageParams) -> Result<Response, ClewdrError> {
        print_out_json(&p, "client_req.json");
        let org_uuid = self
            .org_uuid
            .to_owned()
            .ok_or(ClewdrError::UnexpectedNone)?;

        // Create a new conversation
        let new_uuid = uuid::Uuid::new_v4().to_string();
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations",
            self.endpoint, org_uuid
        );
        let mut body = json!({
            "uuid": new_uuid,
            "name": format!("ClewdR-{}", new_uuid),
        });

        // enable thinking mode
        if p.thinking.is_some() && self.is_pro() {
            body["paprika_mode"] = "extended".into();
            body["model"] = p.model.to_owned().into();
        }
        self.build_request(Method::POST, endpoint)
            .json(&body)
            .send()
            .await?
            .check()
            .await?;
        self.conv_uuid = Some(new_uuid.to_string());
        debug!("New conversation created: {}", new_uuid);

        // generate the request body
        // check if the request is empty
        let mut body = self
            .transform_request(p)
            .ok_or(ClewdrError::BadRequest("Empty request".to_string()))?;

        // check images
        let images = mem::take(&mut body.images);

        // upload images
        let files = self.upload_images(images).await;
        body.files = files;

        // send the request
        print_out_json(&body, "clewdr_req.json");
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}/completion",
            self.endpoint, org_uuid, new_uuid
        );

        self.build_request(Method::POST, endpoint)
            .json(&body)
            .header_append(ACCEPT, "text/event-stream")
            .send()
            .await?
            .check()
            .await
    }
}
