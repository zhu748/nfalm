use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{BoxError, body::Bytes};
use futures::{Stream, TryStream, TryStreamExt};
use tokio::spawn;
use tracing::warn;

use crate::state::AppState;

pub struct ClewdrStream<S>
where
    S: TryStream + Send + 'static,
    S::Ok: Into<Bytes>,
    S::Error: Into<BoxError>,
{
    stream: S,
    state: AppState,
}

impl<S> ClewdrStream<S>
where
    S: TryStream + Send + 'static,
    S::Ok: Into<Bytes>,
    S::Error: Into<BoxError>,
{
    pub fn new(stream: S, state: AppState) -> Self {
        Self { stream, state }
    }
}

impl<S> Stream for ClewdrStream<S>
where
    S: TryStream + Send + 'static + Unpin,
    S::Ok: Into<Bytes>,
    S::Error: Into<BoxError>,
{
    type Item = Result<Bytes, BoxError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.stream.try_poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes.into()))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                let state = this.state.clone();
                spawn(async move {
                    if let Err(e) = state.delete_chat().await {
                        warn!("Error deleting chat: {:?}", e);
                    }
                });
                Poll::Ready(None)
            }
        }
    }
}
