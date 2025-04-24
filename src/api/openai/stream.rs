use axum::response::sse::Event;
use eventsource_stream::EventStreamError;
use futures::{Stream, StreamExt, pin_mut};
use serde_json::Value;
use transform_stream::{AsyncTryStream, Yielder};

use crate::error::ClewdrError;

#[derive(Debug)]
pub struct ClewdrTransformer {}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct StreamEventData {
    choices: Vec<StreamEventDelta>,
}

impl StreamEventData {
    fn new(content: EventContent) -> Self {
        Self {
            choices: vec![StreamEventDelta { delta: content }],
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NonStreamEventData {
    choices: Vec<NonStreamEventMessage>,
}

impl NonStreamEventData {
    pub fn new(content: String) -> Self {
        Self {
            choices: vec![NonStreamEventMessage {
                message: EventContent::Content { content },
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
#[serde(untagged)]
enum EventContent {
    Content { content: String },
    Reasoning { reasoning_content: String },
}

impl ClewdrTransformer {
    pub fn new() -> Self {
        Self {}
    }

    fn build_event(&self, content: EventContent) -> Event {
        let event = Event::default();
        let data = StreamEventData::new(content);
        event.json_data(data).unwrap()
    }

    async fn parse_event(
        &mut self,
        event: eventsource_stream::Event,
        y: &mut Yielder<Result<Event, ClewdrError>>,
    ) {
        let buf = &event.data;
        if buf.is_empty() {
            return;
        }

        let Ok(parsed) = serde_json::from_str::<Value>(buf) else {
            return;
        };

        if let Some(thinking) = parsed["delta"]["thinking"].as_str() {
            let event = self.build_event(EventContent::Reasoning {
                reasoning_content: thinking.to_string(),
            });
            y.yield_ok(event).await;
            return;
        }

        let Some(completion) = parsed
            .get("completion")
            .or(parsed.pointer("/delta/text"))
            .or(parsed.pointer("/choices/0/delta/content"))
            .and_then(|c| c.as_str())
        else {
            return;
        };

        let event = self.build_event(EventContent::Content {
            content: completion.to_string(),
        });
        y.yield_ok(event).await;
    }

    async fn flush(&mut self, y: &mut Yielder<Result<Event, ClewdrError>>) {
        // Flush logic
        let event = Event::default();
        y.yield_ok(event.data("[DONE]")).await;
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
                match chunk {
                    Ok(event) => {
                        self.parse_event(event, &mut y).await;
                    }
                    Err(e) => {
                        y.yield_err(e.into()).await;
                    }
                }
            }
            self.flush(&mut y).await;
            Ok(())
        })
    }
}
