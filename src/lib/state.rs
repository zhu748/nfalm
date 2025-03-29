use std::{collections::HashMap, sync::Arc};

use colored::Colorize;
use parking_lot::RwLock;
use regex::Regex;
use regex::RegexBuilder;
use rquest::Response;
use rquest::header::COOKIE;
use rquest::header::ORIGIN;
use rquest::header::REFERER;
use tokio::{
    spawn,
    time::{Duration, timeout},
};
use tracing::debug;

use crate::SUPER_CLIENT;
use crate::config::UselessCookie;
use crate::config::UselessReason;
use crate::error::ClewdrError;
use crate::utils::header_ref;
use crate::{completion::Message, config::Config, utils::ENDPOINT};

#[derive(Default)]
pub struct InnerState {
    pub config: RwLock<Config>,
    pub model_list: RwLock<Vec<String>>,
    pub is_pro: RwLock<Option<String>>,
    pub cookie_model: RwLock<Option<String>>,
    pub uuid_org: RwLock<String>,
    pub changing: RwLock<bool>,
    pub change_flag: RwLock<usize>,
    pub current_index: RwLock<usize>,
    pub first_login: RwLock<bool>,
    pub timestamp: RwLock<i64>,
    pub change_times: RwLock<usize>,
    pub total_times: usize,
    pub model: RwLock<Option<String>>,
    pub cookies: RwLock<HashMap<String, String>>,
    pub uuid_org_array: RwLock<Vec<String>>,
    pub conv_uuid: RwLock<Option<String>>,
    pub conv_char: RwLock<Option<String>>,
    pub conv_depth: RwLock<i64>,
    pub prev_messages: RwLock<Vec<Message>>,
    pub prev_impersonated: RwLock<bool>,
    pub regex_log: RwLock<String>,
}

#[derive(Clone)]
pub struct AppState(pub Arc<InnerState>);

impl AppState {
    pub fn new(config: Config) -> Self {
        let total_times = config.cookie_array.len();
        let m = InnerState {
            config: RwLock::new(config),
            first_login: RwLock::new(true),
            total_times,
            ..Default::default()
        };
        let m = Arc::new(m);
        AppState(m)
    }

    pub fn update_cookie_from_res(&self, res: &Response) {
        if let Some(s) = res
            .headers()
            .get("set-cookie")
            .and_then(|h| h.to_str().ok())
        {
            self.update_cookies(s)
        }
    }

    pub fn update_cookies(&self, str: &str) {
        let str = str.split("\n").to_owned().collect::<Vec<_>>().join("");
        if str.is_empty() {
            return;
        }
        let re1 = Regex::new(r";\s?").unwrap();
        let re2 = RegexBuilder::new(r"^(path|expires|domain|HttpOnly|Secure|SameSite)[=;]*")
            .case_insensitive(true)
            .build()
            .unwrap();
        let re3 = Regex::new(r"^(.*?)=\s*(.*)").unwrap();
        re1.split(&str)
            .filter(|s| !re2.is_match(s) && !s.is_empty())
            .for_each(|s| {
                let caps = re3.captures(s);
                if let Some(caps) = caps {
                    let key = caps[1].to_string();
                    let value = caps[2].to_string();
                    let mut cookies = self.0.cookies.write();
                    cookies.insert(key, value);
                }
            });
    }

    pub fn header_cookie(&self) -> String {
        let cookies = self.0.cookies.read();
        cookies
            .iter()
            .enumerate()
            .map(|(idx, (name, value))| {
                format!(
                    "{}={}{}",
                    name,
                    value,
                    if idx == cookies.len() - 1 { "" } else { ";" }
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim_end()
            .to_string()
    }

    pub fn cookie_changer(&self, reset_timer: Option<bool>, cleanup: Option<bool>) -> bool {
        let my_state = self.0.clone();
        let reset_timer = reset_timer.unwrap_or(true);
        let cleanup = cleanup.unwrap_or(false);
        let config = self.0.config.read();
        if config.cookie_array.is_empty() {
            *my_state.changing.write() = false;
            false
        } else {
            *my_state.change_flag.write() = 0;
            *my_state.changing.write() = true;
            if !cleanup {
                // rotate the cookie
                let mut index = my_state.current_index.write();
                let array_len = config.cookie_array.len();
                *index = (*index + 1) % array_len;
                println!("{}", "Changing cookie".green());
            }
            // set timeout callback
            let dur = if config.rproxy.is_empty() || config.rproxy == ENDPOINT {
                15000 + *my_state.timestamp.read() - chrono::Utc::now().timestamp_millis()
            } else {
                0
            };
            let dur = Duration::from_millis(dur as u64);
            let self_clone = self.clone();
            spawn(timeout(dur, async move {
                spawn(async move { self_clone.bootstrap().await });
                if reset_timer {
                    let now = chrono::Utc::now().timestamp_millis();
                    *my_state.timestamp.write() = now;
                }
            }));
            false
        }
    }

    pub async fn wait_for_change(&self) {
        // if changing is true, wait for it to be false
        let istate = self.0.clone();
        while *istate.changing.read() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub fn cookie_cleaner(&self, reason: UselessReason) -> bool {
        let mut config = self.0.config.write();
        if config.current_cookie_info().is_none() {
            return false;
        }
        let current_index = *self.0.current_index.read();
        let mut config = self.0.config.write();
        let current_cookie = config.cookie_array.remove(current_index);
        config.cookie.clear();
        config
            .wasted_cookie
            .push(UselessCookie::new(current_cookie.cookie, reason));
        config.save().unwrap_or_else(|e| {
            println!("Failed to save config: {}", e);
        });
        println!("Cleaning Cookie...");
        self.cookie_changer(Some(true), Some(true))
    }

    pub async fn delete_chat(&self, uuid: String) -> Result<(), ClewdrError> {
        if uuid.is_empty() {
            return Ok(());
        }
        let istate = self.0.clone();
        let conv_uuid = istate.conv_uuid.read().clone();
        if let Some(conv_uuid) = conv_uuid {
            if uuid == conv_uuid {
                istate.conv_uuid.write().take();
                debug!("Deleting chat: {}", uuid);
                *istate.conv_depth.write() = 0;
            }
        };
        if istate.config.read().settings.preserve_chats {
            return Ok(());
        }
        let endpoint = istate.config.read().endpoint("api/organizations");
        let uuid_org = istate.uuid_org.read().clone();
        let endpoint = format!("{}/{}/chat_conversations/{}", endpoint, uuid_org, uuid);
        let cookies = self.header_cookie();
        let res = SUPER_CLIENT
            .delete(endpoint.clone())
            .header_append(ORIGIN, ENDPOINT)
            .header_append(REFERER, header_ref(""))
            .header_append(COOKIE, cookies)
            .send()
            .await?;
        self.update_cookie_from_res(&res);
        Ok(())
    }
}
