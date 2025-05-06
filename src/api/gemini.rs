use axum::{body::Body, extract::State};
use tracing::info;

use crate::{
    config::GEMINI_ENDPOINT,
    error::ClewdrError,
    gemini_state::GeminiState,
    middleware::gemini::{GeminiContext, GeminiPreprocess},
};

pub async fn api_post_gemini(
    State(mut state): State<GeminiState>,
    GeminiPreprocess(body, ctx): GeminiPreprocess,
) -> Result<Body, ClewdrError> {
    state.update_from_ctx(&ctx);
    let GeminiContext {
        path,
        query,
        stream,
    } = ctx;
    info!("POST /v1beta/{}", path);
    let mut query_vec = query.to_vec();
    state.request_key().await?;
    let Some(key) = state.key.clone() else {
        return Err(ClewdrError::UnexpectedNone);
    };
    let key = key.key.to_string();
    query_vec.push(("key", key.as_str()));
    let res = state
        .client
        .post(format!("{}/v1beta/{}", GEMINI_ENDPOINT, path))
        .query(&query_vec)
        .json(&body)
        .send()
        .await?;

    // let res = res.error_for_status()?;
    if stream {
        let stream = res.bytes_stream();
        Ok(Body::from_stream(stream))
    } else {
        let bytes = res.bytes().await?;
        let body = Body::from(bytes);
        Ok(body)
    }
}
