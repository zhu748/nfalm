use axum::response::sse::Event;
use eventsource_stream::EventStreamError;
use futures::pin_mut;
use serde_json::Value;
use tokio_stream::{Stream, StreamExt};
use tracing::{error, warn};
use transform_stream::{AsyncTryStream, Yielder};

use crate::error::ClewdrError;

#[derive(Debug)]
pub struct ClewdrTransformer {
    streaming: bool,
    completes: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct StreamEventData {
    choices: Vec<StreamEventDelta>,
}

impl StreamEventData {
    fn new(content: String) -> Self {
        Self {
            choices: vec![StreamEventDelta {
                delta: EventContent { content },
            }],
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct NonStreamEventData {
    choices: Vec<NonStreamEventMessage>,
}

impl NonStreamEventData {
    fn new(content: String) -> Self {
        Self {
            choices: vec![NonStreamEventMessage {
                message: EventContent { content },
            }],
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct StreamEventDelta {
    delta: EventContent,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct NonStreamEventMessage {
    message: EventContent,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct EventContent {
    content: String,
}

impl ClewdrTransformer {
    pub fn new(streaming: bool) -> Self {
        Self {
            streaming,
            completes: Vec::with_capacity(1024),
        }
    }

    fn build(&self, selection: &str) -> Event {
        let event = Event::default();
        if self.streaming {
            let data = StreamEventData::new(selection.to_string());
            event.json_data(data).unwrap()
        } else {
            let data = NonStreamEventData::new(selection.to_string());
            event.json_data(data).unwrap()
        }
    }

    async fn parse_buf(&mut self, buf: &str, y: &mut Yielder<Result<Event, ClewdrError>>) {
        if buf.is_empty() {
            return;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(buf) else {
            warn!("Failed to parse JSON: {}", buf);
            return;
        };
        let Some(completion) = parsed
            .get("completion")
            .or(parsed.pointer("/delta/text"))
            .or(parsed.pointer("/choices/0/delta/content"))
            .and_then(|c| c.as_str())
            .map(|c| c.to_string())
        else {
            return;
        };
        if self.streaming {
            let event = self.build(completion.as_str());
            y.yield_ok(event).await;
        } else {
            self.completes.push(completion);
        }
    }

    async fn transform(
        &mut self,
        chunk: Result<eventsource_stream::Event, EventStreamError<rquest::Error>>,
        y: &mut Yielder<Result<Event, ClewdrError>>,
    ) -> Result<(), ClewdrError> {
        let event = chunk.map_err(ClewdrError::EventSourceError)?;
        let data = event.data;
        self.parse_buf(&data, y).await;
        Ok(())
    }

    async fn flush(&mut self, y: &mut Yielder<Result<Event, ClewdrError>>) {
        // Flush logic
        if !self.streaming {
            y.yield_ok(self.build(self.completes.join("").as_str()))
                .await;
        }
        if self.streaming {
            let event = Event::default();
            y.yield_ok(event.data("[DONE]")).await;
        }
    }

    pub fn transform_stream<S>(
        mut self,
        input: S,
    ) -> AsyncTryStream<
        Event,
        ClewdrError,
        impl std::future::Future<Output = Result<(), ClewdrError>> + Send,
    >
    where
        S: Stream<Item = Result<eventsource_stream::Event, EventStreamError<rquest::Error>>>
            + Send
            + 'static,
    {
        AsyncTryStream::new(move |mut y| async move {
            pin_mut!(input);

            while let Some(chunk) = input.next().await {
                if let Err(e) = self.transform(chunk, &mut y).await {
                    error!("Stream error: {}", e);
                }
            }
            self.flush(&mut y).await;
            Ok(())
        })
    }
}
