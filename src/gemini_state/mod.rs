use std::sync::LazyLock;

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use colored::Colorize;
use futures::Stream;
use rquest::{Client, ClientBuilder, header::AUTHORIZATION};
use tokio::spawn;
use tracing::{Instrument, Level, error, info, span};

use crate::{
    config::{CLEWDR_CONFIG, GEMINI_ENDPOINT, KeyStatus},
    error::ClewdrError,
    gemini_body::GeminiQuery,
    middleware::gemini::GeminiContext,
    services::{cache::CACHE, key_manager::KeyEventSender},
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
    pub model: String,
    pub vertex: bool,
    pub path: String,
    pub key: Option<KeyStatus>,
    pub stream: bool,
    pub query: GeminiQuery,
    pub fake_stream: bool,
    pub event_sender: KeyEventSender,
    pub api_format: GeminiApiFormat,
    pub client: Client,
    pub cache_key: Option<(u64, usize)>,
}

impl GeminiState {
    /// Create a new AppState instance
    pub fn new(tx: KeyEventSender) -> Self {
        GeminiState {
            model: String::new(),
            vertex: false,
            path: String::new(),
            query: GeminiQuery::default(),
            stream: false,
            key: None,
            fake_stream: false,
            event_sender: tx,
            api_format: GeminiApiFormat::Gemini,
            client: DUMMY_CLIENT.to_owned(),
            cache_key: None,
        }
    }

    pub async fn request_key(&mut self) -> Result<(), ClewdrError> {
        let key = self.event_sender.request().await?;
        self.key = Some(key.to_owned());
        let client = ClientBuilder::new();
        let client = if let Some(proxy) = CLEWDR_CONFIG.load().proxy.to_owned() {
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
        self.model = ctx.model.to_owned();
        self.vertex = ctx.vertex.to_owned();
    }

    async fn vertex_response(
        &mut self,
        p: GeminiRequestBody,
    ) -> Result<rquest::Response, ClewdrError> {
        let client = ClientBuilder::new();
        let client = if let Some(proxy) = CLEWDR_CONFIG.load().proxy.to_owned() {
            client.proxy(proxy)
        } else {
            client
        };
        self.client = client.build()?;
        let method = if self.stream {
            "streamGenerateContent"
        } else {
            "generateContent"
        };
        let mut json = serde_json::to_value(&CLEWDR_CONFIG.load().vertex)?;
        json["grant_type"] = "refresh_token".into();
        let res = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .json(&json)
            .send()
            .await?;
        let res = res.error_for_status()?;
        let res = res.json::<serde_json::Value>().await?;
        let access_token = res["access_token"]
            .as_str()
            .ok_or(ClewdrError::UnexpectedNone)?;
        let bearer = format!("Bearer {}", access_token);
        let endpoint = format!(
            "https://aiplatform.googleapis.com/v1/projects/{PROJECT_ID}/locations/global/publishers/google/models/{MODEL_ID}:{method}",
            PROJECT_ID = CLEWDR_CONFIG
                .load()
                .vertex
                .project_id
                .as_deref()
                .unwrap_or_default(),
            MODEL_ID = self.model
        );
        let query_vec = self.query.to_vec();
        let res = self
            .client
            .post(endpoint)
            .query(&query_vec)
            .header(AUTHORIZATION, bearer)
            .json(&p)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn send_chat(
        &mut self,
        p: GeminiRequestBody,
    ) -> Result<impl Stream<Item = Result<Bytes, rquest::Error>> + Send + 'static, ClewdrError>
    {
        if self.vertex {
            let res = self.vertex_response(p).await?;
            let stream = res.bytes_stream();
            return Ok(stream);
        }
        self.request_key().await?;
        let Some(key) = self.key.to_owned() else {
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
        Ok(res.bytes_stream())
    }

    pub async fn try_chat(&mut self, p: GeminiRequestBody) -> Result<Response, ClewdrError> {
        for i in 0..CLEWDR_CONFIG.load().max_retries + 1 {
            if i > 0 {
                info!("[RETRY] attempt: {}", i.to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();

            match state.send_chat(p).await {
                Ok(b) => {
                    let res = state.transform_response(b).await;
                    return Ok(res);
                }
                Err(e) => {
                    if let Some(key) = state.key.to_owned() {
                        error!("[{}] {}", key.key.ellipse().green(), e);
                    } else {
                        error!("{}", e);
                    }
                }
            }
        }
        error!("Max retries exceeded");
        Err(ClewdrError::TooManyRetries)
    }

    pub async fn try_from_cache(&self, p: &GeminiRequestBody) -> Option<axum::response::Response> {
        let key = p.get_hash(self.path.as_str());
        if let Some(stream) = CACHE.pop(key) {
            info!("[CACHE] found response for key: {}", key);
            return Some(self.transform_response(stream).await);
        }
        for id in 0..CLEWDR_CONFIG.load().cache_response {
            let mut state = self.to_owned();
            state.cache_key = Some((key, id));
            let p = p.to_owned();
            let cache_span = span!(Level::ERROR, "cache");
            spawn(async move { state.try_chat(p).instrument(cache_span).await });
        }
        None
    }

    pub async fn transform_response(
        &self,
        input: impl Stream<Item = Result<Bytes, rquest::Error>> + Send + 'static,
    ) -> axum::response::Response {
        // response is used for caching
        if let Some((key, id)) = self.cache_key {
            CACHE.push(input, key, id);
            // return whatever, not used
            return Body::empty().into_response();
        }
        // response is used for returning
        // not streaming
        // stream the response
        Body::from_stream(input).into_response()
    }
}
