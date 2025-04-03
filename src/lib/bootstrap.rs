use colored::Colorize;
use serde_json::Value;
use tracing::{error, warn};

use crate::{
    client::{AppendHeaders, SUPER_CLIENT},
    config::Reason,
    error::{ClewdrError, check_res_err},
    state::AppState,
    utils::JsBool,
};

impl AppState {
    pub async fn bootstrap(&self) {
        let res = self.try_bootstrap().await;
        match res {
            Err(ClewdrError::OtherHttpError(c, _)) if c == 401 || c == 403 => {
                // Invalid authorization
                error!("{}", "Invalid authorization, Cookie is useless");
                self.cookie_rotate(Reason::Invalid);
            }
            Err(ClewdrError::ExhaustedCookie(i)) => {
                // Cookie is exhausted
                error!("Cookie is exhausted, until: {}", i);
                self.cookie_rotate(Reason::Exhausted(i));
            }
            Err(ClewdrError::InvalidCookie) => {
                // Invalid cookie
                error!("Cookie is invalid");
                self.cookie_rotate(Reason::Invalid);
            }
            Err(ClewdrError::OtherHttpError(c, e)) => {
                error!("HTTP Error, code: {}, error: {}", c, e);
            }
            Err(e) => {
                error!("Error: {}", e);
            }
            _ => {}
        }
    }

    async fn try_bootstrap(&self) -> Result<(), ClewdrError> {
        let proxy = self.config.read().rquest_proxy.clone();
        {
            let mut config = self.config.write();
            let Some(cookie) = config.current_cookie_info().cloned() else {
                error!("No cookie found.");
                return Err(ClewdrError::InvalidCookie);
            };
            self.update_cookies(&cookie.cookie.to_string());
        }
        let end_point = format!("{}/api/bootstrap", self.config.read().endpoint());
        let res = SUPER_CLIENT
            .get(end_point.clone())
            .append_headers("", self.header_cookie()?, proxy.clone())
            .send()
            .await?;
        let res = check_res_err(res).await?;
        let bootstrap = res.json::<Value>().await?;
        if bootstrap["account"].is_null() {
            error!("Null Error, Useless Cookie");
            return Err(ClewdrError::InvalidCookie);
        }
        let memberships = bootstrap["account"]["memberships"]
            .as_array()
            .cloned()
            .ok_or(ClewdrError::UnexpectedNone)?;
        let boot_acc_info = memberships
            .iter()
            .find(|m| {
                m["organization"]["capabilities"]
                    .as_array()
                    .is_some_and(|c| c.iter().any(|c| c.as_str() == Some("chat")))
            })
            .and_then(|m| m["organization"].as_object())
            .ok_or(ClewdrError::UnexpectedNone)?;
        let mut cookie_model = None;
        if let Some(model) = bootstrap.pointer("/statsig/values/layer_configs/HPOHwBLNLQLxkj5Yn4bfSkgCQnBX28kPR7h~1BNKdVLw=/value/console_default_model_override/model")
            .and_then(|m| m.as_str())
        {
            cookie_model = Some(model.to_string());
        }
        if cookie_model.is_none() {
            if let Some(model) = bootstrap.pointer("/statsig/values/dynamic_configs/6zA9wvTedwkzjLxWy9PVe7yydI00XDQ6L5Fejjq~12o8=/value/model")
                .and_then(|m| m.as_str())
            {
                cookie_model = Some(model.to_string());
            }
        }
        let mut pro = None;
        if let Some(capabilities) = boot_acc_info["capabilities"].as_array() {
            if capabilities
                .iter()
                .any(|c| c.as_str() == Some("claude_pro"))
            {
                pro = Some("claude_pro".to_string());
            } else if capabilities.iter().any(|c| c.as_str() == Some("raven")) {
                pro = Some("claude_team_pro".to_string())
            }
        }
        *self.pro.write() = pro.clone();

        // Check if is a pro account
        {
            // drop lock by using a new scope
            let mut config = self.config.write();

            let model_name = pro.clone().or(cookie_model.clone()).unwrap_or_default();
            if let Some(current_cookie) = config.current_cookie_info() {
                if !model_name.is_empty() {
                    current_cookie.model = Some(model_name);
                    config.save().unwrap_or_else(|e| {
                        error!("Failed to save config: {}", e);
                    });
                }
            }
        }
        let config = self.config.read().clone();
        let index = format!("(Index: {}) ", config.index()).blue().to_string();
        let name = boot_acc_info
            .get("name")
            .and_then(|n| n.as_str())
            .and_then(|n| n.split_once("@"))
            .map(|(n, _)| n)
            .unwrap_or_default();
        let email = bootstrap
            .pointer("/account/email_address")
            .and_then(|e| e.as_str())
            .unwrap_or_default();
        let caps = boot_acc_info
            .get("capabilities")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{}Logged in \nname: {}\nmail: {}\ncookieModel: {}\ncapabilities: {}",
            index,
            name.blue(),
            email.blue(),
            cookie_model.unwrap_or_default().blue(),
            caps.blue()
        );
        let uuid = boot_acc_info["uuid"]
            .as_str()
            .ok_or(ClewdrError::UnexpectedNone)?;
        let uuid_included = self.uuid_org_array.read().clone();
        let uuid_included = boot_acc_info["uuid"]
            .as_str()
            .is_some_and(|uuid| uuid_included.iter().any(|u| u.as_str() == uuid));
        let api_disabled_reason = boot_acc_info.get("api_disabled_reason").js_bool();
        let api_disabled_until = boot_acc_info.get("api_disabled_until").js_bool();
        let completed_verification_at = bootstrap
            .get("account")
            .and_then(|a| a.get("completed_verification_at"))
            .js_bool();
        if (uuid_included && !config.cookie_array_len() == 0)
            || (api_disabled_reason && !api_disabled_until)
            || !completed_verification_at
        {
            let reason = if api_disabled_reason {
                Reason::Disabled
            } else if !completed_verification_at {
                Reason::Unverified
            } else {
                Reason::Overlap
            };
            error!("Cookie is useless, reason: {}", reason.to_string().red());
            return Err(ClewdrError::InvalidCookie);
        } else {
            self.uuid_org_array.write().push(uuid.to_string());
        }

        // Bootstrap complete
        let end_point = config.endpoint();
        let end_point = format!("{}/api/organizations", end_point);
        let res = SUPER_CLIENT
            .get(end_point.clone())
            .append_headers("", self.header_cookie()?, proxy.clone())
            .send()
            .await?;
        self.update_cookie_from_res(&res);
        let res = check_res_err(res).await?;
        let ret_json = res.json::<Value>().await?;
        // print bootstrap to out.json, if it exists, overwrite it
        let acc_info = ret_json
            .as_array()
            .and_then(|a| {
                a.iter().find(|v| {
                    v.get("capabilities")
                        .and_then(|c| c.as_array())
                        .is_some_and(|c| c.iter().any(|c| c.as_str() == Some("chat")))
                })
            })
            .ok_or(ClewdrError::UnexpectedNone)?;

        self.check_flags(acc_info)?;

        let u = acc_info
            .get("uuid")
            .and_then(|u| u.as_str())
            .ok_or(ClewdrError::UnexpectedNone)?;
        *self.uuid_org.write() = u.to_string();
        Ok(())
    }

    fn check_flags(&self, acc_info: &Value) -> Result<(), ClewdrError> {
        let active_flags = acc_info
            .get("active_flags")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        if active_flags.is_empty() {
            return Ok(());
        }
        let now = chrono::Utc::now();
        let mut restrict_until = 0;
        let formatted_flags = active_flags
            .iter()
            .map_while(|f| {
                let expire = f["expires_at"].as_str()?;
                let expire = chrono::DateTime::parse_from_rfc3339(expire).ok()?;
                let timestamp = expire.timestamp();
                let diff = expire.to_utc() - now;
                if diff < chrono::Duration::zero() {
                    return None;
                }
                restrict_until = timestamp.max(restrict_until);
                let r#type = f["type"].as_str()?;
                Some(format!(
                    "{}: expires in {} hours",
                    r#type.red(),
                    diff.num_hours().to_string().red()
                ))
            })
            .collect::<Vec<_>>();

        if formatted_flags.is_empty() {
            return Ok(());
        }

        let banned = active_flags
            .iter()
            .any(|f| f["type"].as_str() == Some("consumer_banned"));
        let banned_str = if banned {
            "[BANNED] ".red().to_string()
        } else {
            "".to_string()
        };

        println!("{}{}", banned_str, "Your account has warnings:".red());
        for flag in formatted_flags {
            println!("{}", flag);
        }
        if banned {
            println!(
                "{}",
                "Your account is banned, please use another account.".red()
            );
            return Err(ClewdrError::InvalidCookie);
        } else {
            // Restricted
            println!("{}", "Your account is restricted.".red());
            if self.config.read().settings.skip_restricted && restrict_until > 0 {
                warn!("skip_restricted is enabled, skipping...");
                return Err(ClewdrError::ExhaustedCookie(restrict_until));
            }
        }
        Ok(())
    }
}
