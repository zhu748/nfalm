use futures::{Stream, stream};
use rquest::Response;
use serde_json::Value;
use std::{convert::Infallible, fmt::Display};
use tracing::{error, warn};

use crate::types::message::{
    ContentBlock, ContentBlockDelta, MessageDeltaContent, MessageStartContent, StreamEvent,
};

#[derive(thiserror::Error, Debug)]
pub enum ClewdrError {
    #[error("Invalid authorization")]
    InvalidAuth,
    #[error("Json error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("TOML Deserialize error: {0}")]
    TomlDeError(#[from] toml::de::Error),
    #[error("TOML Serialize error: {0}")]
    TomlSeError(#[from] toml::ser::Error),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Rquest error: {0}")]
    RquestError(#[from] rquest::Error),
    #[error("UTF8 error: {0}")]
    UTF8Error(#[from] std::string::FromUtf8Error),
    #[error("JavaScript error {0}")]
    JsError(JsError),
    #[error("Too many requests: {0}")]
    TooManyRequest(JsError, i64),
    #[error("Unexpected None")]
    UnexpectedNone,
    #[error("No valid key")]
    NoValidKey,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid model name: {0}")]
    InvalidModel(String),
    #[error("Config error: {0}")]
    PathNotFound(String),
    #[error("Invalid timestamp: {0}")]
    TimestampError(i64),
    #[error("Wait for cookie rotation")]
    CookieRotating,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct JsError {
    pub name: String,
    pub message: Option<Value>,
    pub status: Option<Value>,
    pub planned: Option<bool>,
    pub r#type: Option<Value>,
}

impl Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

pub async fn check_res_err(res: Response) -> Result<Response, ClewdrError> {
    let mut ret = JsError {
        name: "Error".to_string(),
        message: None,
        status: None,
        planned: None,
        r#type: None,
    };
    let status = res.status();
    if !status.is_success() {
        ret.message = Some(format!("Unexpected response code: {}", status).into());
        error!("Unexpected response code: {}", status);
    } else {
        return Ok(res);
    }
    let json = res.text().await.inspect_err(|e| {
        error!("Failed to get response: {}\n", e);
    })?;
    let json = serde_json::from_str::<Value>(&json).inspect_err(|e| {
        error!("Failed to parse response: {}\n", e);
    })?;
    let Some(err_api) = json.get("error") else {
        return Err(ClewdrError::JsError(ret));
    };
    ret.status = json.get("status").cloned();
    ret.planned = true.into();
    if !err_api["message"].is_null() {
        ret.message = err_api.get("message").cloned();
    }
    if !err_api["type"].is_null() {
        ret.r#type = err_api.get("type").cloned();
    }
    if status == 429 {
        if let Some(time) = err_api["message"]
            .as_str()
            .and_then(|m| serde_json::from_str::<Value>(m).ok())
            .and_then(|m| m["resetsAt"].as_i64())
        {
            let reset_time = chrono::DateTime::from_timestamp(time, 0)
                .ok_or(ClewdrError::TimestampError(time))?
                .to_utc();
            let now = chrono::Utc::now();
            let diff = reset_time - now;
            let hours = diff.num_hours();
            let message = format!("Rate limit exceeded, expires in {} hours", hours);
            warn!(message);
            ret.message = Some(message.into());
            return Err(ClewdrError::TooManyRequest(ret, time));
        }
    }
    Err(ClewdrError::JsError(ret))
}

pub fn error_stream(e: ClewdrError) -> impl Stream<Item = Result<axum::body::Bytes, Infallible>> {
    let msg_start_content = MessageStartContent::default();
    let msg_start_block = StreamEvent::MessageStart {
        message: msg_start_content,
    };
    let content_block = ContentBlock::Text {
        text: String::new(),
    };
    let content_block_start = StreamEvent::ContentBlockStart {
        index: 0,
        content_block,
    };
    let content_block_delta = ContentBlockDelta::TextDelta {
        text: format!("ClewdR Error: {e}"),
    };
    let content_block_delta = StreamEvent::ContentBlockDelta {
        index: 0,
        delta: content_block_delta,
    };
    let content_block_end = StreamEvent::ContentBlockStop { index: 0 };
    let message_delta = StreamEvent::MessageDelta {
        delta: MessageDeltaContent::default(),
        usage: None,
    };
    let message_stop = StreamEvent::MessageStop;
    let vec = vec![
        msg_start_block,
        content_block_start,
        content_block_delta,
        content_block_end,
        message_delta,
        message_stop,
    ];
    let vec = vec.into_iter().map(|e| {
        let e = serde_json::to_string(&e).unwrap();
        let e = format!("data: {e}\n\n");
        let bytes = axum::body::Bytes::from(e);
        Ok::<axum::body::Bytes, Infallible>(bytes)
    });
    stream::iter(vec)
}
