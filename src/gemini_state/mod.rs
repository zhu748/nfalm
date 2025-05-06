use std::sync::LazyLock;

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use colored::Colorize;
use rquest::{Client, ClientBuilder, Proxy};
use tracing::{error, info};

use crate::{
    config::{CLEWDR_CONFIG, GEMINI_ENDPOINT, KeyStatus},
    error::ClewdrError,
    gemini_body::GeminiQuery,
    middleware::gemini::GeminiContext,
    services::key_manager::KeyEventSender,
    types::gemini::request::GeminiRequestBody,
};

#[derive(Clone)]
pub enum GeminiApiFormat {
    Gemini,
    OpenAI,
}

static DUMMY_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Clone)]
pub struct GeminiState {
    pub path: String,
    pub key: Option<KeyStatus>,
    pub stream: bool,
    pub query: GeminiQuery,
    pub fake_stream: bool,
    pub proxy: Option<Proxy>,
    pub event_sender: KeyEventSender,
    pub api_format: GeminiApiFormat,
    pub client: Client,
    pub cache_key: Option<(u64, usize)>,
}

impl GeminiState {
    /// Create a new AppState instance
    pub fn new(tx: KeyEventSender) -> Self {
        GeminiState {
            path: String::new(),
            query: GeminiQuery::default(),
            stream: false,
            key: None,
            fake_stream: false,
            event_sender: tx,
            proxy: None,
            api_format: GeminiApiFormat::Gemini,
            client: DUMMY_CLIENT.to_owned(),
            cache_key: None,
        }
    }

    pub async fn request_key(&mut self) -> Result<(), ClewdrError> {
        let key = self.event_sender.request().await?;
        self.key = Some(key.to_owned());
        let client = ClientBuilder::new();
        let client = if let Some(proxy) = self.proxy.to_owned() {
            client.proxy(proxy)
        } else {
            client
        };
        self.client = client.build()?;
        Ok(())
    }

    pub fn update_from_ctx(&mut self, ctx: &GeminiContext) {
        self.path = ctx.path.to_owned();
        self.stream = ctx.stream.to_owned();
        self.query = ctx.query.to_owned();
    }

    pub async fn send_chat(&mut self, p: GeminiRequestBody) -> Result<Response, ClewdrError> {
        self.request_key().await?;
        let Some(key) = self.key.clone() else {
            return Err(ClewdrError::UnexpectedNone);
        };
        let key = key.key.to_string();
        let mut query_vec = self.query.to_vec();
        query_vec.push(("key", key.as_str()));
        let res = self
            .client
            .post(format!("{}/v1beta/{}", GEMINI_ENDPOINT, self.path))
            .query(&query_vec)
            .json(&p)
            .send()
            .await?;
        let res = res.error_for_status()?;
        let body = if self.stream {
            let stream = res.bytes_stream();
            Body::from_stream(stream)
        } else {
            let bytes = res.bytes().await?;
            Body::from(bytes)
        };
        Ok(body.into_response())
    }

    pub async fn try_chat(&mut self, p: GeminiRequestBody) -> Result<Response, ClewdrError> {
        for i in 0..CLEWDR_CONFIG.load().max_retries + 1 {
            if i > 0 {
                info!("[RETRY] attempt: {}", i.to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();

            // check if request is successful
            match state.send_chat(p).await {
                Ok(b) => {
                    return Ok(b);
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
        error!("Max retries exceeded");
        Err(ClewdrError::TooManyRetries)
    }
}
