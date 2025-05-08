use axum::{
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use colored::Colorize;
use futures::{TryFutureExt, stream};
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
    let GeminiContext {
        model,
        stream,
        vertex,
        ..
    } = ctx;
    info!(
        "[REQ] stream: {}, vertex: {}, model: {}",
        enabled(stream),
        enabled(vertex),
        model.green(),
    );

    // For non-streaming requests, we need to handle keep-alive differently
    if !stream {
        let stream = stream::once(async move {
            state
                .try_chat(body)
                .map_err(|e| axum::Error::new(e.to_string()))
                .and_then(|res| axum::body::to_bytes(res.into_body(), usize::MAX))
                .await
        });
        return Ok(Body::from_stream(stream).into_response());
    }

    // For streaming requests, proceed as before
    let res = state.try_chat(body).await?;
    Ok(res)
}
