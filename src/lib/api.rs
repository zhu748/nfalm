use std::sync::{Arc, LazyLock, Mutex};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{get, options, post},
};
use colored::Colorize;
use const_format::{concatc, formatc};
use rquest::{Client, ClientBuilder};
use serde_json::{Map, Value, json};

use crate::{completion::completion, config::Config, utils::MODELS};

pub struct RouterBuilder {
    inner: Router,
}

#[derive(Default, Clone)]
pub struct ApiState {
    pub config: Arc<Config>,
    model_list: Arc<Mutex<Vec<String>>>,
    pub is_pro: Arc<Mutex<bool>>,
    pub cookie_model: Arc<Mutex<String>>,
    pub uuid_org: Arc<Mutex<String>>,
    pub changing: Arc<Mutex<bool>>,
    pub change_flag: Arc<Mutex<u32>>,
    pub current_index: Arc<Mutex<u32>>,
    first_login: Arc<Mutex<bool>>,
    timestamp: Arc<Mutex<u64>>,
    change_time: Arc<Mutex<u64>>,
    total_time: Arc<Mutex<u64>>,
    model: Arc<Mutex<String>>,
}

impl ApiState {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            first_login: Arc::new(Mutex::new(true)),
            ..Default::default()
        }
    }

    fn cookie_changer(&self, reset_timer: Option<bool>, cleanup: Option<bool>) -> bool {
        let reset_timer = reset_timer.unwrap_or(true);
        let cleanup = cleanup.unwrap_or(false);
        if self.config.cookie_array.is_empty() {
            !self.changing.lock().unwrap().clone()
        } else {
            *self.change_flag.lock().unwrap() = 0;
            *self.changing.lock().unwrap() = true;
            if !cleanup {
                // rotate the cookie
                let mut index = self.current_index.lock().unwrap();
                let array_len = self.config.cookie_array.len() as u32;
                *index = (*index + 1) % array_len;
                println!("{}", "Changing cookie".green());
            }
            false
        }
    }

    fn on_listen(&self) {
        if self.first_login.lock().unwrap().clone() {
            *self.first_login.lock().unwrap() = false;
            // get time now
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            *self.timestamp.lock().unwrap() = now;
            *self.total_time.lock().unwrap() = self.config.api_rproxy.len() as u64;
            const TITLE: &str = formatc!(
                "Clewdr v{} by {}",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_AUTHORS")
            );
            let addr = self.config.ip.clone() + ":" + &self.config.port.to_string();
            println!("{}", TITLE.blue());
            println!("Listening on {}", addr.green());
            // TODO: Print the config
            // TODO: Local tunnel
        }
        if !self.config.cookie_array.is_empty() {
            let current_cookie = self
                .config
                .cookie_array
                .get(*self.current_index.lock().unwrap() as usize)
                .cloned()
                .unwrap_or_default();
            // if not start with sessionKey=, add it
            let current_cookie = if current_cookie.starts_with("sessionKey=") {
                current_cookie
            } else {
                format!("sessionKey={}", current_cookie)
            };
            *self.change_time.lock().unwrap() += 1;
            if (!self.model.lock().unwrap().is_empty()) {}
        }
    }
}

impl RouterBuilder {
    pub fn new(config: Arc<Config>) -> Self {
        let api_state = ApiState::new(config);
        Self {
            inner: Router::new()
                .route("/v1/models", get(get_models))
                .route("/v1/chat/completions", post(completion))
                .route("/v1/complete", post(api_complete))
                .route("/v1", options(api_options))
                .route("/", options(api_options))
                .fallback(api_fallback)
                .with_state(api_state),
        }
    }

    pub fn build(self) -> Router {
        self.inner
    }
}
async fn api_options() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Headers",
        "Authorization, Content-Type".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Methods",
        "POST, GET, OPTIONS".parse().unwrap(),
    );
    headers
}

async fn get_models(
    headers: HeaderMap,
    State(api_state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    let authorization = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    // TODO: get api_rproxy from url query
    let config = &api_state.config;
    let models = if authorization.matches("oaiKey:").count() > 0 && !config.api_rproxy.is_empty() {
        static CLIENT: LazyLock<Client> = LazyLock::new(|| {
            ClientBuilder::new()
                .build()
                .expect("Failed to create client")
        });
        let url = format!("{}/v1/models", config.api_rproxy);
        let key = authorization.replace("oaiKey:", ",");
        if let Some((key, _)) = key.split_once(",") {
            let key = key.trim();
            let resp = CLIENT
                .get(&url)
                .header("Authorization", key)
                .send()
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let models = resp
                .json::<Value>()
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            models["data"].as_array().cloned().unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    let mut data = MODELS
        .iter()
        .cloned()
        .chain(config.unknown_models.iter().map(|s| s.as_str()))
        .map(|name| {
            json!({
                "id":name,
            })
        })
        .chain(models)
        .collect::<Vec<_>>();
    data.sort_unstable_by(|a, b| {
        let a = a["id"].as_str().unwrap_or("");
        let b = b["id"].as_str().unwrap_or("");
        a.cmp(b)
    });
    data.dedup();
    // write to model_list
    let mut model_list = api_state.model_list.lock().unwrap();
    model_list.clear();
    model_list.extend(
        data.iter()
            .filter_map(|model| model["id"].as_str().map(|s| s.to_string())),
    );
    let response = json!({
        "data": data,
    });
    Ok(Json(response))
}

async fn api_complete() -> (StatusCode, Json<Value>) {
    let json = json!(
        {
            "error":{
                "message":                "Clewdr: Set \"Chat Completion source\" to OpenAI instead of Claude. Enable \"External\" models aswell",
                "code": 404,
            }
        }
    );
    (StatusCode::NOT_FOUND, Json(json))
}

async fn api_fallback(req: Request) -> Html<&'static str> {
    let url = req.uri().path();
    if !["/", "/v1", "/favicon.ico"].contains(&url) {
        println!("Unknown request url: {}", url);
    }
    const VX_BY_AUTHOR: &str = formatc!(
        "v{} by {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );
    Html(concatc!(
        r#"
<html><head>
<meta charset="utf-8">
<script>
function copyToClipboard(text) {
  var textarea = document.createElement("textarea");
  textarea.textContent = text;
  textarea.style.position = "fixed";
  document.body.appendChild(textarea);
  textarea.select();
  try {
    return document.execCommand("copy");
  } catch (ex) {
    console.warn("Copy to clipboard failed.", ex);
    return false;
  } finally {
    document.body.removeChild(textarea);
  }
}
function copyLink(event) {
  event.preventDefault();
  const url = new URL(window.location.href);
  const link = url.protocol + '//' + url.host + '/v1';
  copyToClipboard(link);
  alert('链接已复制: ' + link);
}
</script>
<style id="VMst0.014418824593286361">rt.katakana-terminator-rt::before { content: attr(data-rt); }</style><script id="simplify-jobs-page-script" src="chrome-extension://pbanhockgagggenencehbnadejlgchfc/js/pageScript.bundle.js"></script></head>
<body>
Clewdr "#,
        VX_BY_AUTHOR,
        r#"<br><br>完全开源、免费且禁止商用<br><br>点击复制反向代理: <a href="v1" onclick="copyLink(event)">Copy Link</a><br>填入OpenAI API反向代理并选择OpenAI分类中的claude模型（酒馆需打开Show "External" models，仅在api模式有模型选择差异）<br><br>教程与FAQ: <a href="https://rentry.org/teralomaniac_clewd" target="FAQ">Rentry</a> | <a href="https://discord.com/invite/B7Wr25Z7BZ" target="FAQ">Discord</a><br><br><br>❗警惕任何高风险cookie/伪api(25k cookie)购买服务，以及破坏中文AI开源共享环境倒卖免费资源抹去署名的群组（🈲黑名单：酒馆小二、AI新服务、浅睡(鲑鱼)、赛博女友制作人(青麈/overloaded/科普晓百生)🈲）</body></html>"#
    ))
}
