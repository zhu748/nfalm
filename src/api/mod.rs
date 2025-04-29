pub mod body;
mod config;
mod messages;
mod misc;
mod openai;

pub use config::api_get_config;
pub use config::api_post_config;
pub use messages::api_messages;
pub use misc::api_auth;
pub use misc::api_delete_cookie;
pub use misc::api_get_cookies;
pub use misc::api_post_cookie;
pub use misc::api_version;
pub use openai::api_completion;

use openai::NonStreamEventData;
use openai::transform_stream;

use std::mem;

use axum::{
    Json,
    body::Body,
    response::{IntoResponse, Sse},
};
use body::{ClientRequestBody, non_stream_message};
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures::TryFutureExt;
use rquest::{Method, Response, header::ACCEPT};
use scopeguard::defer;
use serde_json::json;
use strum::Display;
use tokio::spawn;
use tracing::{debug, error, info, warn};

use crate::config::CLEWDR_CONFIG;
use crate::error::ClewdrError;
use crate::error::check_res_err;
use crate::state::ClientState;
use crate::utils::enabled;
use crate::utils::print_out_json;
use crate::utils::print_out_text;
use crate::utils::text::merge_sse;

#[derive(Display, Clone)]
pub enum ApiFormat {
    Claude,
    OpenAI,
}

impl ClientState {
    /// Converts the response from the Claude Web into Claude API or OpenAI API format
    ///
    /// # Arguments
    /// * `input` - The response from the Claude Web
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - Response from Claude or error
    pub async fn transform_response(
        &self,
        input: Response,
    ) -> Result<axum::response::Response, ClewdrError> {
        // if not streaming, return the response
        if !self.stream {
            let stream = input.bytes_stream().eventsource();
            let text = merge_sse(stream).await;
            print_out_text(&text, "non_stream.txt");
            match self.api_format {
                ApiFormat::Claude => return Ok(Json(non_stream_message(text)).into_response()),
                ApiFormat::OpenAI => return Ok(Json(NonStreamEventData::new(text)).into_response()),
            }
        }

        // stream the response
        let input_stream = input.bytes_stream();
        match self.api_format {
            ApiFormat::Claude => Ok(Body::from_stream(input_stream).into_response()),
            ApiFormat::OpenAI => {
                let input_stream = input_stream.eventsource();
                let output = transform_stream(input_stream);
                Ok(Sse::new(output).into_response())
            }
        }
    }

    pub async fn try_chat(
        &mut self,
        p: ClientRequestBody,
    ) -> Result<axum::response::Response, ClewdrError> {
        let stream = p.stream;
        let format_display = match self.api_format {
            ApiFormat::Claude => self.api_format.to_string().green(),
            ApiFormat::OpenAI => self.api_format.to_string().yellow(),
        };
        info!(
            "[REQ] stream: {}, msgs: {}, model: {}, think: {}, format: {}",
            enabled(stream),
            p.messages.len().to_string().green(),
            p.model.green(),
            enabled(p.thinking.is_some()),
            format_display
        );
        for i in 0..CLEWDR_CONFIG.load().max_retries {
            if i > 0 {
                info!("[RETRY] attempt: {}", (i + 1).to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();
            let stopwatch = chrono::Utc::now();

            state.request_cookie().await?;

            let mut defer_clone = state.to_owned();
            defer! {
                // ensure the cookie is returned
                spawn(async move {
                    let dur = chrono::Utc::now().signed_duration_since(stopwatch);
                    info!(
                        "[FIN] elapsed time: {} seconds",
                        dur.num_seconds().to_string().green()
                    );
                    defer_clone.return_cookie(None).await;
                });
            }
            // check if request is successful
            let transform_clone = state.to_owned();
            let web_res = async { state.bootstrap().await.and(state.send_chat(p).await) };
            match web_res
                .and_then(|r| transform_clone.transform_response(r))
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

    /// Tries to send a message to the Claude API
    /// Creates a new conversation, processes the request,
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - Response from Claude or error
    pub async fn send_chat(&mut self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
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
            "name":""
        });

        // enable thinking mode
        if p.thinking.is_some() && self.is_pro() {
            body["paprika_mode"] = "extended".into();
            body["model"] = p.model.to_owned().into();
        }
        let api_res = self
            .request(Method::POST, endpoint)
            .json(&body)
            .send()
            .await?;
        self.update_cookie_from_res(&api_res);

        check_res_err(api_res).await?;
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

        let api_res = self
            .request(Method::POST, endpoint)
            .json(&body)
            .header_append(ACCEPT, "text/event-stream")
            .send()
            .await?;
        self.update_cookie_from_res(&api_res);
        check_res_err(api_res).await
    }
}
