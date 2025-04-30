use axum::extract::FromRequestParts;
use axum_auth::AuthBearer;
use tracing::warn;

use crate::{config::CLEWDR_CONFIG, error::ClewdrError};

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
        let XApiKey(key) = XApiKey::from_request_parts(parts, &())
            .await
            .map_err(|_| ClewdrError::InvalidKey)?;
        if !CLEWDR_CONFIG.load().v1_auth(&key) {
            warn!("Invalid Claude key: {}", key);
            return Err(ClewdrError::InvalidKey);
        }
        Ok(Self)
    }
}
