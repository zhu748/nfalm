use anyhow::{Error, Result};
use chrono::format;
use figlet_rs::FIGfont;
use rquest::Response;
use serde_json::{Value, json};
use std::sync::LazyLock;
use tracing::error;

pub static BANNER: LazyLock<String> = LazyLock::new(|| {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("Clewdr");
    let banner = figure.unwrap().to_string();
    format!(
        "{}\nv{} by {}\n",
        banner,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    )
});

pub const MODELS: [&str; 10] = [
    "claude-3-7-sonnet-20250219",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    "claude-2.1",
    "claude-2.0",
    "claude-1.3",
    "claude-instant-1.2",
    "claude-instant-1.1",
];

pub const ENDPOINT: &str = "https://api.claude.ai";

pub fn header_ref(ref_path: &str) -> String {
    if ref_path.is_empty() {
        format!("{}/", ENDPOINT)
    } else {
        format!("{}/chat/{}", ENDPOINT, ref_path)
    }
}

pub async fn check_res_err(res: Response, ret_json: &mut Option<Value>) -> Result<Value> {
    let mut ret = json!({
        "name": "Error",
    });
    let status = res.status();
    if !status.is_success() {
        ret["message"] = json!(format!("Unexpected response code: {}", status));
        error!("Unexpected response code: {}", status);
    }
    let text = res.text().await.unwrap_or_default();
    let json = serde_json::from_str::<Value>(&text).inspect_err(|e| {
        error!("Failed to parse response: {}\n{}", e, text);
    })?;
    if !json.is_null() {
        *ret_json = Some(json.clone());
    }
    let Some(err_api) = json.get("error") else {
        return Ok(ret);
    };
    ret["status"] = json["status"].clone();
    ret["planned"] = json!(true);
    if !err_api["message"].is_null() {
        ret["message"] = err_api["message"].clone();
    }
    if !err_api["type"].is_null() {
        ret["type"] = err_api["type"].clone();
    }
    if status == 429 {
        if let Some(time) = err_api["message"]
            .as_str()
            .and_then(|m| serde_json::from_str::<Value>(m).ok())
            .and_then(|m| m["resetAt"].as_str().map(|s| s.to_string()))
        {
            let reset_time = chrono::DateTime::parse_from_rfc3339(&time)
                .unwrap()
                .to_utc();
            let now = chrono::Utc::now();
            let diff = reset_time - now;
            let hours = diff.num_hours();
            ret.as_object_mut()
                .and_then(|obj| obj.get_mut("message"))
                .map(|msg| {
                    let new_msg = format!(", expires in {hours} hours");
                    if let Some(str) = msg.as_str() {
                        *msg = format!("{str}{new_msg}").into();
                    } else {
                        *msg = new_msg.into();
                    }
                });
        }
    }
    Err(Error::msg(ret))
}
