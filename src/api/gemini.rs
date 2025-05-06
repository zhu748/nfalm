use axum::{extract::State, response::Response};
use colored::Colorize;
use tracing::info;

use crate::{
    error::ClewdrError,
    gemini_state::GeminiState,
    middleware::gemini::{GeminiContext, GeminiPreprocess},
};

pub async fn api_post_gemini(
    State(mut state): State<GeminiState>,
    GeminiPreprocess(body, ctx): GeminiPreprocess,
) -> Result<Response, ClewdrError> {
    state.update_from_ctx(&ctx);
    let GeminiContext { path, stream, .. } = ctx;
    info!(
        "[REQ] stream: {}, path: {}",
        stream.to_string().green(),
        path.green(),
    );
    let res = state.try_chat(body).await?;
    return Ok(res);
}
