use std::{mem, sync::LazyLock};

use axum::{
    Json,
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use colored::Colorize;
use eventsource_stream::Eventsource;
use rquest::{Method, header::ACCEPT};
use scopeguard::defer;
use serde_json::json;
use tokio::spawn;
use tracing::{debug, error, info, warn};

use crate::{
    api::body::non_stream_message,
    config::CLEWDR_CONFIG,
    error::{ClewdrError, check_res_err},
    state::ClientState,
    types::message::{ContentBlock, Message, Role},
    utils::text::merge_sse,
    utils::{print_out_json, print_out_text},
};

use super::body::{ClientRequestBody, XApiKey};

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
    State(state): State<ClientState>,
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

    let stream = p.stream;
    info!(
        "Request received, stream mode: {}, messages: {}, model: {}",
        stream.to_string().green(),
        p.messages.len().to_string().green(),
        p.model.as_str().green()
    );
    for i in 0..CLEWDR_CONFIG.load().max_retries {
        if i > 0 {
            info!("Retrying request, attempt: {}", (i + 1).to_string().green());
        }
        let mut state = state.clone();
        let p = p.clone();
        let stopwatch = chrono::Utc::now();

        state.request_cookie().await?;

        let mut state_clone = state.clone();
        defer! {
            // ensure the cookie is returned
            spawn(async move {
                let dur = chrono::Utc::now().signed_duration_since(stopwatch);
                info!(
                    "Request finished, elapsed time: {} seconds",
                    dur.num_seconds().to_string().green()
                );
                state_clone.return_cookie(None).await;
            });
        }
        // check if request is successful
        match state.bootstrap().await.and(state.try_message(p).await) {
            Ok(b) => {
                if let Err(e) = state.clean_chat().await {
                    warn!("Failed to delete chat: {}", e);
                }
                return Ok(b);
            }
            Err(e) => {
                // delete chat after an error
                if let Err(e) = state.clean_chat().await {
                    warn!("Failed to delete chat: {}", e);
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

impl ClientState {
    /// Tries to send a message to the Claude API
    /// Creates a new conversation, processes the request, and returns the response
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - Response from Claude or error
    async fn try_message(&mut self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        print_out_json(&p, "0.req.json");
        let stream = p.stream;
        let org_uuid = self.org_uuid.clone().ok_or(ClewdrError::UnexpectedNone)?;

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
            body["model"] = p.model.clone().into();
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
            .transform_anthropic(p)
            .ok_or(ClewdrError::BadRequest("Empty request".to_string()))?;

        // check images
        let images = mem::take(&mut body.images);

        // upload images
        let files = self.upload_images(images).await;
        body.files = files;

        // send the request
        print_out_json(&body, "4.req.json");
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
        let api_res = check_res_err(api_res).await?;

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
