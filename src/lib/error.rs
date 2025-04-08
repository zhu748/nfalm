use colored::Colorize;
use futures::{Stream, stream};
use rquest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{convert::Infallible, fmt::Display};
use tokio::sync::{mpsc::error::SendError, oneshot};
use tracing::{debug, error};

use crate::{
    config::{CookieStatus, Reason},
    messages::non_stream_message,
    types::message::{
        ContentBlock, ContentBlockDelta, Message, MessageDeltaContent, MessageStartContent,
        StreamEvent,
    },
};

#[derive(thiserror::Error, Debug)]
pub enum ClewdrError {
    #[error("Zip error: {0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("Asset Error: {0}")]
    AssetError(String),
    #[error("Invalid version: {0}")]
    InvalidVersion(String),
    #[error("Failed to parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Failed to parse URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Tokio oneshot recv error: {0}")]
    CookieDispatchError(#[from] oneshot::error::RecvError),
    #[error("Tokio mpsc send error: {0}")]
    CookieReqError(#[from] SendError<oneshot::Sender<Result<CookieStatus, ClewdrError>>>),
    #[error("No cookie available")]
    NoCookieAvailable,
    #[error("Invalid Cookie, reason: {0}")]
    InvalidCookie(Reason),
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
    #[error("Http error: code: {}, body: {}", .0.to_string().red(), .1)]
    OtherHttpError(StatusCode, HttpError),
    #[error("Unexpected None")]
    UnexpectedNone,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Config error: {0}")]
    PathNotFound(String),
    #[error("Invalid timestamp: {0}")]
    TimestampError(i64),
}

/// HTTP error response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpError {
    pub error: InnerHttpError,
    r#type: String,
}

impl Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

/// Inner HTTP error response
#[derive(Debug, Serialize, Clone)]
pub struct InnerHttpError {
    pub message: Value,
    pub r#type: String,
}

/// Raw Inner HTTP error response
#[derive(Debug, Serialize, Deserialize)]
struct InnerHttpErrorRaw {
    pub message: String,
    pub r#type: String,
}

impl<'de> Deserialize<'de> for InnerHttpError {
    /// when message is a json string, try parse it as a object
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = InnerHttpErrorRaw::deserialize(deserializer)?;
        if let Ok(message) = serde_json::from_str::<Value>(&raw.message) {
            return Ok(InnerHttpError {
                message,
                r#type: raw.r#type,
            });
        }
        Ok(InnerHttpError {
            message: json!(raw.message),
            r#type: raw.r#type,
        })
    }
}

impl Display for InnerHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

/// Check response from Claude Web
pub async fn check_res_err(res: Response) -> Result<Response, ClewdrError> {
    let status = res.status();
    if status.is_success() {
        return Ok(res);
    }
    debug!("Error response status: {}", status);
    let Ok(err) = res.json::<HttpError>().await else {
        let inner = InnerHttpError {
            message: json!("Failed to parse error response"),
            r#type: "error".to_string(),
        };
        let http_error = HttpError {
            error: inner,
            r#type: "error".to_string(),
        };
        return Err(ClewdrError::OtherHttpError(status, http_error));
    };
    let err_clone = err.clone();
    let inner_error = err.error;
    // check if the error is a rate limit error
    if status == 429 {
        // get the reset time from the error message
        if let Some(time) = inner_error.message["resetsAt"].as_i64() {
            let reset_time = chrono::DateTime::from_timestamp(time, 0)
                .ok_or(ClewdrError::TimestampError(time))?
                .to_utc();
            let now = chrono::Utc::now();
            let diff = reset_time - now;
            let hours = diff.num_hours();
            error!("Rate limit exceeded, expires in {} hours", hours);
            return Err(ClewdrError::InvalidCookie(Reason::TooManyRequest(time)));
        }
    }
    Err(ClewdrError::OtherHttpError(status, err_clone))
}
impl ClewdrError {
    /// Convert a ClewdrError to a Stream of Claude API events
    pub fn error_stream(
        &self,
    ) -> impl Stream<Item = Result<axum::body::Bytes, Infallible>> + use<> {
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
            text: format!("ClewdR Error: {self}"),
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
            // SSE format
            let e = format!("data: {e}\n\n");
            let bytes = axum::body::Bytes::from(e);
            Ok::<axum::body::Bytes, Infallible>(bytes)
        });
        stream::iter(vec)
    }

    pub fn error_body(&self) -> Message {
        non_stream_message(self.to_string())
    }
}
