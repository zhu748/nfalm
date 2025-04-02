use clap::Parser;
use claude_tokenizer::tokenize;
use colored::Colorize;
use rand::{Rng, rng};
use regex::Regex;
use rquest::Proxy;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use tracing::{debug, error, info, warn};

use crate::{Args, error::ClewdrError, utils::config_dir};

pub const CONFIG_NAME: &str = "config.toml";
pub const ENDPOINT: &str = "https://api.claude.ai";

/// A struct representing the configuration of the application
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    // Cookie configurations
    #[serde(default)]
    cookie_array: Vec<CookieInfo>,
    pub wasted_cookie: Vec<UselessCookie>,
    pub max_cons_requests: u64,
    pub wait_time: u64,

    // Network settings
    cookie_index: usize,
    password: String,
    pub proxy: String,
    ip: String,
    port: u16,
    pub local_tunnel: bool,

    // Proxy configurations
    pub rproxy: String,

    // Prompt configurations
    pub use_real_roles: bool,
    pub custom_h: Option<String>,
    pub custom_a: Option<String>,
    pub custom_prompt: String,
    pub padtxt_file: String,
    pub padtxt_len: usize,

    // Nested settings section
    #[serde(default)]
    pub settings: Settings,

    // Skip field
    #[serde(skip)]
    pub rquest_proxy: Option<Proxy>,
    #[serde(skip)]
    pub pad_tokens: Vec<String>,
}

/// Reason why a cookie is considered useless
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum UselessReason {
    Null,
    Disabled,
    Unverified,
    Overlap,
    Banned,
    Invalid,
    Exhausted(i64),
    CoolDown,
}

impl Display for UselessReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UselessReason::Null => write!(f, "Null"),
            UselessReason::Disabled => write!(f, "Disabled"),
            UselessReason::Unverified => write!(f, "Unverified"),
            UselessReason::Overlap => write!(f, "Overlap"),
            UselessReason::Banned => write!(f, "Banned"),
            UselessReason::Invalid => write!(f, "Invalid"),
            UselessReason::Exhausted(i) => write!(f, "Temporarily Exhausted: {}", i),
            UselessReason::CoolDown => write!(f, "CoolDown"),
        }
    }
}

/// A struct representing a useless cookie
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UselessCookie {
    pub cookie: Cookie,
    pub reason: UselessReason,
}

impl UselessCookie {
    pub fn new(cookie: Cookie, reason: UselessReason) -> Self {
        Self { cookie, reason }
    }
}

/// A struct representing a cookie with its information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CookieInfo {
    pub cookie: Cookie,
    pub model: Option<String>,
    #[serde(deserialize_with = "validate_reset")]
    #[serde(default)]
    pub reset_time: Option<i64>,
}

/// Additional settings, ported from clewd, may be merged into config in the future
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub pass_params: bool,
    pub preserve_chats: bool,
    pub skip_restricted: bool,
}

/// Default cookie value for testing purposes
const PLACEHOLDER_COOKIE: &str = "sk-ant-sid01----------------------------SET_YOUR_COOKIE_HERE----------------------------------------AAAAAAAA";

/// Function to validate the reset time of a cookie while deserializing
fn validate_reset<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // skip no deserializable value
    let Ok(value) = Option::<i64>::deserialize(deserializer) else {
        return Ok(None);
    };
    // skip empty value
    let Some(v) = value else {
        return Ok(None);
    };
    // parse timestamp
    let Some(time) = chrono::DateTime::from_timestamp(v, 0) else {
        warn!("Invalid reset time: {}", v);
        return Ok(None);
    };
    let now = chrono::Utc::now();
    if time < now {
        // cookie have reset
        info!("Cookie reset time is in the past: {}", time);
        return Ok(None);
    }
    let remaining_time = time - now;
    info!("Cookie reset in {} hours", remaining_time.num_hours());
    Ok(Some(v))
}

impl CookieInfo {
    pub fn new(cookie: &str, model: Option<&str>, reset_time: Option<i64>) -> Self {
        Self {
            cookie: Cookie::from(cookie),
            model: model.map(|m| m.to_string()),
            reset_time,
        }
    }

    /// Check if the cookie is a pro cookie
    pub fn is_pro(&self) -> bool {
        self.model
            .as_ref()
            .is_some_and(|model| model.contains("claude") && model.contains("_pro"))
    }

    /// Check if cookie is usable. Besides, reset the cookie if it is expired
    pub fn check_timer(&mut self) -> bool {
        if let Some(reset_time) = self.reset_time {
            let now = chrono::Utc::now();
            if reset_time < now.timestamp() {
                self.reset_time = None;
                return true;
            }
            return false;
        }
        true
    }
}

/// A struct representing a cookie
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cookie {
    inner: String,
}

impl Cookie {
    /// Check if the cookie is valid format
    pub fn validate(&self) -> bool {
        // Check if the cookie is valid
        let re = regex::Regex::new(r"sk-ant-sid01-[0-9A-Za-z_-]{86}-[0-9A-Za-z_-]{6}AA").unwrap();
        re.is_match(&self.inner)
    }

    pub fn clear(&mut self) {
        // Clear the cookie
        self.inner.clear();
    }
}

impl From<&str> for Cookie {
    /// Create a new cookie from a string
    fn from(cookie: &str) -> Self {
        // split off first '@' to keep compatibility with clewd
        let cookie = cookie.split_once('@').map_or(cookie, |(_, c)| c);
        // only keep '=' '_' '-' and alphanumeric characters
        let cookie = cookie
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '=' || *c == '_' || *c == '-')
            .collect::<String>()
            .trim_start_matches("sessionKey=")
            .to_string();
        let cookie = Self { inner: cookie };
        if !cookie.validate() {
            warn!("Invalid cookie format: {}", cookie);
        }
        cookie
    }
}

impl Display for Cookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sessionKey={}", self.inner)
    }
}

impl Debug for Cookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sessionKey={}", self.inner)
    }
}

impl Serialize for Cookie {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = self.to_string();
        serializer.serialize_str(&str)
    }
}

impl<'de> Deserialize<'de> for Cookie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Cookie::from(s.as_str()))
    }
}

/// Generate a random password of given length
fn generate_password(length: usize) -> String {
    println!(
        "{}",
        "Generating random password, paste it to your proxy setting in SillyTavern".green()
    );
    let mut rng = rng();
    (0..length)
        .map(|_| rng.random_range(33..=126) as u8 as char) // 33–126 inclusive
        .collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cookie_array: vec![
                CookieInfo::new(PLACEHOLDER_COOKIE, None, None),
                CookieInfo::new(PLACEHOLDER_COOKIE, Some("claude_pro"), None),
            ],
            max_cons_requests: 3,
            wait_time: 15,
            wasted_cookie: Vec::new(),
            cookie_index: 0,
            password: String::new(),
            proxy: String::new(),
            ip: "127.0.0.1".to_string(),
            port: 8484,
            local_tunnel: false,
            rproxy: String::new(),
            settings: Settings::default(),
            use_real_roles: false,
            custom_prompt: String::new(),
            padtxt_file: String::new(),
            padtxt_len: 0,
            custom_h: None,
            custom_a: None,
            rquest_proxy: None,
            pad_tokens: Vec::new(),
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // one line per field
        write!(
            f,
            "Password: {}\n\
            Cookie index: {}\n\
            Forward Proxy: {}\n\
            Reverse Proxy: {}\n\
            Available Cookies in array: {}\n",
            self.password.yellow(),
            self.cookie_index.to_string().blue(),
            self.proxy.to_string().blue(),
            self.rproxy.to_string().blue(),
            self.cookie_array
                .iter()
                .filter(|x| x.reset_time.is_none())
                .count()
                .to_string()
                .blue()
        )?;
        if !self.pad_tokens.is_empty() {
            Ok(writeln!(
                f,
                "Pad txt token count: {}",
                self.pad_tokens.len().to_string().blue()
            )?)
        } else {
            Ok(())
        }
    }
}

impl Config {
    pub fn auth(&self, key: &str) -> bool {
        if key == self.password {
            return true;
        } else {
            warn!("Invalid password");
            return false;
        }
    }

    /// Load the configuration from the file
    pub fn load() -> Result<Self, ClewdrError> {
        // try to read from pwd
        let file_string = std::fs::read_to_string(CONFIG_NAME).or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                // try to read from exec path
                let exec_path = std::env::current_exe()?;
                let config_dir = exec_path.parent().ok_or(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Failed to get parent directory",
                ))?;
                let config_path = config_dir.join(CONFIG_NAME);
                std::fs::read_to_string(config_path)
            } else {
                Err(e)
            }
        });
        match file_string {
            Ok(file_string) => {
                // parse the config file
                let mut config: Config = toml::de::from_str(&file_string)?;
                config.load_from_arg_file();
                config.load_padtxt();
                config = config.validate();
                // load command line arguments
                let args = Args::parse();
                if let Some(index) = args.index {
                    if index < config.cookie_array.len() {
                        info!("Setting cookie index to {}", index);
                        config.cookie_index = index;
                    } else {
                        warn!("Index out of range");
                    }
                }
                config.save()?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // create a default config file
                let exec_path = std::env::current_exe()?;
                let config_dir = exec_path.parent().ok_or(ClewdrError::PathNotFound(
                    "Failed to get parent directory".to_string(),
                ))?;
                let mut default_config = Config::default();
                let canonical_path = std::fs::canonicalize(config_dir)?;
                println!(
                    "Default config file created at {}/config.toml",
                    canonical_path.display()
                );
                println!("{}", "SET YOUR COOKIE HERE".green());
                default_config.load_from_arg_file();
                default_config = default_config.validate();
                default_config.save()?;
                Ok(default_config)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn load_padtxt(&mut self) {
        let padtxt = self.padtxt_file.clone();
        if padtxt.trim().is_empty() {
            return;
        }

        let Ok(dir) = config_dir() else {
            error!("No config found in cwd or exec dir");
            return;
        };
        let padtxt_path = dir.join(padtxt);
        if !padtxt_path.exists() {
            error!("Pad txt file not found: {}", padtxt_path.display());
            return;
        }
        let Ok(padtxt_string) = std::fs::read_to_string(padtxt_path.clone()) else {
            error!("Failed to read pad txt file: {}", padtxt_path.display());
            return;
        };
        // remove tokenizer special characters
        let re = Regex::new(r"[^\x00-\x7F]").unwrap();
        let tokens = tokenize(&padtxt_string)
            .expect("Failed to tokenize pad txt")
            .into_iter()
            // remove special characters
            .map(|t| re.replace_all(t.1.as_str(), "").trim().to_string())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();
        if tokens.len() < 4096 {
            panic!("Pad txt file is too short: {}", padtxt_path.display());
        }
        self.pad_tokens = tokens;
    }

    /// Clean current cookie and add it to the wasted cookie list
    pub fn cookie_cleaner(&mut self, reason: UselessReason) {
        if let UselessReason::Exhausted(_) = reason {
            debug!("Temporary useless cookie, not cleaning");
            return;
        }
        let Some(current_cookie) = self.delete_current_cookie() else {
            warn!("No current cookie found");
            return;
        };
        self.wasted_cookie
            .push(UselessCookie::new(current_cookie.cookie, reason));
        self.save().unwrap_or_else(|e| {
            error!("Failed to save config: {}", e);
        });
        debug!("Cleaning Cookie...");
    }

    /// API endpoint of server
    pub fn endpoint(&self) -> String {
        if self.rproxy.is_empty() {
            ENDPOINT.to_string()
        } else {
            self.rproxy.clone()
        }
    }

    /// address of proxy
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    /// Save the configuration to a file
    pub fn save(&self) -> Result<(), ClewdrError> {
        // try find existing config file
        let existing = config_dir();
        if let Ok(existing) = existing {
            let config_path = existing.join(CONFIG_NAME);
            // overwrite the file if it exists
            std::fs::write(config_path, toml::ser::to_string_pretty(self)?)?;
            return Ok(());
        }
        // try to create a new config file in exec path or pwd
        let exec_path = std::env::current_exe()?;
        let config_dir = exec_path.parent().ok_or(ClewdrError::PathNotFound(
            "Failed to get parent directory".to_string(),
        ))?;
        // create the config directory if it doesn't exist
        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }
        // Save the config to a file
        let config_path = config_dir.join(CONFIG_NAME);
        let config_string = toml::ser::to_string_pretty(self)?;
        std::fs::write(config_path, config_string)?;
        Ok(())
    }

    /// Get current cookie info
    pub fn current_cookie_info(&mut self) -> Option<&mut CookieInfo> {
        if self.cookie_index < self.cookie_array.len() {
            Some(&mut self.cookie_array[self.cookie_index as usize])
        } else {
            None
        }
    }

    /// Get current cookie index
    pub fn index(&self) -> usize {
        self.cookie_index
    }

    /// Remove the current cookie from the array
    /// and return it, also change index
    fn delete_current_cookie(&mut self) -> Option<CookieInfo> {
        if self.cookie_index < self.cookie_array.len() {
            let index = self.cookie_index;
            let removed = self.cookie_array.remove(index);
            if index == self.cookie_array.len() {
                self.cookie_index = 0;
            }
            warn!("Removed cookie: {}", removed.cookie.to_string().red());
            return Some(removed);
        }
        None
    }

    /// length of cookie array
    pub fn cookie_array_len(&self) -> usize {
        self.cookie_array.len()
    }

    /// Rotate the cookie index to the next usable cookie
    pub fn rotate_cookie(&mut self) {
        if self.cookie_array.is_empty() {
            return;
        }
        let array_len = self.cookie_array.len();
        let mut index = self.cookie_index;
        index = (index + 1) % array_len;
        while let Some(cookie) = self.cookie_array.get_mut(index as usize) {
            debug!("Checking cookie in {}", index);
            if index == self.cookie_index {
                // Terminate if all cookies are useless
                error!("All cookies are useless");
                return;
            }
            // Check if the cookie is usable
            if cookie.check_timer() {
                break;
            }
            index = (index + 1) % array_len;
        }
        self.cookie_index = index;
        self.save().unwrap_or_else(|e| {
            error!("Failed to save config: {}", e);
        });
    }

    /// Validate the configuration
    fn validate(mut self) -> Self {
        if self.password.trim().is_empty() {
            self.password = generate_password(32);
            self.save().expect("Failed to save config");
        }
        if !self.cookie_array.is_empty() && self.cookie_index >= self.cookie_array.len() {
            self.cookie_index = rng().random_range(0..self.cookie_array.len());
        }
        self.ip = self.ip.trim().to_string();
        self.rproxy = self.rproxy.trim().to_string();
        self.proxy = self.proxy.trim().to_string();
        let proxy = if self.proxy.is_empty() {
            None
        } else {
            Proxy::all(self.proxy.clone())
                .inspect_err(|e| {
                    error!("Failed to parse proxy: {}", e);
                })
                .ok()
        };
        self.rquest_proxy = proxy;
        self
    }

    /// Load cookies from command line arguments
    fn load_from_arg_file(&mut self) {
        let args: Args = clap::Parser::parse();
        let file = args.cookie_file;
        let Some(file) = file else {
            return;
        };
        let Ok(file_string) = std::fs::read_to_string(file) else {
            return;
        };
        // one line per cookie
        let mut new_array = file_string
            .lines()
            .filter_map(|line| {
                let c = Cookie::from(line);
                if !c.validate() {
                    warn!("Invalid cookie format: {}", line);
                    return None;
                }
                if self.cookie_array.iter().any(|x| x.cookie == c) {
                    warn!("Duplicate cookie: {}", line);
                    return None;
                }
                if self.wasted_cookie.iter().any(|x| x.cookie == c) {
                    warn!("Wasted cookie: {}", line);
                    return None;
                }
                Some(CookieInfo {
                    cookie: c,
                    model: None,
                    reset_time: None,
                })
            })
            .collect::<Vec<_>>();
        // remove duplicates
        new_array.sort_unstable_by(|a, b| a.cookie.cmp(&b.cookie));
        new_array.dedup_by(|a, b| a.cookie == b.cookie);
        self.cookie_array.extend(new_array);
    }
}
