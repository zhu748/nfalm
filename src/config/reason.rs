use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

use crate::config::ClewdrCookie;

/// Reason why a cookie is considered useless
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Reason {
    NonPro,
    Disabled,
    Banned,
    Null,
    Restricted(i64),
    TooManyRequest(i64),
}

impl Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reason::Disabled => write!(f, "Organization Disabled"),
            Reason::NonPro => write!(f, "Free account"),
            Reason::Banned => write!(f, "Banned"),
            Reason::Null => write!(f, "Null"),
            Reason::Restricted(i) => {
                let time = chrono::DateTime::from_timestamp(*i, 0)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string().yellow())
                    .unwrap_or("Invalid date".to_string().yellow());
                write!(f, "Restricted: until {}", time)
            }
            Reason::TooManyRequest(i) => {
                let time = chrono::DateTime::from_timestamp(*i, 0)
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string().yellow())
                    .unwrap_or("Invalid date".to_string().yellow());
                write!(f, "429 Too many request: until {}", time)
            }
        }
    }
}

/// A struct representing a cookie that can't be used
/// Contains the cookie and the reason why it's considered unusable
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UselessCookie {
    pub cookie: ClewdrCookie,
    pub reason: Reason,
}

impl PartialEq for UselessCookie {
    fn eq(&self, other: &Self) -> bool {
        self.cookie == other.cookie
    }
}

impl Eq for UselessCookie {}

impl Hash for UselessCookie {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cookie.hash(state);
    }
}

impl UselessCookie {
    /// Creates a new UselessCookie instance
    ///
    /// # Arguments
    /// * `cookie` - The cookie that is unusable
    /// * `reason` - The reason why the cookie is unusable
    ///
    /// # Returns
    /// A new UselessCookie instance
    pub fn new(cookie: ClewdrCookie, reason: Reason) -> Self {
        Self { cookie, reason }
    }
}
