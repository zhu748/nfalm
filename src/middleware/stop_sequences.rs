use axum::response::{IntoResponse, Response, Sse, sse::Event};
use eventsource_stream::Eventsource;
use futures::{StreamExt, stream};
use trie_rs::inc_search::{IncSearch, Position};

use crate::types::message::{ContentBlockDelta, MessageDeltaContent, StreamEvent};

use super::ExtraContext;

pub async fn stop(resp: Response) -> Response {
    let Some(f) = resp.extensions().get::<ExtraContext>().cloned() else {
        return resp;
    };
    if !f.stream || resp.status() != 200 || f.stop_sequences.is_empty() {
        return resp;
    }

    let stream = resp.into_body().into_data_stream().eventsource();
    let trie =
        trie_rs::map::Trie::from_iter(f.stop_sequences.iter().cloned().map(|s| (s.to_owned(), s)));
    let search = trie.inc_search();
    let mut position = Position::from(search);
    let mut stop = false;
    let stream = stream.flat_map(move |event| {
        if stop {
            return stream::iter(vec![]);
        }
        let mut search = IncSearch::resume(&trie, position);
        let data = match event {
            Ok(eventsource_stream::Event { data, .. }) => data,
            Err(e) => return stream::iter(vec![Err(e)]),
        };
        let Ok(parsed) = serde_json::from_str::<StreamEvent>(&data) else {
            let event = Event::default();
            let event = event.data(data);
            return stream::iter(vec![Ok(event)]);
        };
        let event = Event::default();
        let event = event.json_data(&parsed).unwrap();
        let StreamEvent::ContentBlockDelta { ref delta, index } = parsed else {
            return stream::iter(vec![Ok(event)]);
        };
        let ContentBlockDelta::TextDelta { text } = delta else {
            return stream::iter(vec![Ok(event)]);
        };
        let input = text.as_bytes();
        let mut res = vec![Ok(event)];
        for i in 0..input.len() {
            match search.query(&input[i]) {
                None => {
                    search.reset();
                }
                Some(a) if a.is_match() => {
                    let seq = search.value().unwrap();
                    // stop sequence found
                    let result = String::from_utf8_lossy(&input[..i + 1]).to_string();
                    stop = true;
                    let event = StreamEvent::ContentBlockDelta {
                        delta: ContentBlockDelta::TextDelta { text: result },
                        index,
                    };
                    let content_block_stop = StreamEvent::ContentBlockStop { index: index + 1 };
                    let message_delta = StreamEvent::MessageDelta {
                        delta: MessageDeltaContent {
                            stop_reason: Some(crate::types::message::StopReason::StopSequence),
                            stop_sequence: Some(seq.to_string()),
                        },
                        usage: None,
                    };
                    let message_stop = StreamEvent::MessageStop;

                    res = vec![event, content_block_stop, message_delta, message_stop]
                        .iter()
                        .map(|e| {
                            let event = Event::default();
                            let event = event.json_data(e).unwrap();
                            Ok(event)
                        })
                        .collect::<Vec<_>>();
                    break;
                }
                _ => {}
            }
        }
        position = Position::from(search);
        return stream::iter(res);
    });
    let mut resp = Sse::new(stream)
        .keep_alive(Default::default())
        .into_response();
    resp.extensions_mut().insert(f);
    resp
}
