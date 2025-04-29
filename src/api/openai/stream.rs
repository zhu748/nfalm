use axum::response::sse::Event;
use eventsource_stream::EventStreamError;
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

/// Represents the data structure for non-streaming responses in OpenAI API format
/// Contains a choices array with complete messages
#[derive(Debug, Serialize)]
pub struct NonStreamEventData {
    choices: Vec<NonStreamEventMessage>,
}

impl NonStreamEventData {
    /// Creates a new NonStreamEventData with the given content
    ///
    /// # Arguments
    /// * `content` - The complete response text
    ///
    /// # Returns
    /// A new NonStreamEventData instance with the content wrapped in choices array
    pub fn new(content: String) -> Self {
        Self {
            choices: vec![NonStreamEventMessage {
                message: EventContent::Content { content },
            }],
        }
    }
}

/// Represents a delta update in a streaming response
/// Contains the content change for the current chunk
#[derive(Debug, Serialize)]
struct StreamEventDelta {
    delta: EventContent,
}

/// Represents a complete message in a non-streaming response
/// Contains the full message content
#[derive(Debug, Serialize)]
struct NonStreamEventMessage {
    message: EventContent,
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
pub fn transform_stream<I>(s: I) -> impl Stream<Item = Result<Event, ClewdrError>> + Send + 'static
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
                    let parsed = serde_json::from_str::<StreamEvent>(&data).ok()?;
                    match parsed {
                        StreamEvent::ContentBlockDelta { delta, .. } => match delta {
                            ContentBlockDelta::TextDelta { text } => {
                                Some(Ok(build_event(EventContent::Content { content: text })))
                            }
                            ContentBlockDelta::ThinkingDelta { thinking } => {
                                Some(Ok(build_event(EventContent::Reasoning {
                                    reasoning_content: thinking,
                                })))
                            }
                            _ => None,
                        },
                        _ => None,
                    }
                }
                Err(e) => Some(Err(e.into())),
            }
        }
    })
}
