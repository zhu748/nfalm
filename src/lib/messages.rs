use std::{fmt::Debug, mem, sync::LazyLock};

use axum::{
    Json,
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use rquest::header::ACCEPT;
use scopeguard::defer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::spawn;
use tracing::{debug, warn};

use crate::{
    client::{AppendHeaders, SUPER_CLIENT, upload_images},
    config::UselessReason,
    error::{ClewdrError, check_res_err, error_stream},
    state::AppState,
    types::message::{ContentBlock, ImageSource, Message, Role},
    utils::print_out_json,
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
            file_size: content.bytes().len() as u64,
            extracted_content: content,
            file_name: "paste.txt".to_string(),
            file_type: "txt".to_string(),
        }
    }
}

/// Request body to be sent to the Claude.ai
#[derive(Deserialize, Serialize, Debug)]
pub struct RequestBody {
    pub max_tokens_to_sample: Option<u64>,
    pub attachments: Vec<Attachment>,
    pub files: Vec<String>,
    pub model: String,
    pub rendering_mode: String,
    pub prompt: String,
    pub timezone: String,
    #[serde(skip)]
    pub images: Vec<ImageSource>,
}

/// Request body sent from the client
#[derive(Deserialize, Serialize, Debug)]
pub struct ClientRequestBody {
    pub max_tokens: Option<u64>,
    pub messages: Vec<Message>,
    pub stop_sequences: Vec<String>,
    pub model: String,
    #[serde(default)]
    pub stream: bool,
    pub thinking: Option<Thinking>,
    #[serde(default)]
    pub system: String,
}

/// Thinking mode in Claude API Request
#[derive(Deserialize, Serialize, Debug)]
pub struct Thinking {
    budget_tokens: u64,
    r#type: String,
}

/// Axum handler for the API messages
pub async fn api_messages(
    State(state): State<AppState>,
    Json(p): Json<ClientRequestBody>,
) -> Response {
    let stream = p.stream;

    // check if request is successful
    match state.try_message(p).await {
        Ok(b) => {
            // delete chat after a successful request
            defer! {
                spawn(async move {
                    if let Err(e) = state.delete_chat().await {
                        warn!("Failed to delete chat: {:?}", e);
                    }
                    // increment the request count
                    state.increase_cons_requests();
                });
            }
            b.into_response()
        }
        Err(e) => {
            // delete chat after an error
            if let Err(e) = state.delete_chat().await {
                warn!("Failed to delete chat: {:?}", e);
            }
            // 429 error
            if let ClewdrError::TooManyRequest(i) = e {
                state.cookie_rotate(UselessReason::Exhausted(i));
            }
            warn!("Error: {:?}", e);
            if stream {
                // stream the error as a response
                Body::from_stream(error_stream(e)).into_response()
            } else {
                // return the error as a response
                serde_json::ser::to_string(&Message::new_text(
                    Role::Assistant,
                    format!("Error: {}", e),
                ))
                .unwrap()
                .into_response()
            }
        }
    }
}

impl AppState {
    /// Try to send a message to the Claude API
    async fn try_message(&self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        print_out_json(&p, "0.req.json");
        let stream = p.stream;
        let proxy = self.config.read().rquest_proxy.clone();

        // Check if the request is a test message
        if !p.stream && p.messages == vec![TEST_MESSAGE.clone()] {
            // respond with a test message
            return Ok(serde_json::ser::to_string(&Message::new_text(
                Role::Assistant,
                "Test message".to_string(),
            ))
            .unwrap()
            .into_response());
        }

        // Create a new conversation
        let new_uuid = uuid::Uuid::new_v4().to_string();
        *self.conv_uuid.write() = Some(new_uuid.to_string());
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations",
            self.config.read().endpoint(),
            self.uuid_org.read()
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
            .append_headers("", &self.header_cookie()?, proxy.clone())
            .send()
            .await?;
        debug!("New conversation created: {}", new_uuid);

        // update cookie
        self.update_cookie_from_res(&api_res);
        check_res_err(api_res).await?;

        // generate the request body
        // check if the request is empty
        let Some(mut body) = self.transform(p) else {
            return Ok(serde_json::ser::to_string(&Message::new_text(
                Role::Assistant,
                "Empty message?".to_string(),
            ))
            .unwrap()
            .into_response());
        };

        // check images
        let images = mem::take(&mut body.images);

        // upload images
        let uuid_org = self.uuid_org.read().clone();
        let files = upload_images(images, self.header_cookie()?, uuid_org, proxy.clone()).await;
        body.files = files;

        // send the request
        print_out_json(&body, "4.req.json");
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}/completion",
            self.config.read().endpoint(),
            self.uuid_org.read(),
            new_uuid
        );

        let api_res = SUPER_CLIENT
            .post(endpoint)
            .json(&body)
            .append_headers("", self.header_cookie()?, proxy.clone())
            .header_append(ACCEPT, "text/event-stream")
            .send()
            .await?;
        self.update_cookie_from_res(&api_res);
        let api_res = check_res_err(api_res).await?;

        // if not streaming, return the response
        if !stream {
            let text = api_res.text().await?;
            return Ok(text.into_response());
        }

        // stream the response
        let input_stream = api_res.bytes_stream();
        Ok(Body::from_stream(input_stream).into_response())
    }
}
