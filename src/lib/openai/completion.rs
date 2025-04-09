use std::mem;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response, Sse},
};
use axum_auth::AuthBearer;
use colored::Colorize;
use eventsource_stream::Eventsource;
use rquest::{StatusCode, header::ACCEPT};
use scopeguard::defer;
use serde_json::json;
use tokio::spawn;
use tracing::{debug, info, warn};

use crate::{
    client::AppendHeaders,
    error::{ClewdrError, check_res_err},
    messages::{ClientRequestBody, non_stream_message},
    openai::stream::ClewdrTransformer,
    state::AppState,
    utils::print_out_json,
};

/// Axum handler for the API messages
pub async fn api_completion(
    AuthBearer(token): AuthBearer,
    State(mut state): State<AppState>,
    Json(p): Json<ClientRequestBody>,
) -> Response {
    if !state.config.auth(&token) {
        return (StatusCode::UNAUTHORIZED, Json("Unauthorized".to_string())).into_response();
    }
    // TODO: Check if the request is a test message

    let stream = p.stream;
    let stopwatch = chrono::Utc::now();
    info!(
        "Request received, stream mode: {}, messages: {}, model: {}",
        stream.to_string().green(),
        p.messages.len().to_string().green(),
        p.model.to_string().green()
    );

    if let Err(e) = state.request_cookie().await {
        return Json(json!(
            {
                "error": {
                    "message": e.to_string(),
                    "type": "invalid_request_error",
                    "param": null,
                    "code": 500
                }
            }
        ))
        .into_response();
    }
    let mut state_clone = state.clone();
    defer! {
        // ensure the cookie is returned
        spawn(async move {
            let dur = chrono::Utc::now().signed_duration_since(stopwatch);
            info!(
                "Request finished, elapsed time: {} seconds",
                dur.num_seconds().to_string().green()
            );
            state_clone.return_cookie(None).await;
        });
    }
    // check if request is successful
    match state.bootstrap().await.and(state.try_completion(p).await) {
        Ok(b) => {
            if let Err(e) = state.delete_chat().await {
                warn!("Failed to delete chat: {}", e);
            }
            b.into_response()
        }
        Err(e) => {
            // delete chat after an error
            if let Err(e) = state.delete_chat().await {
                warn!("Failed to delete chat: {}", e);
            }
            warn!("Error: {}", e);
            // 429 error
            match e {
                ClewdrError::InvalidCookie(ref r) => {
                    state.return_cookie(Some(r.clone())).await;
                }
                ClewdrError::OtherHttpError(c, e) => {
                    state.return_cookie(None).await;
                    return (c, Json(e.error)).into_response();
                }
                _ => {
                    state.return_cookie(None).await;
                }
            }

            // return the error as a response
            Json(json! {
                {
                    "error": {
                        "message": e.to_string(),
                        "type": "invalid_request_error",
                        "param": null,
                        "code": 500
                    }
                }
            })
            .into_response()
        }
    }
}

impl AppState {
    /// Try to send a message to the Claude API
    async fn try_completion(&mut self, mut p: ClientRequestBody) -> Result<Response, ClewdrError> {
        print_out_json(&p, "0.req.json");
        let stream = p.stream;
        let proxy = self.config.rquest_proxy.clone();
        let Some(ref org_uuid) = self.org_uuid else {
            return Ok(Json(json!(
                {
                    "error": {
                        "message": "No organization found, please check your cookie.",
                        "type": "invalid_request_error",
                        "param": null,
                        "code": 500
                    }
                }
            ))
            .into_response());
        };

        // Create a new conversation
        let new_uuid = uuid::Uuid::new_v4().to_string();
        self.conv_uuid = Some(new_uuid.to_string());
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations",
            self.config.endpoint(),
            org_uuid
        );
        let mut body = json!({
            "uuid": new_uuid,
            "name":""
        });
        // enable thinking mode
        if p.model.contains("-thinking") {
            body["paprika_mode"] = "extended".into();
            body["model"] = p.model.clone().into();
            p.model = p.model.trim_end_matches("-thinking").to_string();
        }

        let api_res = self
            .client
            .post(endpoint)
            .json(&body)
            .append_headers("", proxy.clone())
            .send()
            .await?;
        debug!("New conversation created: {}", new_uuid);

        check_res_err(api_res).await?;

        // generate the request body
        // check if the request is empty
        let Some(mut body) = self.transform(p) else {
            return Ok(Json(non_stream_message(
                "Empty request, please send a message.".to_string(),
            ))
            .into_response());
        };

        // check images
        let images = mem::take(&mut body.images);

        // upload images
        let files = self.upload_images(images).await;
        body.files = files;

        // send the request
        print_out_json(&body, "4.req.json");
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}/completion",
            self.config.endpoint(),
            org_uuid,
            new_uuid
        );

        let api_res = self
            .client
            .post(endpoint)
            .json(&body)
            .append_headers("", proxy)
            .header_append(ACCEPT, "text/event-stream")
            .send()
            .await?;

        let api_res = check_res_err(api_res).await?;

        // stream the response
        let input_stream = api_res.bytes_stream().eventsource();
        let trans = ClewdrTransformer::new(stream);
        let output = trans.transform_stream(input_stream);
        Ok(Sse::new(output).into_response())
    }
}
