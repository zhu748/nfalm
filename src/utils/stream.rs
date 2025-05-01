use axum::response::sse::Event;
use futures::{Stream, StreamExt};
use serde::Serialize;

use crate::{
    error::ClewdrError,
    types::message::{ContentBlockDelta, StreamEvent},
};

/// Represents the data structure for streaming events in OpenAI API format
/// Contains a choices array with deltas of content
#[derive(Debug, Serialize)]
struct StreamEventData {
    choices: Vec<StreamEventDelta>,
}

impl StreamEventData {
    /// Creates a new StreamEventData with the given content
    ///
    /// # Arguments
    /// * `content` - The event content to include
    ///
    /// # Returns
    /// A new StreamEventData instance with the content wrapped in choices array
    fn new(content: EventContent) -> Self {
        Self {
            choices: vec![StreamEventDelta { delta: content }],
        }
    }
}

/// Represents a delta update in a streaming response
/// Contains the content change for the current chunk
#[derive(Debug, Serialize)]
struct StreamEventDelta {
    delta: EventContent,
}

/// Content of an event, either regular content or reasoning (thinking mode)
/// Uses untagged enum to handle different response formats
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum EventContent {
    Content { content: String },
    Reasoning { reasoning_content: String },
}

/// Creates an SSE event with the given content in OpenAI format
///
/// # Arguments
/// * `content` - The event content to include
///
/// # Returns
/// A formatted SSE Event ready to be sent to the client
fn build_event(content: EventContent) -> Event {
    let event = Event::default();
    let data = StreamEventData::new(content);
    event.json_data(data).unwrap()
}

/// Transforms a Claude.ai event stream into an OpenAI-compatible event stream
/// Extracts content from Claude events and reformats them to match OpenAI's streaming format
///
/// # Arguments
/// * `s` - The input stream of Claude.ai events
///
/// # Returns
/// A stream of OpenAI-compatible SSE events
pub fn transform_stream<I, E>(
    s: I,
) -> impl Stream<Item = Result<Event, ClewdrError>> + Send + 'static
where
    I: Stream<Item = Result<eventsource_stream::Event, E>> + Send + 'static,
    E: Into<ClewdrError> + Send + 'static,
{
    s.filter_map(async |event| match event {
        Ok(eventsource_stream::Event { data, .. }) => {
            let parsed = serde_json::from_str::<StreamEvent>(&data).ok()?;
            let StreamEvent::ContentBlockDelta { delta, .. } = parsed else {
                return None;
            };
            match delta {
                ContentBlockDelta::TextDelta { text } => {
                    Some(Ok(build_event(EventContent::Content { content: text })))
                }
                ContentBlockDelta::ThinkingDelta { thinking } => {
                    Some(Ok(build_event(EventContent::Reasoning {
                        reasoning_content: thinking,
                    })))
                }
                _ => None,
            }
        }
        Err(e) => Some(Err(e.into())),
    })
}
