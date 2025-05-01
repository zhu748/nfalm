use bytes::Bytes;
use futures::{Stream, StreamExt, pin_mut, stream};
use moka::sync::Cache;
use serde_json::Value;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, LazyLock, Mutex},
};
use tokio::spawn;
use tracing::{debug, info};

use crate::{
    config::CLEWDR_CONFIG,
    types::message::{CreateMessageParams, Message},
};

pub static CACHE: LazyLock<ClewdrCache> = LazyLock::new(|| ClewdrCache::new());

pub struct ClewdrCache {
    moka: Cache<u64, Arc<Mutex<CachedResponse>>>,
}

impl ClewdrCache {
    pub fn new() -> Self {
        Self {
            moka: Cache::builder()
                .max_capacity(100)
                .time_to_live(std::time::Duration::from_secs(60 * 10))
                .build(),
        }
    }

    pub fn push(
        &'static self,
        stream: impl Stream<Item = Result<Bytes, rquest::Error>> + Send + 'static,
        key: u64,
        id: usize,
    ) {
        spawn(async move {
            let vec = stream_to_vec(stream).await;
            let value = self.moka.get_with(key, Default::default);
            {
                let Ok(mut value) = value.lock() else {
                    debug!("Failed to lock cache for key: {}", key);
                    return;
                };
                if value.len() >= CLEWDR_CONFIG.load().max_cache {
                    debug!("Cache is full, skipping cache for key: {}", key);
                    return;
                }
                value.push(vec);
            }
            info!("[CACHE {}] cached response for key: {}", id, key);
        });
    }

    pub fn pop(
        &self,
        key: u64,
    ) -> Option<impl Stream<Item = Result<Bytes, rquest::Error>> + Send + 'static> {
        let value = self.moka.get(&key)?;
        let (vec, now_empty) = {
            let mut value = value
                .lock()
                .inspect_err(|e| debug!("Failed to lock cache for key: {}: {}", key, e))
                .ok()?;
            (value.pop(), value.is_empty())
        };
        if now_empty {
            debug!("Cache is empty for key: {}", key);
            // remove the cache entry
            self.moka.invalidate(&key);
        }
        let vec = vec?;
        info!("Cache hit for key: {}", key);
        Some(vec_to_stream(vec))
    }
}

#[derive(Clone, Debug)]
struct CachedResponse {
    bodies: Vec<Vec<Bytes>>,
}

impl CachedResponse {
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    pub fn pop(&mut self) -> Option<Vec<Bytes>> {
        self.bodies.pop()
    }

    pub fn push(&mut self, vec: Vec<Bytes>) {
        self.bodies.push(vec);
    }
}
impl Default for CachedResponse {
    fn default() -> Self {
        Self { bodies: Vec::new() }
    }
}

async fn stream_to_vec(stream: impl Stream<Item = Result<Bytes, rquest::Error>>) -> Vec<Bytes> {
    pin_mut!(stream);
    stream
        .filter_map(async |item| item.ok())
        .collect::<Vec<_>>()
        .await
}

fn vec_to_stream(bytes: Vec<Bytes>) -> impl Stream<Item = Result<Bytes, rquest::Error>> {
    stream::iter(bytes.into_iter().map(Ok))
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RequestKeys {
    /// Maximum number of tokens to generate
    pub max_tokens: u32,
    /// Input messages for the conversation
    pub messages: Vec<Message>,
    /// Model to use
    pub model: String,
    /// System prompt
    pub system: Option<Value>,
    /// Custom stop sequences
    pub stop_sequences: Option<Vec<String>>,
    /// Thinking mode configuration
    pub thinking: bool,
    /// Top-k sampling
    pub top_k: Option<u32>,
}

impl RequestKeys {
    fn get_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl CreateMessageParams {
    pub fn get_hash(&self) -> u64 {
        let keys = RequestKeys::from(self);
        keys.get_hash()
    }
}

impl From<&CreateMessageParams> for RequestKeys {
    // TODO: handle useless parameters
    fn from(params: &CreateMessageParams) -> Self {
        RequestKeys {
            max_tokens: params.max_tokens,
            messages: params.messages.to_owned(),
            model: params.model.to_owned(),
            system: params.system.to_owned(),
            stop_sequences: params.stop_sequences.to_owned(),
            thinking: params.thinking.is_some(),
            top_k: params.top_k,
        }
    }
}
