use crate::{
    SUPER_CLIENT,
    api::{AppState, InnerState},
    stream::{ClewdrConfig, ClewdrTransformer},
    utils::{ClewdrError, TEST_MESSAGE},
};
use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::HeaderMap,
};
use bytes::Bytes;
use futures::pin_mut;
use serde::{de, ser};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tracing::info;

pub async fn stream_example(
    State(state): State<AppState>,
    header: HeaderMap,
    Json(payload): Json<Value>,
) -> Body {
    // Create a channel for streaming response chunks to the client
    let (tx, rx) = mpsc::channel::<Result<Bytes, axum::Error>>(32);

    // Configure the transformer
    let config = ClewdrConfig::new("xx", "pro", true, 8, true);
    let trans = ClewdrTransformer::new(config);

    // Perform the external request
    let super_res = SUPER_CLIENT
        .get("https://api.claude.ai")
        .send()
        .await
        .unwrap(); // In production, handle this error gracefully

    // Spawn a task to handle the streaming transformation
    tokio::spawn(async move {
        let input_stream = super_res.bytes_stream();
        let output_stream = trans.transform_stream(input_stream);
        pin_mut!(output_stream);

        while let Some(result) = output_stream.next().await {
            // Simulate expensive work (optional, adjust as needed)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Send the chunk to the client
            let chunk = Bytes::from(result.unwrap()); // Convert String to Bytes
            if tx.send(Ok(chunk)).await.is_err() {
                info!("Client disconnected, cancelling task");
                break;
            }
        }
    });

    // Return the streaming body
    let response_stream = ReceiverStream::new(rx);
    Body::from_stream(response_stream)
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ClientRequestInfo {
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(default)]
    messages: Vec<Message>,
    #[serde(default)]
    model: String,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<i64>,
    #[serde(default)]
    stop: Vec<String>,
    #[serde(default)]
    top_p: Option<f64>,
    #[serde(default)]
    top_k: Option<i64>,
}
impl ClientRequestInfo {
    fn sanitize_client_request(self) -> ClientRequestInfo {
        if let Some(mut temp) = self.temperature {
            temp = temp.clamp(0.0, 1.0);
        }
        self
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub customname: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub strip: Option<bool>,
    #[serde(default)]
    pub jailbreak: Option<bool>,
    #[serde(default)]
    pub main: Option<bool>,
    #[serde(default)]
    pub discard: Option<bool>,
    #[serde(default)]
    pub merged: Option<bool>,
    #[serde(default)]
    pub personality: Option<bool>,
    #[serde(default)]
    pub scenario: Option<bool>,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            role: "user".to_string(),
            content: "".to_string(),
            customname: None,
            name: None,
            strip: None,
            jailbreak: None,
            main: None,
            discard: None,
            merged: None,
            personality: None,
            scenario: None,
        }
    }
}

pub async fn completion(
    State(state): State<AppState>,
    header: HeaderMap,
    Json(payload): Json<ClientRequestInfo>,
) {
    let _ = state.try_completion(payload).await;
}

impl AppState {
    async fn try_completion(&self, payload: ClientRequestInfo) -> Result<(), ClewdrError> {
        // TODO: 3rd key, API key, auth token, etc.
        let s = self.0.as_ref();
        let p = payload.sanitize_client_request();
        *s.model.write() = if s.is_pro.read().is_some() {
            Some(p.model.replace("--force", "").trim().to_string())
        } else {
            s.cookie_model.read().clone()
        };
        if s.uuid_org.read().is_empty() {
            // TODO: more keys
            return Err(ClewdrError::NoValidKey);
        }
        if !*s.changing.read()
            && s.is_pro.read().is_none()
            && *s.model.read() != *s.cookie_model.read()
        {
            self.cookie_changer(None, None);
            self.wait_for_change().await;
        }
        if p.messages.is_empty() {
            return Err(ClewdrError::WrongCompletionFormat);
        }
        // if p.messages.first() == Some(&TEST_MESSAGE) {}
        Ok(())
    }
}
