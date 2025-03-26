use crate::{
    SUPER_CLIENT,
    api::{AppState, InnerState},
    stream::{ClewdrConfig, ClewdrTransformer},
};
use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::HeaderMap,
};
use bytes::Bytes;
use futures::pin_mut;
use serde::{de, ser};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tracing::info;

pub async fn stream_example(
    State(state): State<AppState>,
    header: HeaderMap,
    Json(payload): Json<Value>,
) -> Body {
    // Create a channel for streaming response chunks to the client
    let (tx, rx) = mpsc::channel::<Result<Bytes, axum::Error>>(32);

    // Configure the transformer
    let config = ClewdrConfig::new("xx", "pro", true, 8, true);
    let trans = ClewdrTransformer::new(config);

    // Perform the external request
    let super_res = SUPER_CLIENT
        .get("https://api.claude.ai")
        .send()
        .await
        .unwrap(); // In production, handle this error gracefully
    
    // Spawn a task to handle the streaming transformation
    tokio::spawn(async move {
        let input_stream = super_res.bytes_stream();
        let output_stream = trans.transform_stream(input_stream);
        pin_mut!(output_stream);

        while let Some(result) = output_stream.next().await {
            // Simulate expensive work (optional, adjust as needed)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Send the chunk to the client
            let chunk = Bytes::from(result.unwrap()); // Convert String to Bytes
            if tx.send(Ok(chunk)).await.is_err() {
                info!("Client disconnected, cancelling task");
                break;
            }
        }
    });

    // Return the streaming body
    let response_stream = ReceiverStream::new(rx);
    Body::from_stream(response_stream)
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ClientRequestBody {
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(default)]
    messages: Vec<Value>,
    #[serde(default)]
    model: String,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    max_tokens: Option<i64>,
    #[serde(default)]
    stop: Vec<String>,
    #[serde(default)]
    top_p: Option<f64>,
    #[serde(default)]
    top_k: Option<i64>,
}

fn sanitize_client_request(body: &mut ClientRequestBody, state: &InnerState) {
    if let Some(ref mut temp) = body.temperature {
        *temp = temp.clamp(0.0, 1.0);
    }
}

pub async fn completion(
    State(state): State<AppState>,
    header: HeaderMap,
    Json(payload): Json<ClientRequestBody>,
) {
    let auth = header.get("Authorization").and_then(|h| h.to_str().ok());
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    // TODO
}
