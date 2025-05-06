use axum::response::{IntoResponse, Response, Sse, sse::Event};
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde::Serialize;

use crate::{
    claude_state::ClaudeApiFormat,
    types::claude_message::{ContentBlockDelta, StreamEvent},
};

use super::ClaudeContext;

/// Transforms responses to ensure compatibility with the OpenAI API format
///
/// This middleware function analyzes responses and transforms them when necessary
/// to ensure compatibility between Claude and OpenAI API formats, particularly
/// for streaming responses. If the response is:
///
/// - From the Claude API format: No transformation needed
/// - Not streaming: No transformation needed
/// - Has a non-200 status code: No transformation needed
/// - OpenAI format and streaming: Transforms the stream to match OpenAI event format
///
/// # Arguments
///
/// * `resp` - The original response to be potentially transformed
///
/// # Returns
///
/// The original or transformed response as appropriate
pub async fn to_oai(resp: Response) -> impl IntoResponse {
    let Some(f) = resp.extensions().get::<ClaudeContext>() else {
        return resp;
    };
    if ClaudeApiFormat::Claude == f.api_format || !f.stream || resp.status() != 200 {
        return resp;
    }
    let body = resp.into_body();
    let stream = body.into_data_stream().eventsource();
    let stream = transform_stream(stream);
    Sse::new(stream)
        .keep_alive(Default::default())
        .into_response()
}

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
pub enum EventContent {
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
pub fn build_event(content: EventContent) -> Event {
    let event = Event::default();
    let data = StreamEventData::new(content);
    event.json_data(data).unwrap()
}

/// Transforms a Claude.ai event stream into an OpenAI-compatible event stream
///
/// Extracts content from Claude events and reformats them to match OpenAI's streaming format.
/// This function processes each event in the stream, identifying the delta content type
/// (text or thinking), and converting it to the appropriate OpenAI-compatible event format.
///
/// # Arguments
/// * `s` - The input stream of Claude.ai events
///
/// # Returns
/// A stream of OpenAI-compatible SSE events
///
/// # Type Parameters
/// * `I` - The input stream type
/// * `E` - The error type for the stream
pub fn transform_stream<I, E>(s: I) -> impl Stream<Item = Result<Event, E>> + Send
where
    I: Stream<Item = Result<eventsource_stream::Event, E>> + Send,
    E: Send,
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
        Err(e) => Some(Err(e)),
    })
}
