use std::{convert::Infallible, sync::LazyLock};

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use bytes::{BufMut, Bytes, BytesMut};
use colored::Colorize;
use futures::{Stream, StreamExt, pin_mut, stream};
use rquest::{Client, ClientBuilder, header::AUTHORIZATION};
use serde::Serialize;
use serde_json::{Value, json};
use strum::Display;
use tokio::spawn;
use tracing::{Instrument, Level, error, info, span, warn};

use crate::{
    config::{CLEWDR_CONFIG, GEMINI_ENDPOINT, KeyStatus},
    error::{CheckGeminiErr, ClewdrError},
    gemini_body::GeminiArgs,
    middleware::gemini::GeminiContext,
    services::{
        cache::{CACHE, GetHashKey},
        key_manager::KeyEventSender,
    },
};

#[derive(Clone, Display, PartialEq, Eq)]
pub enum GeminiApiFormat {
    Gemini,
    OpenAI,
}

pub static SAFETY_SETTINGS: LazyLock<Value> = LazyLock::new(|| {
    json!([
      { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "OFF" },
      { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "OFF" },
      { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "OFF" },
      { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "OFF" },
      {
        "category": "HARM_CATEGORY_CIVIC_INTEGRITY",
        "threshold": "BLOCK_NONE"
      }
    ])
});

static DUMMY_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Clone)]
pub struct GeminiState {
    pub model: String,
    pub vertex: bool,
    pub path: String,
    pub key: Option<KeyStatus>,
    pub stream: bool,
    pub query: GeminiArgs,
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
            query: GeminiArgs::default(),
            stream: false,
            key: None,
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
        self.api_format = ctx.api_format.to_owned();
    }

    async fn vertex_response(
        &mut self,
        p: impl Sized + Serialize,
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
        let res = res.check_gemini().await?;
        let res = res.json::<serde_json::Value>().await?;
        let access_token = res["access_token"]
            .as_str()
            .ok_or(ClewdrError::UnexpectedNone)?;
        let bearer = format!("Bearer {}", access_token);
        let res = match self.api_format {
            GeminiApiFormat::Gemini => {
                let endpoint = format!(
                    "https://aiplatform.googleapis.com/v1/projects/{}/locations/global/publishers/google/models/{}:{method}",
                    CLEWDR_CONFIG
                        .load()
                        .vertex
                        .project_id
                        .as_deref()
                        .unwrap_or_default(),
                    self.model
                );
                let query_vec = self.query.to_vec();
                self
                    .client
                    .post(endpoint)
                    .query(&query_vec)
                    .header(AUTHORIZATION, bearer)
                    .json(&p)
                    .send()
                    .await?
            }
            GeminiApiFormat::OpenAI => {
                self.client
                    .post(format!(
                        "https://aiplatform.googleapis.com/v1beta1/projects/{}/locations/global/endpoints/openapi/chat/completions",
                        CLEWDR_CONFIG
                            .load()
                            .vertex
                            .project_id
                            .as_deref()
                            .unwrap_or_default(),
                    ))
                    .header(AUTHORIZATION, bearer)
                    .json(&p)
                    .send()
                    .await?
            }
        };
        let res = res.check_gemini().await?;
        Ok(res)
    }

    pub async fn send_chat(
        &mut self,
        p: impl Sized + Serialize,
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
        info!("[KEY] {}", key.key.ellipse().green());
        let key = key.key.to_string();
        let res = match self.api_format {
            GeminiApiFormat::Gemini => {
                let mut query_vec = self.query.to_vec();
                query_vec.push(("key", key.as_str()));
                self.client
                    .post(format!("{}/v1beta/{}", GEMINI_ENDPOINT, self.path))
                    .query(&query_vec)
                    .json(&p)
                    .send()
                    .await?
            }
            GeminiApiFormat::OpenAI => {
                self.client
                    .post(format!(
                        "{}/v1beta/openai/chat/completions",
                        GEMINI_ENDPOINT,
                    ))
                    .header(AUTHORIZATION, format!("Bearer {}", key))
                    .json(&p)
                    .send()
                    .await?
            }
        };
        let res = res.check_gemini().await?;
        Ok(res.bytes_stream())
    }

    pub async fn try_chat(
        &mut self,
        p: impl Serialize + GetHashKey + Clone,
    ) -> Result<Response, ClewdrError> {
        for i in 0..CLEWDR_CONFIG.load().max_retries + 1 {
            if i > 0 {
                info!("[RETRY] attempt: {}", i.to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();

            match state.send_chat(p).await {
                Ok(b) => {
                    if !state.stream {
                        let Ok(b) = state.check_empty_choices(b).await else {
                            warn!("Empty choices");
                            continue;
                        };
                        let res = transform_response(self.cache_key, b).await;
                        return Ok(res);
                    } else {
                        let res = transform_response(self.cache_key, b).await;
                        return Ok(res);
                    };
                }
                Err(e) => {
                    if let Some(key) = state.key.to_owned() {
                        error!("[{}] {}", key.key.ellipse().green(), e);
                    } else {
                        error!("{}", e);
                    }
                    match e {
                        ClewdrError::GeminiHttpError(_, _) => {
                            continue;
                        }
                        e => return Err(e),
                    }
                }
            }
        }
        error!("Max retries exceeded");
        Err(ClewdrError::TooManyRetries)
    }

    pub async fn try_from_cache(
        &self,
        p: &(impl Serialize + GetHashKey + Clone + Send + 'static),
    ) -> Option<axum::response::Response> {
        let key = p.get_hash();
        if let Some(stream) = CACHE.pop(key) {
            info!("[CACHE] found response for key: {}", key);
            return Some(Body::from_stream(stream).into_response());
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

    async fn check_empty_choices(
        &self,
        input: impl Stream<Item = Result<Bytes, impl std::error::Error + Send + Sync + 'static>>
        + Send
        + 'static,
    ) -> Result<
        impl Stream<Item = Result<Bytes, impl std::error::Error + Send + Sync + 'static>>
        + Send
        + 'static,
        ClewdrError,
    > {
        let mut bytes = BytesMut::new();
        pin_mut!(input);
        while let Some(item) = input.next().await {
            match item {
                Ok(b) => bytes.put(b),
                Err(_) => continue,
            }
        }
        let bytes = bytes.freeze();
        if let Ok(json) = serde_json::from_slice::<Value>(&bytes) {
            match self.api_format {
                GeminiApiFormat::Gemini => {
                    if json["contents"].as_array().map_or(false, |v| v.is_empty()) {
                        return Err(ClewdrError::EmptyChoices);
                    }
                }
                GeminiApiFormat::OpenAI => {
                    if json["choices"].as_array().map_or(false, |v| v.is_empty()) {
                        return Err(ClewdrError::EmptyChoices);
                    }
                }
            }
        }
        Ok(stream::once(async { Ok::<_, Infallible>(bytes) }))
    }
}

async fn transform_response<E>(
    cache_key: Option<(u64, usize)>,
    input: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
) -> axum::response::Response
where
    E: std::error::Error + Send + Sync + 'static,
{
    // response is used for caching
    if let Some((key, id)) = cache_key {
        CACHE.push(input, key, id);
        // return whatever, not used
        return Body::empty().into_response();
    }
    // response is used for returning
    // not streaming
    // stream the response
    Body::from_stream(input).into_response()
}
