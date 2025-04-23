use std::sync::Arc;

use axum::Json;
use axum_auth::AuthBearer;
use rquest::StatusCode;

use crate::config::{CLEWDR_CONFIG, ClewdrConfig};

pub async fn api_get_config(
    AuthBearer(t): AuthBearer,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !CLEWDR_CONFIG.load().auth(&t) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized"
            })),
        ));
    }

    let config = CLEWDR_CONFIG.load_full();
    let mut config_json = serde_json::to_value(ClewdrConfig::clone(&config)).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to serialize config: {}", e)
            })),
        )
    })?;
    // remove cookie_array and wasted_cookie
    if let Some(obj) = config_json.as_object_mut() {
        obj.remove("cookie_array");
        obj.remove("wasted_cookie");
    }

    Ok(Json(config_json))
}

pub async fn api_post_config(
    AuthBearer(t): AuthBearer,
    Json(c): Json<ClewdrConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !CLEWDR_CONFIG.load().auth(&t) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized"
            })),
        ));
    }
    let c = c.validate();
    let mut new_c = c.clone();
    // add cookie_array and wasted_cookie
    new_c.cookie_array = CLEWDR_CONFIG.load().cookie_array.clone();
    new_c.wasted_cookie = CLEWDR_CONFIG.load().wasted_cookie.clone();
    // update config
    CLEWDR_CONFIG.store(Arc::new(new_c));

    Ok(Json(serde_json::json!({
        "message": "Config updated successfully",
        "config": c
    })))
}
