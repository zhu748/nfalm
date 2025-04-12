use colored::Colorize;
use regex::Regex;
use regex::RegexBuilder;
use rquest::Response;
use rquest::header::SET_COOKIE;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::error;

use std::collections::HashMap;
use std::sync::Arc;

use crate::client::SUPER_CLIENT;
use crate::client::SetupRequest;
use crate::config::Config;
use crate::config::CookieStatus;
use crate::config::Reason;
use crate::error::ClewdrError;

/// State of current connection
#[derive(Clone)]
pub struct AppState {
    pub req_tx: Sender<oneshot::Sender<Result<CookieStatus, ClewdrError>>>,
    pub ret_tx: Sender<(CookieStatus, Option<Reason>)>,
    pub submit_tx: Sender<CookieStatus>,
    pub cookie: Option<CookieStatus>,
    pub config: Arc<Config>,
    pub org_uuid: Option<String>,
    pub conv_uuid: Option<String>,
    cookies: HashMap<String, String>,
}

impl AppState {
    /// Create a new AppState instance
    pub fn new(
        config: Config,
        req_tx: Sender<oneshot::Sender<Result<CookieStatus, ClewdrError>>>,
        ret_tx: Sender<(CookieStatus, Option<Reason>)>,
        submit_tx: Sender<CookieStatus>,
    ) -> Self {
        AppState {
            config: Arc::new(config),
            req_tx,
            ret_tx,
            submit_tx,
            cookie: None,
            org_uuid: None,
            conv_uuid: None,
            cookies: HashMap::new(),
        }
    }

    /// Update cookie from the server response
    pub fn update_cookie_from_res(&mut self, res: &Response) {
        if let Some(s) = res.headers().get(SET_COOKIE).and_then(|h| h.to_str().ok()) {
            self.update_cookies(s)
        }
    }

    /// Update cookies from string
    fn update_cookies(&mut self, str: &str) {
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
                    self.cookies.insert(key, value);
                }
            });
    }

    /// Current cookie string that are used in requests
    pub fn header_cookie(&self) -> String {
        // check rotating guard
        self.cookies
            .iter()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>()
            .join("; ")
            .trim()
            .to_string()
    }

    /// request a new cookie from cookie manager
    pub async fn request_cookie(&mut self) -> Result<(), ClewdrError> {
        // real client to avoid mixed use of cookies
        let (one_tx, one_rx) = oneshot::channel();
        self.req_tx.send(one_tx).await?;
        let res = one_rx.await??;
        self.cookie = Some(res.clone());
        self.update_cookies(res.cookie.to_string().as_str());
        println!("Cookie: {}", res.cookie.to_string().green());
        Ok(())
    }

    /// return the cookie to the cookie manager
    pub async fn return_cookie(&mut self, reason: Option<Reason>) {
        // return the cookie to the cookie manager
        if let Some(cookie) = self.cookie.take() {
            self.ret_tx
                .send((cookie, reason))
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send cookie: {}", e);
                });
        }
    }

    /// Delete current chat conversation
    pub async fn delete_chat(&self) -> Result<(), ClewdrError> {
        let Some(ref org_uuid) = self.org_uuid else {
            return Ok(());
        };
        let Some(ref conv_uuid) = self.conv_uuid else {
            return Ok(());
        };
        // if preserve_chats is true, do not delete chat
        if self.config.preserve_chats {
            return Ok(());
        }
        debug!("Deleting chat: {}", conv_uuid);
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}",
            self.config.endpoint(),
            org_uuid,
            conv_uuid
        );
        let proxy = self.config.rquest_proxy.clone();
        let _ = SUPER_CLIENT
            .delete(endpoint)
            .setup_request("", self.header_cookie(), proxy)
            .send()
            .await?;
        Ok(())
    }
}
