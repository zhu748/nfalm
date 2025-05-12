use axum::{
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use colored::Colorize;
use futures::{FutureExt, StreamExt};
use serde::Serialize;
use tracing::info;

use crate::{
    error::ClewdrError,
    gemini_state::{GeminiApiFormat, GeminiState},
    middleware::gemini::{GeminiContext, GeminiOaiPreprocess, GeminiPreprocess},
    services::cache::GetHashKey,
    utils::enabled,
};

// Common handler function to process both Gemini and OpenAI format requests
async fn handle_gemini_request<T: Serialize + GetHashKey + Clone + Send + 'static>(
    mut state: GeminiState,
    body: T,
    ctx: GeminiContext,
) -> Result<Response, ClewdrError> {
    state.update_from_ctx(&ctx);
    let GeminiContext {
        model,
        stream,
        vertex,
        ..
    } = ctx;
    info!(
        "[REQ] stream: {}, vertex: {}, format: {}, model: {}",
        enabled(stream),
        enabled(vertex),
        if ctx.api_format == GeminiApiFormat::Gemini {
            ctx.api_format.to_string().green()
        } else {
            ctx.api_format.to_string().yellow()
        },
        model.green(),
    );

    // For non-streaming requests, we need to handle keep-alive differently
    if !stream {
        let stream = async move {
            state
                .try_chat(body)
                .await
                .map(|res| res.into_body().into_data_stream())
                .unwrap_or_else(|e| e.into_response().into_body().into_data_stream())
        }
        .into_stream()
        .flatten();
        return Ok(Body::from_stream(stream).into_response());
    }

    // For streaming requests, proceed as before
    let res = state.try_chat(body).await?;
    Ok(res)
}

pub async fn api_post_gemini(
    State(state): State<GeminiState>,
    GeminiPreprocess(body, ctx): GeminiPreprocess,
) -> Result<Response, ClewdrError> {
    handle_gemini_request(state, body, ctx).await
}

pub async fn api_post_gemini_oai(
    State(state): State<GeminiState>,
    GeminiOaiPreprocess(body, ctx): GeminiOaiPreprocess,
) -> Result<Response, ClewdrError> {
    handle_gemini_request(state, body, ctx).await
}
