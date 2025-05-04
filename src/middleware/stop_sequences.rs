use async_stream::try_stream;
use axum::response::{IntoResponse, Response, Sse, sse::Event};
use eventsource_stream::Eventsource;
use futures::Stream;

use crate::types::message::{ContentBlockDelta, MessageDeltaContent, StopReason, StreamEvent};

use super::ExtraContext;

type EventResult<T> = Result<T, eventsource_stream::EventStreamError<axum::Error>>;

fn stop_stream<S>(
    sequences: Vec<String>,
    stream: S,
) -> impl Stream<Item = EventResult<Event>> + Send
where
    S: Stream<Item = EventResult<eventsource_stream::Event>> + Send + 'static,
{
    let trie = trie_rs::map::Trie::from_iter(sequences.into_iter().map(|s| (s.to_owned(), s)));
    try_stream!({
        let mut searches = vec![trie.inc_search()];
        for await event in stream {
            let eventsource_stream::Event { data, .. } = event?;
            let Ok(parsed) = serde_json::from_str::<StreamEvent>(&data) else {
                let event = Event::default();
                let event = event.data(data);
                yield event;
                continue;
            };
            let event = Event::default();
            let event = event.json_data(&parsed).unwrap();
            let StreamEvent::ContentBlockDelta { ref delta, index } = parsed else {
                yield event;
                continue;
            };
            let ContentBlockDelta::TextDelta { text } = delta else {
                yield event;
                continue;
            };
            let input = text.as_bytes();
            for i in 0..input.len() {
                let mut next_searches = vec![trie.inc_search()];
                for mut s in searches.into_iter() {
                    match s.query(&input[i]) {
                        // match found, return
                        Some(a) if a.is_match() => {
                            let seq = s.value().unwrap();
                            // stop sequence found
                            let result = String::from_utf8_lossy(&input[..i + 1]).to_string();
                            let event = StreamEvent::ContentBlockDelta {
                                delta: ContentBlockDelta::TextDelta { text: result },
                                index,
                            };
                            let content_block_stop = StreamEvent::ContentBlockStop { index };
                            let message_delta = StreamEvent::MessageDelta {
                                delta: MessageDeltaContent {
                                    stop_reason: Some(StopReason::StopSequence),
                                    stop_sequence: Some(seq.to_string()),
                                },
                                usage: None,
                            };
                            let message_stop = StreamEvent::MessageStop;

                            for e in [event, content_block_stop, message_delta, message_stop] {
                                let event = Event::default();
                                let event = event.json_data(e).unwrap();
                                yield event;
                            }
                            return;
                        }
                        // prefix found, add it to the next searches
                        Some(a) if a.is_prefix() => next_searches.push(s),
                        _ => (),
                    }
                }
                searches = next_searches;
            }
            yield event;
        }
    })
}

pub async fn apply_stop_sequences(resp: Response) -> Response {
    let Some(f) = resp.extensions().get::<ExtraContext>().cloned() else {
        return resp;
    };
    if !f.stream || resp.status() != 200 || f.stop_sequences.is_empty() {
        return resp;
    }

    let stream = resp.into_body().into_data_stream().eventsource();
    let stream = stop_stream(f.stop_sequences.to_owned(), stream);
    let mut resp = Sse::new(stream)
        .keep_alive(Default::default())
        .into_response();

    resp.extensions_mut().insert(f);
    resp
}
