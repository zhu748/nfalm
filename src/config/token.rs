use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{DurationSeconds, TimestampSecondsWithFrac, serde_as};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Account {
    pub email_address: String,
    pub uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]

pub struct Organization {
    pub name: String,
    pub uuid: String,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenInfoRaw {
    pub access_token: String,
    pub account: Account,
    #[serde_as(as = "DurationSeconds")]
    pub expires_in: Duration,
    pub organization: Organization,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenInfo {
    pub access_token: String,
    pub account: Account,
    #[serde_as(as = "DurationSeconds")]
    pub expires_in: Duration,
    pub organization: Organization,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    #[serde_as(as = "TimestampSecondsWithFrac")]
    pub expires_at: DateTime<Utc>,
}

impl TokenInfo {
    pub fn new(raw: TokenInfoRaw) -> Self {
        let expires_at = Utc::now() + raw.expires_in;
        Self {
            access_token: raw.access_token,
            account: raw.account,
            expires_in: raw.expires_in,
            organization: raw.organization,
            refresh_token: raw.refresh_token,
            scope: raw.scope,
            token_type: raw.token_type,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        debug!("Expires at: {}", self.expires_at.to_rfc3339());
        Utc::now() >= self.expires_at - Duration::from_secs(60 * 5) // 5 minutes
    }
}
