use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, LazyLock, Mutex},
};

use bytes::Bytes;
use futures::{Stream, StreamExt, pin_mut, stream};
use moka::future::Cache;
use serde_json::Value;

use crate::types::message::{CreateMessageParams, Message};

pub static CACHE: LazyLock<Cache<u64, Arc<Mutex<CachedResponse>>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(100)
        .time_to_live(std::time::Duration::from_secs(60 * 10))
        .build()
});

#[derive(Clone, Debug)]
pub struct CachedResponse {
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

pub async fn stream_to_vec(stream: impl Stream<Item = Result<Bytes, rquest::Error>>) -> Vec<Bytes> {
    pin_mut!(stream);
    stream
        .filter_map(async |item| item.ok())
        .collect::<Vec<_>>()
        .await
}

pub fn vec_to_stream(bytes: Vec<Bytes>) -> impl Stream<Item = Result<Bytes, rquest::Error>> {
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
