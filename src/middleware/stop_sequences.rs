use std::sync::Arc;

use axum::response::{IntoResponse, Response, Sse, sse::Event};
use eventsource_stream::Eventsource;
use futures::{StreamExt, stream};
use trie_rs::Trie;

use crate::types::message::{ContentBlockDelta, MessageDeltaContent, StreamEvent};

use super::FormatInfo;

pub async fn stop(resp: Response) -> impl IntoResponse {
    let Some(f) = resp.extensions().get::<FormatInfo>().cloned() else {
        return resp;
    };
    if !f.stream || resp.status() != 200 {
        return resp;
    }
    if f.stop_sequences.is_empty() {
        return resp;
    }

    let stream = resp.into_body().into_data_stream().eventsource();
    let trie = Arc::new(Trie::from_iter(f.stop_sequences.iter()));
    let mut search = trie.inc_search();
    let mut stop = false;
    let stream = stream.flat_map(move |event| {
        if stop {
            return stream::iter(vec![]);
        }
        match event {
            Ok(eventsource_stream::Event { data, .. }) => {
                let Ok(parsed) = serde_json::from_str::<StreamEvent>(&data) else {
                    return stream::iter(vec![]);
                };
                let StreamEvent::ContentBlockDelta { delta, index } = parsed else {
                    return stream::iter(vec![]);
                };
                let ContentBlockDelta::TextDelta { text } = delta else {
                    return stream::iter(vec![]);
                };
                let input = text.as_bytes();
                for i in 0..input.len() {
                    match search.query(&input[i]) {
                        None => search.reset(),
                        Some(a) if a.is_match() => {
                            // stop sequence found
                            let result = String::from_utf8_lossy(&input[..i]).to_string();
                            stop = true;
                            let event = StreamEvent::ContentBlockDelta {
                                delta: ContentBlockDelta::TextDelta { text: result },
                                index,
                            };
                            let content_block_stop =
                                StreamEvent::ContentBlockStop { index: index + 1 };
                            let message_delta = StreamEvent::MessageDelta {
                                delta: MessageDeltaContent {
                                    stop_reason: Some(
                                        crate::types::message::StopReason::StopSequence,
                                    ),
                                    stop_sequence: None, // TODO: add stop sequence
                                },
                                usage: None,
                            };
                            let message_stop = StreamEvent::MessageStop;

                            return stream::iter(
                                vec![event, content_block_stop, message_delta, message_stop]
                                    .iter()
                                    .map(|e| {
                                        let event = Event::default();
                                        let event = event.json_data(e).unwrap();
                                        Ok(event)
                                    })
                                    .collect::<Vec<_>>(),
                            );
                        }
                        _ => {
                            // continue searching
                        }
                    }
                }
                return stream::iter(vec![]);
            }
            Err(e) => stream::iter(vec![Err(e)]).into(),
        }
    });
    Sse::new(stream)
        .keep_alive(Default::default())
        .into_response()
}
