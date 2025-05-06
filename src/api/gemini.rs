use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::State,
    response::{IntoResponse, Response},
};
use colored::Colorize;
use futures::Stream;
use tokio::time::{self, Duration, Instant};
use tokio_stream::wrappers::IntervalStream;
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
    if stream {
        return Ok(res);
    }
    let byte_stream = res.into_body().into_data_stream();
    let keep_alive_stream = KeepAliveStream::new(
        byte_stream,
        Duration::from_secs(15),
        Duration::from_secs(60 * 3),
    );
    Ok(Body::from_stream(keep_alive_stream).into_response())
}

// A stream that periodically sends keep-alive chunks
struct KeepAliveStream<S> {
    inner: S,
    interval: IntervalStream,
    last_data: Instant,
    timeout: Duration,
}

impl<S> KeepAliveStream<S>
where
    S: Stream<Item = Result<axum::body::Bytes, axum::Error>> + Unpin,
{
    fn new(stream: S, keep_alive_interval: Duration, timeout: Duration) -> Self {
        Self {
            inner: stream,
            interval: IntervalStream::new(time::interval(keep_alive_interval)),
            last_data: Instant::now(),
            timeout,
        }
    }
}

impl<S> Stream for KeepAliveStream<S>
where
    S: Stream<Item = Result<axum::body::Bytes, axum::Error>> + Unpin,
{
    type Item = Result<axum::body::Bytes, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First poll the inner stream
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(data)) => {
                // Update the last data timestamp
                self.last_data = Instant::now();
                return Poll::Ready(Some(data));
            }
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => {
                // Check if we've exceeded the timeout
                if self.last_data.elapsed() > self.timeout {
                    return Poll::Ready(None);
                }
            }
        }

        // If inner stream is pending, check if we need to send a keep-alive
        match Pin::new(&mut self.interval).poll_next(cx) {
            Poll::Ready(Some(_)) => {
                // Send a keep-alive comment
                return Poll::Ready(Some(Ok(axum::body::Bytes::from("<!-- keep-alive -->"))));
            }
            _ => {}
        }

        Poll::Pending
    }
}
