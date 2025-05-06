use crate::error::ClewdrError;
use axum::extract::{FromRequestParts, Query};
use serde::Deserialize;
use struct_iterable::Iterable;

#[derive(Debug, Clone, Deserialize, Iterable, Default)]
pub struct GeminiQuery {
    pub key: String,
    pub alt: Option<String>,
}

impl<S> FromRequestParts<S> for GeminiQuery
where
    S: Sync,
{
    type Rejection = ClewdrError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<GeminiQuery>::from_request_parts(parts, &()).await?;
        Ok(query)
    }
}

impl GeminiQuery {
    pub fn to_vec(&self) -> Vec<(&'static str, &str)> {
        let mut vec = Vec::new();
        for (k, vv) in self.iter() {
            if k == "key" {
                continue;
            }
            if let Some(v) = vv
                .downcast_ref::<String>()
                .or(vv.downcast_ref::<Option<String>>().and_then(|v| v.as_ref()))
            {
                vec.push((k, v.as_str()));
            }
        }
        vec
    }
}
