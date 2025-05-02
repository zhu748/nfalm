use regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
};
use tracing::{info, warn};

use crate::config::PLACEHOLDER_COOKIE;

/// A struct representing a cookie with its information
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CookieStatus {
    pub cookie: ClewdrCookie,
    #[serde(default)]
    pub reset_time: Option<i64>,
}

impl PartialEq for CookieStatus {
    fn eq(&self, other: &Self) -> bool {
        self.cookie == other.cookie
    }
}

impl Eq for CookieStatus {}

impl Hash for CookieStatus {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cookie.hash(state);
    }
}

impl Ord for CookieStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cookie.cmp(&other.cookie)
    }
}

impl PartialOrd for CookieStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl CookieStatus {
    /// Creates a new CookieStatus instance
    ///
    /// # Arguments
    /// * `cookie` - Cookie string
    /// * `reset_time` - Optional timestamp when the cookie can be reused
    ///
    /// # Returns
    /// A new CookieStatus instance
    pub fn new(cookie: &str, reset_time: Option<i64>) -> Self {
        Self {
            cookie: ClewdrCookie::from(cookie),
            reset_time,
        }
    }

    /// Checks if the cookie's reset time has expired
    /// If the reset time has passed, sets it to None so the cookie becomes valid again
    ///
    /// # Returns
    /// The same CookieStatus with potentially updated reset_time
    pub fn reset(self) -> Self {
        if let Some(t) = self.reset_time {
            if t < chrono::Utc::now().timestamp() {
                info!("Cookie reset time expired");
                return Self {
                    reset_time: None,
                    ..self
                };
            }
        }
        self
    }
}

/// A struct representing a cookie
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClewdrCookie {
    inner: String,
}

impl Deref for ClewdrCookie {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for ClewdrCookie {
    fn default() -> Self {
        Self {
            inner: PLACEHOLDER_COOKIE.to_string(),
        }
    }
}

impl ClewdrCookie {
    /// Checks if the cookie has a valid format
    /// Validates the cookie string against the expected pattern
    ///
    /// # Returns
    /// * `bool` - True if the cookie has a valid format, false otherwise
    pub fn validate(&self) -> bool {
        // Check if the cookie is valid
        let re = regex::Regex::new(r"^[0-9A-Za-z_-]{86}-[0-9A-Za-z_-]{6}AA$").unwrap();
        re.is_match(&self.inner)
    }

    pub fn ellipse(&self) -> String {
        let len = self.inner.len();
        if len > 10 {
            format!("{}...", &self.inner[..10])
        } else {
            self.inner.to_owned()
        }
    }
}

impl From<&str> for ClewdrCookie {
    /// Create a new cookie from a string
    fn from(original: &str) -> Self {
        // split off first '@' to keep compatibility with clewd
        let cookie = original.split_once('@').map_or(original, |(_, c)| c);
        // only keep '=' '_' '-' and alphanumeric characters
        let cookie = cookie
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '=' || *c == '_' || *c == '-')
            .collect::<String>()
            .trim_start_matches("sessionKey=")
            .trim_start_matches("sk-ant-sid01-")
            .to_string();
        let cookie = Self { inner: cookie };
        if !cookie.validate() {
            warn!("Invalid cookie format: {}", original);
        }
        cookie
    }
}

impl Display for ClewdrCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sessionKey=sk-ant-sid01-{}", self.inner)
    }
}

impl Debug for ClewdrCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sessionKey=sk-ant-sid01-{}", self.inner)
    }
}

impl Serialize for ClewdrCookie {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = self.to_string();
        serializer.serialize_str(&str)
    }
}

impl<'de> Deserialize<'de> for ClewdrCookie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ClewdrCookie::from(s.as_str()))
    }
}
