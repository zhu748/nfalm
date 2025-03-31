use std::{fmt::Debug, mem, sync::LazyLock};

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
    client::{AppendHeaders, SUPER_CLIENT, upload_images},
    config::UselessReason,
    error::{ClewdrError, check_res_err},
    state::AppState,
    text::merge_messages,
    types::message::{ContentBlock, ImageSource, Message, Role},
    utils::{TIME_ZONE, print_out_json},
};

pub static TEST_MESSAGE: LazyLock<Message> = LazyLock::new(|| {
    Message::new_blocks(
        Role::User,
        vec![ContentBlock::Text {
            text: "Hi".to_string(),
        }],
    )
});

#[derive(Deserialize, Serialize, Debug)]
struct Attachment {
    extracted_content: String,
    file_name: String,
    file_type: String,
    file_size: u64,
}

impl Attachment {
    fn new(content: String) -> Self {
        Attachment {
            file_size: content.bytes().len() as u64,
            extracted_content: content,
            file_name: "paste.txt".to_string(),
            file_type: "txt".to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestBody {
    max_tokens_to_sample: Option<u64>,
    attachments: Vec<Attachment>,
    files: Vec<String>,
    model: String,
    rendering_mode: String,
    prompt: String,
    timezone: String,
    #[serde(skip)]
    images: Vec<ImageSource>,
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

fn transform(value: ClientRequestBody, user_real_roles: bool) -> Option<RequestBody> {
    let merged = merge_messages(value.messages, user_real_roles)?;
    Some(RequestBody {
        max_tokens_to_sample: value.max_tokens,
        attachments: vec![Attachment::new(merged.paste)],
        files: vec![],
        model: value.model,
        rendering_mode: "messages".to_string(),
        prompt: merged._prompt,
        timezone: TIME_ZONE.to_string(),
        images: merged.images,
    })
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
            serde_json::ser::to_string(&Message::new_text(
                Role::Assistant,
                format!("Error: {:?}", e),
            ))
            .unwrap()
            .into_response()
        }
    }
}

impl AppState {
    async fn try_message(&self, p: ClientRequestBody) -> Result<Response, ClewdrError> {
        let s = self.0.clone();
        print_out_json(&p, "0.req.json");

        // Check if the request is a test message
        if !p.stream && p.messages == vec![TEST_MESSAGE.clone()] {
            return Ok(serde_json::ser::to_string(&Message::new_text(
                Role::Assistant,
                "Test message".to_string(),
            ))
            .unwrap()
            .into_response());
        }

        // delete the previous conversation if it exists
        self.delete_chat().await?;
        debug!("Chat deleted");

        // Create a new conversation
        *s.conv_uuid.write() = Some(uuid::Uuid::new_v4().to_string());
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

        // prepare the request
        let user_real_roles = s.config.read().user_real_roles;
        let Some(mut body) = transform(p, user_real_roles) else {
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
        let uuid_org = s.uuid_org.read().clone();
        let files = upload_images(images, self.header_cookie()?, uuid_org).await;
        body.files = files;

        // file processed
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
