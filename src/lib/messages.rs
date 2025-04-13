use std::{fmt::Debug, mem, sync::LazyLock};

use axum::{
    Json,
    body::Body,
    extract::{FromRequestParts, State},
    response::{IntoResponse, Response},
};
use colored::Colorize;
use eventsource_stream::Eventsource;
use rquest::{StatusCode, header::ACCEPT};
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::spawn;
use tracing::{debug, error, info, warn};

use crate::{
    client::{SUPER_CLIENT, SetupRequest},
    error::{ClewdrError, check_res_err},
    state::AppState,
    text::merge_sse,
    types::message::{ContentBlock, ImageSource, Message, Role},
    utils::{print_out_json, print_out_text},
};

/// Exact test message send by SillyTavern
pub static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| {
    Message::new_blocks(
        Role::User,
        vec![ContentBlock::Text {
            text: "Hi".to_string(),
        }],
    )
});

/// Claude.ai attachment
#[derive(Deserialize, Serialize, Debug)]
pub struct Attachment {
    extracted_content: String,
    file_name: String,
    file_type: String,
    file_size: u64,
}

impl Attachment {
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
    pub model: Option<String>,
    pub rendering_mode: String,
    pub prompt: String,
    pub timezone: String,
    #[serde(skip)]
    pub images: Vec<ImageSource>,
}

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

pub struct Auth(pub String);

impl FromRequestParts<AppState> for Auth {
    type Rejection = StatusCode;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !state.config.auth(key) {
            warn!("Invalid password: {}", key);
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(Auth(key.to_string()))
    }
}

/// Axum handler for the API messages
pub async fn api_messages(
    Auth(_): Auth,
    State(state): State<AppState>,
    Json(p): Json<ClientRequestBody>,
) -> Response {
    // Check if the request is a test message
    if !p.stream && p.messages == vec![TEST_MESSAGE.clone()] {
        // respond with a test message
        return Json(non_stream_message(
            "Claude Reverse Proxy is working, please send a real message.".to_string(),
        ))
        .into_response();
    }

    let stream = p.stream;
    info!(
        "Request received, stream mode: {}, messages: {}, model: {}",
        stream.to_string().green(),
        p.messages.len().to_string().green(),
        p.model.as_str().green()
    );
    for i in 0..state.config.max_retries {
        if i > 0 {
            info!("Retrying request, attempt: {}", (i + 1).to_string().green());
        }
        let mut state = state.clone();
        let p = p.clone();
        let stopwatch = chrono::Utc::now();

        if let Err(e) = state.request_cookie().await {
            return Body::from_stream(e.error_stream()).into_response();
        }
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
                if let Err(e) = state.delete_chat().await {
                    warn!("Failed to delete chat: {}", e);
                }
                return b.into_response();
            }
            Err(e) => {
                // delete chat after an error
                if let Err(e) = state.delete_chat().await {
                    warn!("Failed to delete chat: {}", e);
                }
                warn!("Error: {}", e);
                // 429 error
                match e {
                    ClewdrError::InvalidCookie(ref r) => {
                        state.return_cookie(Some(r.clone())).await;
                        continue;
                    }
                    ClewdrError::OtherHttpError(c, e) => {
                        state.return_cookie(None).await;
                        return (c, Json(e)).into_response();
                    }
                    _ => {
                        state.return_cookie(None).await;
                    }
                }
                if stream {
                    // stream the error as a response
                    return Body::from_stream(e.error_stream()).into_response();
                } else {
                    // return the error as a response
                    return Json(e.error_body()).into_response();
                }
            }
        }
    }
    error!("Max retries exceeded");
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ClewdrError::TooManyRetries.error_body()),
    )
        .into_response()
}

impl AppState {
    /// Try to send a message to the Claude API
    async fn try_message(&mut self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        print_out_json(&p, "0.req.json");
        let stream = p.stream;
        let proxy = self.config.rquest_proxy.clone();
        let Some(org_uuid) = self.org_uuid.clone() else {
            return Ok(Json(non_stream_message(
                "No organization found, please check your cookie.".to_string(),
            ))
            .into_response());
        };

        // Create a new conversation
        let new_uuid = uuid::Uuid::new_v4().to_string();
        self.conv_uuid = Some(new_uuid.to_string());
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations",
            self.config.endpoint(),
            org_uuid
        );
        let mut body = json!({
            "uuid": new_uuid,
            "name":""
        });

        // enable thinking mode
        if p.thinking.is_some() {
            body["paprika_mode"] = "extended".into();
            body["model"] = p.model.clone().into();
        }
        let api_res = SUPER_CLIENT
            .post(endpoint)
            .json(&body)
            .setup_request("", self.header_cookie(), proxy.clone())
            .send()
            .await?;
        self.update_cookie_from_res(&api_res);
        debug!("New conversation created: {}", new_uuid);

        check_res_err(api_res).await?;

        // generate the request body
        // check if the request is empty
        let Some(mut body) = self.transform_anthropic(p) else {
            return Ok(Json(non_stream_message(
                "Empty request, please send a message.".to_string(),
            ))
            .into_response());
        };

        // check images
        let images = mem::take(&mut body.images);

        // upload images
        let files = self.upload_images(images).await;
        body.files = files;

        // send the request
        print_out_json(&body, "4.req.json");
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}/completion",
            self.config.endpoint(),
            org_uuid,
            new_uuid
        );

        let api_res = SUPER_CLIENT
            .post(endpoint)
            .json(&body)
            .setup_request(new_uuid, self.header_cookie(), proxy)
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

/// Transform a string to a message
pub fn non_stream_message(str: String) -> Message {
    Message::new_blocks(Role::Assistant, vec![ContentBlock::Text { text: str }])
}
