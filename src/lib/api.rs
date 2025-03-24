use std::{
    collections::HashMap,
    process::exit,
    sync::{Arc, LazyLock, Mutex, RwLock},
    time::{Duration, SystemTime},
};

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
use tokio::{
    spawn,
    time::{Interval, timeout},
};

use crate::{
    completion::completion,
    config::Config,
    utils::{ENDPOINT, MODELS},
};

pub struct RouterBuilder {
    inner: Router,
}

#[derive(Default)]
pub struct MyState {
    pub config: RwLock<Config>,
    model_list: RwLock<Vec<String>>,
    pub is_pro: RwLock<bool>,
    pub cookie_model: RwLock<String>,
    pub uuid_org: RwLock<String>,
    pub changing: RwLock<bool>,
    pub change_flag: RwLock<usize>,
    pub current_index: RwLock<usize>,
    pub first_login: RwLock<bool>,
    pub timestamp: RwLock<u128>,
    pub change_times: RwLock<usize>,
    pub total_times: usize,
    pub model: RwLock<String>,
    pub cookies: RwLock<HashMap<String, String>>,
}

#[derive(Clone)]
pub struct AppState(pub Arc<MyState>);

impl AppState {
    pub fn new(config: Config) -> Self {
        let total_times = config.cookie_array.len();
        let m = MyState {
            config: RwLock::new(config),
            first_login: RwLock::new(true),
            total_times,
            ..Default::default()
        };
        let m = Arc::new(m);
        AppState(m)
    }

    fn cookie_changer(&self, reset_timer: Option<bool>, cleanup: Option<bool>) -> bool {
        let my_state = self.0.clone();
        let reset_timer = reset_timer.unwrap_or(true);
        let cleanup = cleanup.unwrap_or(false);
        let config = self.0.config.read().unwrap();
        if config.cookie_array.is_empty() {
            *my_state.changing.write().unwrap() = false;
            false
        } else {
            *my_state.change_flag.write().unwrap() = 0;
            *my_state.changing.write().unwrap() = true;
            if !cleanup {
                // rotate the cookie
                let mut index = my_state.current_index.write().unwrap();
                let array_len = config.cookie_array.len();
                *index = (*index + 1) % array_len;
                println!("{}", "Changing cookie".green());
            }
            // set timeout callback
            let dur = if config.rproxy.is_empty() || config.rproxy == ENDPOINT {
                15000 + my_state.timestamp.read().unwrap().clone()
                    - SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
            } else {
                0
            };
            let dur = Duration::from_millis(dur as u64);
            let self_clone = self.clone();
            spawn(timeout(dur, async move {
                self_clone.on_listen();
                if reset_timer {
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    *my_state.timestamp.write().unwrap() = now;
                }
            }));
            false
        }
    }

    fn on_listen(&self) -> bool {
        let my_state = self.0.clone();
        let mut config = my_state.config.write().unwrap();
        if my_state.first_login.read().unwrap().clone() {
            *my_state.first_login.write().unwrap() = false;
            // get time now
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            *my_state.timestamp.write().unwrap() = now;
            const TITLE: &str = formatc!(
                "Clewdr v{} by {}",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_AUTHORS")
            );
            let addr = config.ip.clone() + ":" + &config.port.to_string();
            println!("{}", TITLE.blue());
            println!("Listening on {}", addr.green());
            // TODO: Print the config
            // TODO: Local tunnel
        }
        if !config.cookie_array.is_empty() {
            let current_cookie = config.current_cookie_info().unwrap();
            config.cookie = current_cookie.cookie.clone();

            *my_state.change_times.write().unwrap() += 1;
            if !my_state.model.read().unwrap().is_empty() {
                //TODO: check cookie prefix "claude"
            }
        }
        let percentage = ((*my_state.change_times.read().unwrap() as f32)
            + config.cookie_index.saturating_sub(1) as f32)
            / (my_state.total_times as f32)
            * 100.0;
        if !config.cookie.validate() {
            *my_state.changing.write().unwrap() = false;
            print!("{}", "No cookie available, enter apiKey-only mode.".red());
            return false;
        }
        false
    }
}

impl RouterBuilder {
    pub fn new(config: Config) -> Self {
        let api_state = AppState::new(config);
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
    State(api_state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let api_state = api_state.0;
    let authorization = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    // TODO: get api_rproxy from url query
    let api_rproxy = api_state.config.read().unwrap().api_rproxy.clone();
    let models = if authorization.matches("oaiKey:").count() > 0 && !api_rproxy.is_empty() {
        static CLIENT: LazyLock<Client> = LazyLock::new(|| {
            ClientBuilder::new()
                .build()
                .expect("Failed to create client")
        });
        let url = format!("{}/v1/models", api_rproxy);
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
    let config = api_state.config.read().unwrap();
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
    let mut model_list = api_state.model_list.write().unwrap();
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
