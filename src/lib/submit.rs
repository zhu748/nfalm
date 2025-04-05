use axum::{Json, extract::State};
use tracing::{error, info};

use crate::{config::CookieStatus, messages::Auth, state::AppState};

pub async fn api_submit(State(s): State<AppState>, Auth(_): Auth, Json(mut c): Json<CookieStatus>) {
    c.reset_time = None;
    if !c.cookie.validate() {
        error!("Invalid cookie: {}", c.cookie);
        return;
    }
    info!("Cookie accepted: {}", c.cookie);
    s.submit_tx.send(c).await.unwrap_or_else(|_| {
        error!("Failed to send cookie to submit channel");
    });
}
