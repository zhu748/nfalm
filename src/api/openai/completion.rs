use std::mem;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response, Sse},
};
use axum_auth::AuthBearer;
use colored::Colorize;
use eventsource_stream::Eventsource;
use rquest::{Method, header::ACCEPT};
use scopeguard::defer;
use serde_json::json;
use tokio::spawn;
use tracing::{debug, error, info, warn};

use crate::{
    api::{body::ClientRequestBody, openai::stream::transform},
    config::CLEWDR_CONFIG,
    error::{ClewdrError, check_res_err},
    state::ClientState,
    utils::{print_out_json, print_out_text, text::merge_sse},
};

use super::stream::NonStreamEventData;

/// OpenAI-compatible API endpoint for chat completions
/// Handles authentication, processes messages, and supports both streaming and non-streaming responses
///
/// # Arguments
/// * `token` - Bearer token for API authentication
/// * `state` - Application state containing client information
/// * `p` - Request body containing messages and configuration
///
/// # Returns
/// * `Response` - JSON or stream response in OpenAI format
pub async fn api_completion(
    AuthBearer(token): AuthBearer,
    State(mut state): State<ClientState>,
    Json(p): Json<ClientRequestBody>,
) -> Result<Response, ClewdrError> {
    if !CLEWDR_CONFIG.load().v1_auth(&token) {
        return Err(ClewdrError::IncorrectKey);
    }
    // TODO: Check if the request is a test message

    let stream = p.stream;
    let stopwatch = chrono::Utc::now();
    info!(
        "Request received, stream mode: {}, messages: {}, model: {}",
        stream.to_string().green(),
        p.messages.len().to_string().green(),
        p.model.to_string().green()
    );

    for i in 0..CLEWDR_CONFIG.load().max_retries {
        let p = p.clone();
        if i > 0 {
            info!("Retrying request, attempt: {}", (i + 1).to_string().green());
        }
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
        match state.bootstrap().await.and(state.try_completion(p).await) {
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
    /// Sends a completion request to the Claude API in OpenAI-compatible format
    /// Creates a new conversation, processes the request, and returns formatted response
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<Response, ClewdrError>` - OpenAI-formatted response or error
    async fn try_completion(&mut self, mut p: ClientRequestBody) -> Result<Response, ClewdrError> {
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
        let actual_model = p.model.trim_end_matches("-thinking").to_string();
        if p.model.contains("-thinking") && self.is_pro() {
            body["paprika_mode"] = "extended".into();
            body["model"] = actual_model.clone().into();
        }
        p.model = actual_model;

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

        if !stream {
            let stream = api_res.bytes_stream().eventsource();
            let text = merge_sse(stream).await;
            print_out_text(&text, "non_stream.txt");
            return Ok(Json(NonStreamEventData::new(text)).into_response());
        }
        // stream the response
        let input_stream = api_res.bytes_stream().eventsource();
        let output = transform(input_stream);

        Ok(Sse::new(output).into_response())
    }
}
