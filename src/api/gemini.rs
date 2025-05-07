use axum::{extract::State, response::Response};
use colored::Colorize;
use tracing::info;

use crate::{
    error::ClewdrError,
    gemini_state::GeminiState,
    middleware::gemini::{GeminiContext, GeminiPreprocess},
    utils::enabled,
};

pub async fn api_post_gemini(
    State(mut state): State<GeminiState>,
    GeminiPreprocess(body, ctx): GeminiPreprocess,
) -> Result<Response, ClewdrError> {
    state.update_from_ctx(&ctx);
    let GeminiContext { model, stream, vertex,.. } = ctx;
    info!(
        "[REQ] stream: {}, vertex: {}, model: {}",
        enabled(stream),
        enabled(vertex),
        model.green(),
    );
    let res = state.try_chat(body).await?;
    return Ok(res);
}
