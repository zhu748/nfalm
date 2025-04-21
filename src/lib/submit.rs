use axum::{Json, extract::State};
use rquest::StatusCode;
use tracing::{error, info, warn};

use crate::{config::CookieStatus, messages::Auth, state::ClientState};

pub async fn api_submit(
    State(s): State<ClientState>,
    Auth(_): Auth,
    Json(mut c): Json<CookieStatus>,
) -> StatusCode {
    if !c.cookie.validate() {
        warn!("Invalid cookie: {}", c.cookie);
        return StatusCode::BAD_REQUEST;
    }
    c.reset_time = None;
    if let Some(t) = c.due {
        if t < chrono::Utc::now().timestamp() {
            warn!("Past payment due date: {}", c.cookie);
            c.due = None;
        }
    }
    info!("Cookie accepted: {}", c.cookie);
    match s.event_sender.submit(c).await {
        Ok(_) => {
            info!("Cookie submitted successfully");
            StatusCode::OK
        }
        Err(e) => {
            error!("Failed to submit cookie: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
