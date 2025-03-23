use crate::api::ApiState;
use axum::{
    Json,
    extract::{Request, State},
    http::HeaderMap,
};
use serde_json::Value;

fn cookie_changer(reset_timer: Option<bool>, cleanup: Option<bool>) {
    let reset_timer = reset_timer.unwrap_or(true);
    let cleanup = cleanup.unwrap_or(false);
    
}

pub async fn completion(
    State(state): State<ApiState>,
    header: HeaderMap,
    Json(payload): Json<Value>,
) {
    let temp = payload["temperature"].clone();
    // if temp is not f64 or within 0.1 to 1.0, set it to Null
    let temp = if temp.is_f64() && (0.1..=1.0).contains(&temp.as_f64().unwrap()) {
        temp
    } else {
        Value::Null
    };
    let message = payload["messages"].clone();
    let auth = header
        .get("Authorization")
        .unwrap()
        .to_str()
        .unwrap_or_default();
    let third_key = auth
        .trim_start_matches("oaiKey:")
        .trim_start_matches("3rdKey:");
    let is_oai_api = auth.matches("oaiKey:").count() > 0;
    let force_model = payload["model"]
        .as_str()
        .unwrap_or_default()
        .contains("--force");
    let api_keys = third_key.split(",").map(|s| s.trim()).collect::<Vec<_>>();
    // TODO: validate api keys
    let model = if !api_keys.is_empty() || force_model || *state.is_pro.lock().unwrap() {
        let m = payload["model"]
            .as_str()
            .unwrap_or_default()
            .replace("--force", "");
        m.trim().to_string()
    } else {
        state.cookie_model.lock().unwrap().clone()
    };
    let max_tokens_to_sample = payload["max_tokens"].as_u64().unwrap_or(0);
    let stop_sequence = payload["stop"].as_str().unwrap_or_default();
    let top_p = payload["top_p"].clone();
    let top_k = payload["top_k"].clone();
    let config = &state.config;
    if api_keys.is_empty()
        && (!config.proxy_password.is_empty()
            && auth != format!("Bearer {}", config.proxy_password)
            || state.uuid_org.lock().unwrap().is_empty())
    {
        let msg = if state.uuid_org.lock().unwrap().is_empty() {
            "No cookie available or apiKey format wrong"
        } else {
            "proxy_password Wrong"
        };
        panic!("{}", msg);
    } else if !*state.changing.lock().unwrap()
        && !api_keys.is_empty()
        && !*state.is_pro.lock().unwrap()
        && model != *state.cookie_model.lock().unwrap()
    {
        panic!("No cookie available");
    }
}
