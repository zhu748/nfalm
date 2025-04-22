use axum::{Json, extract::State};
use axum_auth::AuthBearer;
use rquest::StatusCode;
use tracing::{error, info, warn};

use crate::{VERSION_AUTHOR, config::CookieStatus, state::ClientState};

pub async fn api_submit(
    State(s): State<ClientState>,
    AuthBearer(t): AuthBearer,
    Json(mut c): Json<CookieStatus>,
) -> StatusCode {
    if !s.config.auth(&t) {
        return StatusCode::UNAUTHORIZED;
    }
    if !c.cookie.validate() {
        warn!("Invalid cookie: {}", c.cookie);
        return StatusCode::BAD_REQUEST;
    }
    c.reset_time = None;
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

pub async fn api_version() -> String {
    VERSION_AUTHOR.to_string()
}
