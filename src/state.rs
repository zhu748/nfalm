use colored::Colorize;
use regex::RegexBuilder;
use rquest::Proxy;
use rquest::Response;
use rquest::header::SET_COOKIE;
use serde_json::json;
use tracing::debug;
use tracing::error;

use std::collections::HashMap;

use crate::client::SUPER_CLIENT;
use crate::client::SetupRequest;
use crate::config::CLEWDR_CONFIG;
use crate::config::CookieStatus;
use crate::config::Reason;
use crate::error::ClewdrError;
use crate::services::cookie_manager::CookieEventSender;

/// State of current connection
#[derive(Clone)]
pub struct ClientState {
    pub cookie: Option<CookieStatus>,
    pub event_sender: CookieEventSender,
    pub org_uuid: Option<String>,
    pub conv_uuid: Option<String>,
    cookies: HashMap<String, String>,
    pub capabilities: Vec<String>,
    pub endpoint: String,
    pub proxy: Option<Proxy>,
}

impl ClientState {
    /// Create a new AppState instance
    pub fn new(event_sender: CookieEventSender) -> Self {
        ClientState {
            event_sender,
            cookie: None,
            org_uuid: None,
            conv_uuid: None,
            cookies: HashMap::new(),
            capabilities: Vec::new(),
            endpoint: CLEWDR_CONFIG.load().endpoint(),
            proxy: CLEWDR_CONFIG.load().rquest_proxy.clone(),
        }
    }

    /// Checks if the current user has pro capabilities
    /// Returns true if any capability contains "pro", "enterprise", "raven", or "max"
    pub fn is_pro(&self) -> bool {
        self.capabilities.iter().any(|c| {
            c.contains("pro")
                || c.contains("enterprise")
                || c.contains("raven")
                || c.contains("max")
        })
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
        let re = RegexBuilder::new(r"^(path|expires|domain|HttpOnly|Secure|SameSite)[=;]*")
            .case_insensitive(true)
            .build()
            .unwrap();
        str.split(";")
            .filter(|s| !re.is_match(s) && !s.is_empty())
            .for_each(|s| {
                let Some((name, value)) = s.split_once("=").map(|(n, v)| (n.trim(), v.trim()))
                else {
                    return;
                };
                if name.is_empty() || value.is_empty() {
                    return;
                }
                self.cookies.insert(name.to_string(), value.to_string());
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

    /// Requests a new cookie from the cookie manager
    /// Updates the internal state with the new cookie and proxy configuration
    pub async fn request_cookie(&mut self) -> Result<(), ClewdrError> {
        let res = self.event_sender.request().await?;
        self.cookie = Some(res.clone());
        self.update_cookies(res.cookie.to_string().as_str());
        // load newest config
        self.proxy = CLEWDR_CONFIG.load().rquest_proxy.clone();
        self.endpoint = CLEWDR_CONFIG.load().endpoint();
        println!("Cookie: {}", res.cookie.to_string().green());
        Ok(())
    }

    /// Returns the current cookie to the cookie manager
    /// Optionally provides a reason for returning the cookie (e.g., invalid, banned)
    pub async fn return_cookie(&mut self, reason: Option<Reason>) {
        // return the cookie to the cookie manager
        if let Some(cookie) = self.cookie.take() {
            self.event_sender
                .return_cookie(cookie, reason)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send cookie: {}", e);
                });
        }
    }

    /// Deletes or renames the current chat conversation based on configuration
    /// If preserve_chats is true, the chat is renamed rather than deleted
    pub async fn clean_chat(&self) -> Result<(), ClewdrError> {
        let Some(ref org_uuid) = self.org_uuid else {
            return Ok(());
        };
        let Some(ref conv_uuid) = self.conv_uuid else {
            return Ok(());
        };
        // if preserve_chats is true, do not delete chat, just rename it
        if CLEWDR_CONFIG.load().preserve_chats {
            debug!("Renaming chat: {}", conv_uuid);
            let endpoint = format!(
                "{}/api/organizations/{}/chat_conversations/{}",
                self.endpoint, org_uuid, conv_uuid
            );
            let pld = json!({
                "name": format!("ClewdR-{}-{}", org_uuid, conv_uuid),
            });
            let _ = SUPER_CLIENT
                .put(endpoint)
                .setup_request(conv_uuid, self.header_cookie(), self.proxy.clone())
                .json(&pld)
                .send()
                .await?;
            return Ok(());
        }
        debug!("Deleting chat: {}", conv_uuid);
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}",
            self.endpoint, org_uuid, conv_uuid
        );
        let _ = SUPER_CLIENT
            .delete(endpoint)
            .setup_request(conv_uuid, self.header_cookie(), self.proxy.clone())
            .send()
            .await?;
        Ok(())
    }
}
