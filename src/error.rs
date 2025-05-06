use axum::{Json, extract::rejection::JsonRejection, response::IntoResponse};
use colored::Colorize;
use rquest::{Response, StatusCode, header::InvalidHeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Display;
use strum::IntoStaticStr;
use tokio::sync::oneshot;
use tracing::{debug, error};

use crate::{
    config::Reason,
    services::{cookie_manager::CookieEvent, key_manager::KeyEvent},
    types::claude_message::Message,
};

#[derive(thiserror::Error, Debug, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ClewdrError {
    #[error(transparent)]
    QueryRejection(#[from] axum::extract::rejection::QueryRejection),
    #[error("Cache found")]
    CacheFound(axum::response::Response),
    #[error("Test Message")]
    TestMessage,
    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Pad text too short")]
    PadtxtTooShort,
    #[error(transparent)]
    FigmentError(#[from] figment::Error),
    #[error(transparent)]
    KeySendError(#[from] tokio::sync::mpsc::error::SendError<KeyEvent>),
    #[error(transparent)]
    CookieSendError(#[from] tokio::sync::mpsc::error::SendError<CookieEvent>),
    #[error("Retries exceeded")]
    TooManyRetries,
    #[error(transparent)]
    EventSourceError(#[from] eventsource_stream::EventStreamError<axum::Error>),
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    #[error("Asset Error: {0}")]
    AssetError(String),
    #[error("Invalid version: {0}")]
    InvalidVersion(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    CookieDispatchError(#[from] oneshot::error::RecvError),
    #[error("No cookie available")]
    NoCookieAvailable,
    #[error("No key available")]
    NoKeyAvailable,
    #[error("Invalid Cookie: {0}")]
    InvalidCookie(Reason),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSeError(#[from] toml::ser::Error),
    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),
    #[error(transparent)]
    RquestError(#[from] rquest::Error),
    #[error(transparent)]
    UTF8Error(#[from] std::string::FromUtf8Error),
    #[error("Http error: code: {}, body: {}", .0.to_string().red(), .1.to_string())]
    OtherHttpError(StatusCode, JsErrorBody),
    #[error("Unexpected None")]
    UnexpectedNone,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Config error: {0}")]
    PathNotFound(String),
    #[error("Invalid timestamp: {0}")]
    TimestampError(i64),
    #[error("Key/Password Invalid")]
    InvalidKey,
}

impl IntoResponse for ClewdrError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            ClewdrError::QueryRejection(ref r) => (r.status(), json!(r.body_text())),
            ClewdrError::OtherHttpError(status, inner) => {
                return (status, Json(JsError { error: inner })).into_response();
            }
            ClewdrError::CacheFound(res) => return (StatusCode::OK, res).into_response(),
            ClewdrError::TestMessage => {
                return (
                    StatusCode::OK,
                    Json(Message::from(
                        "Claude Reverse Proxy is working, please send a real message.",
                    )),
                )
                    .into_response();
            }
            ClewdrError::JsonRejection(ref r) => (r.status(), json!(r.body_text())),
            ClewdrError::PadtxtTooShort => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::TooManyRetries => (StatusCode::TOO_MANY_REQUESTS, json!(self.to_string())),
            ClewdrError::InvalidCookie(_) => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::PathNotFound(_) => (StatusCode::NOT_FOUND, json!(self.to_string())),
            ClewdrError::InvalidKey => (StatusCode::UNAUTHORIZED, json!(self.to_string())),
            ClewdrError::BadRequest(_) => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::InvalidHeaderValue(_) => {
                (StatusCode::BAD_REQUEST, json!(self.to_string()))
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, json!(self.to_string())),
        };
        let err = JsError {
            error: JsErrorBody {
                message: msg,
                r#type: <&'static str>::from(self).into(),
                code: Some(status.as_u16()),
            },
        };
        (status, Json(err)).into_response()
    }
}

/// HTTP error response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsError {
    pub error: JsErrorBody,
}

/// Inner HTTP error response
#[derive(Debug, Serialize, Clone)]
pub struct JsErrorBody {
    pub message: Value,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
}

/// Raw Inner HTTP error response
#[derive(Debug, Deserialize)]
struct RawBody {
    pub message: String,
    pub r#type: String,
}

impl<'de> Deserialize<'de> for JsErrorBody {
    /// when message is a json string, try parse it as a object
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawBody::deserialize(deserializer)?;
        if let Ok(message) = serde_json::from_str::<Value>(&raw.message) {
            return Ok(JsErrorBody {
                message,
                r#type: raw.r#type,
                code: None,
            });
        }
        Ok(JsErrorBody {
            message: json!(raw.message),
            r#type: raw.r#type,
            code: None,
        })
    }
}

impl Display for JsErrorBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string_pretty(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

pub trait CheckResErr
where
    Self: Sized,
{
    fn check(self) -> impl Future<Output = Result<Self, ClewdrError>>;
}

impl CheckResErr for Response {
    /// Checks response from Claude Web API for errors
    /// Validates HTTP status codes and parses error messages from responses
    ///
    /// # Arguments
    /// * `res` - The HTTP response to check
    ///
    /// # Returns
    /// * `Ok(Response)` if the request was successful
    /// * `Err(ClewdrError)` if the request failed, with details about the failure
    async fn check(self) -> Result<Self, ClewdrError> {
        let status = self.status();
        if status.is_success() {
            return Ok(self);
        }
        debug!("Error response status: {}", status);
        if status == 302 {
            // blocked by cloudflare
            let error = JsErrorBody {
                message: json!("Blocked, check your IP address"),
                r#type: "error".to_string(),
                code: Some(status.as_u16()),
            };
            return Err(ClewdrError::OtherHttpError(status, error));
        }
        let text = match self.text().await {
            Ok(text) => text,
            Err(err) => {
                let error = JsErrorBody {
                    message: json!(err.to_string()),
                    r#type: "error_get_error_body".to_string(),
                    code: Some(status.as_u16()),
                };
                return Err(ClewdrError::OtherHttpError(status, error));
            }
        };
        let Ok(err) = serde_json::from_str::<JsError>(&text) else {
            let error = JsErrorBody {
                message: format!("Unknown error: {}", text).into(),
                r#type: "error_parse_error_body".to_string(),
                code: Some(status.as_u16()),
            };
            return Err(ClewdrError::OtherHttpError(status, error));
        };
        if status == 400 && err.error.message == json!("This organization has been disabled.") {
            // account disabled
            return Err(ClewdrError::InvalidCookie(Reason::Disabled));
        }
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
        Err(ClewdrError::OtherHttpError(status, inner_error))
    }
}
