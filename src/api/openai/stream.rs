use axum::response::sse::Event;
use eventsource_stream::EventStreamError;
use futures::{Stream, StreamExt};
use serde_json::Value;

use crate::error::ClewdrError;

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

fn build_event(content: EventContent) -> Event {
    let event = Event::default();
    let data = StreamEventData::new(content);
    event.json_data(data).unwrap()
}

pub fn transform<I>(s: I) -> impl Stream<Item = Result<Event, ClewdrError>> + Send + 'static
where
    I: Stream<Item = Result<eventsource_stream::Event, EventStreamError<rquest::Error>>>
        + Send
        + 'static,
{
    s.filter_map(|event| {
        let event = event.map(|e| e.data);
        async move {
            match event {
                Ok(data) => {
                    let parsed = serde_json::from_str::<Value>(&data).ok()?;
                    if let Some(thinking) = parsed["delta"]["thinking"].as_str() {
                        return Some(Ok::<Event, ClewdrError>(build_event(
                            EventContent::Reasoning {
                                reasoning_content: thinking.to_string(),
                            },
                        )));
                    }
                    let completion = parsed
                        .get("completion")
                        .or(parsed.pointer("/delta/text"))
                        .or(parsed.pointer("/choices/0/delta/content"))
                        .and_then(|c| c.as_str())?;
                    Some(Ok(build_event(EventContent::Content {
                        content: completion.to_string(),
                    })))
                }
                Err(e) => Some(Err(e.into())),
            }
        }
    })
}
