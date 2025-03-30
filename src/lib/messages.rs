use std::{fmt::Debug, sync::LazyLock};

use axum::{
    Json,
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use rquest::header::ACCEPT;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::{
    client::{AppendHeaders, SUPER_CLIENT},
    config::UselessReason,
    error::{ClewdrError, check_res_err},
    state::AppState,
    types::message::{ContentBlock, Message, MessageContent, Role},
    utils::{TIME_ZONE, print_out_json},
};

pub static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| Message::new_text(Role::User, "Hi!"));

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestBody {
    files: Vec<Value>,
    model: String,
    rendering_mode: String,
    prompt: String,
    timezone: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientRequestBody {
    max_tokens: Option<u64>,
    messages: Vec<Message>,
    stop_sequences: Vec<String>,
    model: String,
    #[serde(default)]
    stream: bool,
    thinking: Option<Thinking>,
    #[serde(default)]
    system: Value,
}

impl From<ClientRequestBody> for RequestBody {
    fn from(value: ClientRequestBody) -> Self {
        Self {
            files: vec![],
            model: value.model,
            rendering_mode: "messages".to_string(),
            prompt: value
                .messages
                .iter()
                .map(|m| match &m.content {
                    MessageContent::Text { content } => content.clone(),
                    MessageContent::Blocks { content } => content
                        .iter()
                        .map_while(|b| match b {
                            ContentBlock::Text { text } => Some(text),
                            _ => None,
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n"),
                })
                .collect::<Vec<_>>()
                .join("\n"),
            timezone: TIME_ZONE.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct Thinking {
    budget_tokens: u64,
    r#type: String,
}

pub async fn api_messages(
    State(state): State<AppState>,
    Json(p): Json<ClientRequestBody>,
) -> Response {
    match state.try_message(p).await {
        Ok(b) => b.into_response(),
        Err(e) => {
            warn!("Error: {:?}", e);
            e.to_string().into_response()
        }
    }
}

impl AppState {
    async fn try_message(&self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        let s = self.0.clone();
        print_out_json(&p, "0.req.json");

        // Check if the request is a test message
        if !p.stream
            && p.messages.len() == 1
            && p.messages.first().map(|m| format!("{:?}", m)) == Some(format!("{:?}", TEST_MESSAGE))
        {
            return Ok(json!({
                "content": [
                    {
                        "text": "Hi! My name is Doge.",
                        "type": "text"
                    }
                ],
            })
            .to_string()
            .into_response());
        }

        // delete the previous conversation if it exists
        let uuid = s.conv_uuid.read().clone();
        if let Some(uuid) = uuid {
            self.delete_chat(uuid).await?;
        }
        debug!("Chat deleted");

        // Create a new conversation
        *s.conv_uuid.write() = Some(uuid::Uuid::new_v4().to_string());
        *s.conv_depth.write() = 0;
        let endpoint = s.config.read().endpoint("");
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations",
            endpoint,
            s.uuid_org.read()
        );
        let mut body = json!({
            "uuid": s.conv_uuid.read().as_ref().unwrap(),
            "name":""
        });
        if p.thinking.is_some() {
            body["paprika_mode"] = "extended".into();
            body["model"] = p.model.clone().into();
        }
        let api_res = SUPER_CLIENT
            .post(endpoint)
            .json(&body)
            .append_headers("", &self.header_cookie()?)
            .send()
            .await?;
        debug!("New conversation created");
        self.update_cookie_from_res(&api_res);
        check_res_err(api_res).await?;

        // send the request
        let body: RequestBody = p.into();
        print_out_json(&body, "4.req.json");
        let endpoint = s.config.read().endpoint("");
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}/completion",
            endpoint,
            s.uuid_org.read(),
            s.conv_uuid.read().as_ref().cloned().unwrap_or_default()
        );

        let api_res = SUPER_CLIENT
            .post(endpoint)
            .json(&body)
            .append_headers("", self.header_cookie()?)
            .header_append(ACCEPT, "text/event-stream")
            .send()
            .await?;
        self.update_cookie_from_res(&api_res);
        let api_res = check_res_err(api_res).await.inspect_err(|e| {
            if let ClewdrError::TooManyRequest(_, i) = e {
                self.cookie_rotate(UselessReason::Temporary(*i));
            }
        })?;

        // stream the response
        let input_stream = api_res.bytes_stream();
        Ok(Body::from_stream(input_stream).into_response())
    }
}
