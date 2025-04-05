use colored::Colorize;
use serde_json::Value;
use tracing::{error, warn};

use crate::{
    client::AppendHeaders,
    config::Reason,
    error::{ClewdrError, check_res_err},
    state::AppState,
    utils::print_out_json,
};

impl AppState {
    /// Bootstrap the app state
    /// This function will send a request to the server to get the bootstrap data
    /// It will also check if the cookie is valid
    pub async fn bootstrap(&mut self) -> Result<(), ClewdrError> {
        let proxy = self.config.rquest_proxy.clone();
        let end_point = format!("{}/api/bootstrap", self.config.endpoint());
        let res = self
            .client
            .get(end_point)
            .append_headers("", proxy.clone())
            .send()
            .await?;
        let res = check_res_err(res).await?;
        let bootstrap = res.json::<Value>().await?;
        print_out_json(&bootstrap, "bootstrap.json");
        if bootstrap["account"].is_null() {
            error!("Null Error, Useless Cookie");
            return Err(ClewdrError::InvalidCookie(Reason::Null));
        }
        let memberships = bootstrap["account"]["memberships"]
            .as_array()
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
        let name = boot_acc_info["name"]
            .as_str()
            .and_then(|n| n.split_once("@"))
            .map(|(n, _)| n)
            .unwrap_or_default();
        let email = bootstrap["account"]["email_address"]
            .as_str()
            .unwrap_or_default();
        let caps = boot_acc_info["capabilities"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| c.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        if !caps.contains("pro") && !caps.contains("enterprise") && self.config.skip_non_pro {
            error!("Cookie is not pro or enterprise");
            return Err(ClewdrError::InvalidCookie(Reason::NonPro));
        }
        println!(
            "Logged in \nname: {}\nmail: {}\ncapabilities: {}",
            name.blue(),
            email.blue(),
            caps.blue()
        );

        // Bootstrap complete
        let end_point = self.config.endpoint();
        let end_point = format!("{}/api/organizations", end_point);
        let res = self
            .client
            .get(end_point)
            .append_headers("", proxy)
            .send()
            .await?;
        let res = check_res_err(res).await?;
        let ret_json = res.json::<Value>().await?;
        print_out_json(&ret_json, "org.json");
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
        self.org_uuid = Some(u.to_string());
        Ok(())
    }

    /// Check if the account is restricted or banned.
    /// If the account is restricted, check if the restriction is expired.
    /// If the account is banned, return an error.
    /// If the account is not restricted or banned, return Ok.
    fn check_flags(&self, acc_info: &Value) -> Result<(), ClewdrError> {
        let Some(active_flags) = acc_info.get("active_flags").and_then(|a| a.as_array()) else {
            return Ok(());
        };
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
            return Err(ClewdrError::InvalidCookie(Reason::Banned));
        } else {
            // Check if we should skip based on flag types
            let should_skip =
                active_flags
                    .iter()
                    .filter_map(|f| f["type"].as_str())
                    .any(|flag_type| {
                        // skip flags ending with warning
                        let warning_match =
                            self.config.skip_warning && flag_type.ends_with("warning");

                        //  skip flags containing restricted
                        let restricted_match =
                            self.config.skip_restricted && flag_type.contains("restricted");

                        warning_match || restricted_match
                    });

            println!("{}", "Your account is restricted.".red());

            if should_skip && restrict_until > 0 {
                if self.config.skip_warning {
                    warn!("skip_warning is enabled, skipping...");
                }
                if self.config.skip_restricted {
                    warn!("skip_restricted is enabled, skipping...");
                }
                return Err(ClewdrError::InvalidCookie(Reason::Restricted(
                    restrict_until,
                )));
            }
        }
        Ok(())
    }
}
