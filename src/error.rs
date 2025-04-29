use axum::{Json, response::IntoResponse};
use colored::Colorize;
use rquest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Display;
use strum::IntoStaticStr;
use tokio::sync::oneshot;
use tracing::{debug, error};

use crate::{config::Reason, services::cookie_manager::CookieEvent};

#[derive(thiserror::Error, Debug, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ClewdrError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Pad text too short")]
    PadtxtTooShort,
    #[error(transparent)]
    FigmentError(#[from] figment::Error),
    #[error(transparent)]
    MpscSendError(#[from] tokio::sync::mpsc::error::SendError<CookieEvent>),
    #[error("Retries exceeded")]
    TooManyRetries,
    #[error(transparent)]
    EventSourceError(#[from] eventsource_stream::EventStreamError<rquest::Error>),
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
    #[error("Invalid Cookie: {0}")]
    InvalidCookie(Reason),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSeError(#[from] toml::ser::Error),
    #[error(transparent)]
    RegexError(#[from] regex::Error),
    #[error(transparent)]
    RquestError(#[from] rquest::Error),
    #[error(transparent)]
    UTF8Error(#[from] std::string::FromUtf8Error),
    #[error("Http error: code: {}, body: {}", .0.to_string().red(), .1.to_string())]
    OtherHttpError(StatusCode, JsError),
    #[error("Unexpected None")]
    UnexpectedNone,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Config error: {0}")]
    PathNotFound(String),
    #[error("Invalid timestamp: {0}")]
    TimestampError(i64),
    #[error("Key/Password Incorrect")]
    IncorrectKey,
}

impl IntoResponse for ClewdrError {
    fn into_response(mut self) -> axum::response::Response {
        let (status, msg) = match self {
            ClewdrError::OtherHttpError(status, ref mut inner) => (status, inner.message.take()),
            ClewdrError::PadtxtTooShort => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::TooManyRetries => (StatusCode::TOO_MANY_REQUESTS, json!(self.to_string())),
            ClewdrError::InvalidCookie(_) => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::PathNotFound(_) => (StatusCode::NOT_FOUND, json!(self.to_string())),
            ClewdrError::IncorrectKey => (StatusCode::UNAUTHORIZED, json!(self.to_string())),
            ClewdrError::BadRequest(_) => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, json!(self.to_string())),
        };
        let r#type: &'static str = self.into();
        let body = JsError {
            message: msg,
            r#type: r#type.to_owned(),
            code: Some(status.as_u16()),
        };
        (status, Json(body)).into_response()
    }
}

/// HTTP error response
#[derive(Debug, Deserialize, Clone)]
pub struct ApiError {
    pub error: JsError,
}

/// Inner HTTP error response
#[derive(Debug, Serialize, Clone)]
pub struct JsError {
    pub message: Value,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
}

/// Raw Inner HTTP error response
#[derive(Debug, Serialize, Deserialize)]
struct JsErrorRaw {
    pub message: String,
    pub r#type: String,
}

impl<'de> Deserialize<'de> for JsError {
    /// when message is a json string, try parse it as a object
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = JsErrorRaw::deserialize(deserializer)?;
        if let Ok(message) = serde_json::from_str::<Value>(&raw.message) {
            return Ok(JsError {
                message,
                r#type: raw.r#type,
                code: None,
            });
        }
        Ok(JsError {
            message: json!(raw.message),
            r#type: raw.r#type,
            code: None,
        })
    }
}

impl Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string_pretty(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

/// Checks response from Claude Web API for errors
/// Validates HTTP status codes and parses error messages from responses
///
/// # Arguments
/// * `res` - The HTTP response to check
///
/// # Returns
/// * `Ok(Response)` if the request was successful
/// * `Err(ClewdrError)` if the request failed, with details about the failure
pub async fn check_res_err(res: Response) -> Result<Response, ClewdrError> {
    let status = res.status();
    if status.is_success() {
        return Ok(res);
    }
    debug!("Error response status: {}", status);
    if status == 302 {
        // blocked by cloudflare
        let error = JsError {
            message: json!("Blocked, check your IP address"),
            r#type: "error".to_string(),
            code: Some(status.as_u16()),
        };
        return Err(ClewdrError::OtherHttpError(status, error));
    }
    let text = match res.text().await {
        Ok(text) => text,
        Err(err) => {
            let error = JsError {
                message: json!(err.to_string()),
                r#type: "error_get_error_body".to_string(),
                code: Some(status.as_u16()),
            };
            return Err(ClewdrError::OtherHttpError(status, error));
        }
    };
    let Ok(err) = serde_json::from_str::<ApiError>(&text) else {
        let error = JsError {
            message: format!("Unknown error: {}", text).into(),
            r#type: "error_parse_error".to_string(),
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
