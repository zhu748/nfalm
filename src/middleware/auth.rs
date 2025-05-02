use axum::extract::FromRequestParts;
use axum_auth::AuthBearer;
use tracing::warn;

use crate::{config::CLEWDR_CONFIG, error::ClewdrError};

/// Extractor for the X-API-Key header used in Claude API compatibility
///
/// This struct extracts the API key from the "x-api-key" header and makes it
/// available to handlers that need to verify Claude-style authentication.
struct XApiKey(pub String);

impl<S> FromRequestParts<S> for XApiKey
where
    S: Sync,
{
    type Rejection = ClewdrError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or(ClewdrError::InvalidKey)?;
        Ok(Self(key.to_string()))
    }
}

/// Middleware guard that ensures requests have valid admin authentication
///
/// This extractor checks for a valid admin authorization token in the Bearer Auth header.
/// It can be used on routes that should only be accessible to administrators.
///
/// # Example
///
/// ```
/// async fn admin_only_handler(
///     _: RequireAdminAuth,
///     // other extractors...
/// ) -> impl IntoResponse {
///     // This handler only executes if admin authentication succeeds
///     // ...
/// }
/// ```
pub struct RequireAdminAuth;
impl<S> FromRequestParts<S> for RequireAdminAuth
where
    S: Sync,
{
    type Rejection = ClewdrError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let AuthBearer(key) = AuthBearer::from_request_parts(parts, &())
            .await
            .map_err(|_| ClewdrError::InvalidKey)?;
        if !CLEWDR_CONFIG.load().admin_auth(&key) {
            warn!("Invalid admin key");
            return Err(ClewdrError::InvalidKey);
        }
        Ok(Self)
    }
}

/// Middleware guard that ensures requests have valid OpenAI API authentication
///
/// This extractor validates the Bearer token against the configured OpenAI API keys.
/// It's used to protect OpenAI-compatible API endpoints.
///
/// # Example
///
/// ```
/// async fn openai_handler(
///     _: RequireOaiAuth,
///     // other extractors...
/// ) -> impl IntoResponse {
///     // This handler only executes if OpenAI authentication succeeds
///     // ...
/// }
/// ```
pub struct RequireOaiAuth;
impl<S> FromRequestParts<S> for RequireOaiAuth
where
    S: Sync,
{
    type Rejection = ClewdrError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let AuthBearer(key) = AuthBearer::from_request_parts(parts, &())
            .await
            .map_err(|_| ClewdrError::InvalidKey)?;
        if !CLEWDR_CONFIG.load().v1_auth(&key) {
            warn!("Invalid OpenAI key: {}", key);
            return Err(ClewdrError::InvalidKey);
        }
        Ok(Self)
    }
}

/// Middleware guard that ensures requests have valid Claude API authentication
///
/// This extractor validates the X-API-Key header against the configured API keys.
/// It's used to protect Claude-compatible API endpoints.
///
/// # Example
///
/// ```
/// async fn claude_handler(
///     _: RequireClaudeAuth,
///     // other extractors...
/// ) -> impl IntoResponse {
///     // This handler only executes if Claude authentication succeeds
///     // ...
/// }
/// ```
pub struct RequireClaudeAuth;
impl<S> FromRequestParts<S> for RequireClaudeAuth
where
    S: Sync,
{
    type Rejection = ClewdrError;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let XApiKey(key) = XApiKey::from_request_parts(parts, &()).await?;
        if !CLEWDR_CONFIG.load().v1_auth(&key) {
            warn!("Invalid Claude key: {}", key);
            return Err(ClewdrError::InvalidKey);
        }
        Ok(Self)
    }
}
