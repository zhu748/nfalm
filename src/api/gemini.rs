use axum::{Json, body::Body, extract::Path};
use serde_json::Value;
use tracing::info;

use crate::{config::GEMINI_ENDPOINT, error::ClewdrError, gemini_body::GeminiQuery};
const TEST_KEY: &str = "";

pub async fn api_post_gemini(
    Path(path): Path<String>,
    query: GeminiQuery,
    Json(body): Json<Value>,
) -> Result<Body, ClewdrError> {
    info!("POST /v1beta/{}", path);
    let is_stream = path.contains("stream");
    let path = path.trim_start_matches('/').to_string();
    let client = rquest::Client::new();
    let mut query_vec = query.to_vec();
    query_vec.push(("key", TEST_KEY));
    let res = client
        .post(format!("{}/v1beta/{}", GEMINI_ENDPOINT, path))
        .query(&query_vec)
        .json(&body)
        .send()
        .await?;

    // let res = res.error_for_status()?;
    if is_stream {
        let stream = res.bytes_stream();
        Ok(Body::from_stream(stream))
    } else {
        let bytes = res.bytes().await?;
        let body = Body::from(bytes);
        Ok(body)
    }
}
