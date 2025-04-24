use colored::Colorize;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use itertools::Itertools;
use passwords::PasswordGenerator;
use rquest::Proxy;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use tiktoken_rs::o200k_base;
use tracing::{error, warn};

use crate::{
    config::{
        CONFIG_NAME, CookieStatus, Reason, UselessCookie, default_check_update, default_ip,
        default_max_retries, default_padtxt_len, default_port, default_use_real_roles,
    },
    error::ClewdrError,
    utils::ARG_COOKIE_FILE,
};

/// Generates a random password for authentication
/// Creates a secure 64-character password with mixed character types
///
/// # Returns
/// A random password string
fn generate_password() -> String {
    let pg = PasswordGenerator {
        length: 64,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: true,
        spaces: false,
        exclude_similar_characters: true,
        strict: true,
    };

    println!("{}", "Generating random password......".green());
    pg.generate_one().unwrap()
}

/// A struct representing the configuration of the application
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClewdrConfig {
    // Cookie configurations
    #[serde(default)]
    pub cookie_array: Vec<CookieStatus>,
    #[serde(default)]
    pub wasted_cookie: Vec<UselessCookie>,

    // Server settings, cannot hot reload
    #[serde(default = "default_ip")]
    ip: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    pub enable_oai: bool,

    // App settings, can hot reload, but meaningless
    #[serde(default = "default_check_update")]
    pub check_update: bool,
    #[serde(default)]
    pub auto_update: bool,

    // Network settings, can hot reload
    #[serde(default)]
    password: String,
    #[serde(default)]
    admin_password: String,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
    pub rproxy: Option<String>,

    // Api settings, can hot reload
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    #[serde(default)]
    pub pass_params: bool,
    #[serde(default)]
    pub preserve_chats: bool,

    // Cookie settings, can hot reload
    #[serde(default)]
    pub skip_warning: bool,
    #[serde(default)]
    pub skip_restricted: bool,
    #[serde(default)]
    pub skip_non_pro: bool,

    // Prompt configurations, can hot reload
    #[serde(default = "default_use_real_roles")]
    pub use_real_roles: bool,
    #[serde(default)]
    pub custom_h: Option<String>,
    #[serde(default)]
    pub custom_a: Option<String>,
    #[serde(default)]
    pub custom_prompt: String,
    #[serde(default)]
    pub padtxt_file: Option<String>,
    #[serde(default = "default_padtxt_len")]
    pub padtxt_len: usize,

    // Skip field, can hot reload
    #[serde(skip)]
    pub rquest_proxy: Option<Proxy>,
    #[serde(skip)]
    pub pad_tokens: Arc<Vec<String>>,
}

impl Default for ClewdrConfig {
    fn default() -> Self {
        Self {
            enable_oai: false,
            max_retries: default_max_retries(),
            check_update: default_check_update(),
            auto_update: false,
            cookie_array: vec![],
            wasted_cookie: Vec::new(),
            password: String::new(),
            admin_password: String::new(),
            proxy: None,
            ip: default_ip(),
            port: default_port(),
            rproxy: None,
            use_real_roles: default_use_real_roles(),
            custom_prompt: String::new(),
            padtxt_file: None,
            padtxt_len: default_padtxt_len(),
            custom_h: None,
            custom_a: None,
            rquest_proxy: None,
            pad_tokens: Arc::new(vec![]),
            pass_params: false,
            preserve_chats: false,
            skip_warning: false,
            skip_restricted: false,
            skip_non_pro: false,
        }
    }
}

impl Display for ClewdrConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // one line per field
        let api_addr = format!("http://{}/v1", self.address());
        let web_addr = format!("http://{}", self.address());
        write!(
            f,
            "LLM API Endpoint: {}\n\
            LLM API Password: {}\n\
            Web Admin Endpoint: {}\n\
            Web Admin Password: {}\n",
            api_addr.green().underline(),
            self.password.yellow(),
            web_addr.green().underline(),
            self.admin_password.yellow(),
        )?;
        if let Some(ref proxy) = self.proxy {
            writeln!(f, "Proxy: {}", proxy.blue())?;
        }
        if let Some(ref rproxy) = self.rproxy {
            writeln!(f, "Reverse Proxy: {}", rproxy.blue())?;
        }
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

impl ClewdrConfig {
    pub fn v1_auth(&self, key: &str) -> bool {
        key == self.password
    }

    pub fn admin_auth(&self, key: &str) -> bool {
        key == self.admin_password
    }

    /// Loads configuration from files and environment variables
    /// Combines settings from config.toml, clewdr.toml, and environment variables
    /// Also loads cookies from a file if specified
    ///
    /// # Returns
    /// * `Result<Self, ClewdrError>` - Config instance or error
    pub fn new() -> Result<Self, ClewdrError> {
        let mut config: ClewdrConfig = Figment::new()
            .adjoin(Toml::file("config.toml"))
            .adjoin(Toml::file(CONFIG_NAME))
            .admerge(Env::prefixed("CLEWDR_"))
            .extract_lossy()
            .inspect_err(|e| {
                error!("Failed to load config: {}", e);
            })?;
        if let Some(ref f) = *ARG_COOKIE_FILE {
            // load cookies from file
            if f.exists() {
                let Ok(cookies) = std::fs::read_to_string(f) else {
                    error!("Failed to read cookie file: {}", f.display());
                    return Err(ClewdrError::InvalidCookie(Reason::Null));
                };
                let cookies = cookies.lines().map(|line| line.into()).map_while(
                    |c: crate::config::ClewdrCookie| {
                        if c.validate() {
                            Some(CookieStatus::new(c.to_string().as_str(), None))
                        } else {
                            warn!("Invalid cookie format: {}", c);
                            None
                        }
                    },
                );
                config.cookie_array.extend(cookies);
            } else {
                error!("Cookie file not found: {}", f.display());
            }
        }
        let config = config.validate();
        config.save().inspect_err(|e| {
            error!("Failed to save config: {}", e);
        })?;
        Ok(config)
    }

    /// Loads padding text from a file
    /// Used to pad prompts with tokens to reach minimum token requirements
    ///
    /// # Effects
    /// Updates the pad_tokens field with tokenized content from the file
    fn load_padtxt(&mut self) {
        let Some(padtxt) = &self.padtxt_file else {
            self.pad_tokens = Arc::new(vec![]);
            return;
        };
        let padtxt = padtxt.trim();
        if padtxt.is_empty() {
            self.pad_tokens = Arc::new(vec![]);
            return;
        }
        let Ok(padtxt_path) = PathBuf::from_str(padtxt);
        if !padtxt_path.exists() {
            error!("Pad txt file not found: {}", padtxt_path.display());
            self.pad_tokens = Arc::new(vec![]);
            return;
        }
        let Ok(padtxt_string) = std::fs::read_to_string(padtxt_path.as_path()) else {
            error!("Failed to read pad txt file: {}", padtxt_path.display());
            self.pad_tokens = Arc::new(vec![]);
            return;
        };
        // remove tokenizer special characters

        let bpe = o200k_base().unwrap();
        let ranks = bpe.encode_with_special_tokens(&padtxt_string);
        let mut tokens = Vec::with_capacity(4096);
        for token in ranks {
            let Ok(token) = bpe.decode(vec![token]) else {
                continue;
            };
            tokens.push(token);
        }
        if tokens.len() < 4096 {
            panic!("Pad txt file is too short: {}", padtxt_path.display());
        }
        self.pad_tokens = Arc::new(tokens);
    }

    /// Gets the API endpoint for the Claude service
    /// Returns the reverse proxy URL if configured, otherwise the default endpoint
    ///
    /// # Returns
    /// The URL string for the API endpoint
    pub fn endpoint(&self) -> String {
        if let Some(ref proxy) = self.rproxy {
            return proxy.clone();
        }
        crate::config::ENDPOINT.to_string()
    }

    /// address of proxy
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    /// Save the configuration to a file
    pub fn save(&self) -> Result<(), ClewdrError> {
        let Ok(config_path) = PathBuf::from_str(CONFIG_NAME);
        std::fs::write(config_path, toml::ser::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(mut self) -> Self {
        if self.password.trim().is_empty() {
            self.password = generate_password();
            self.save().expect("Failed to save config");
        }
        if self.admin_password.trim().is_empty() {
            self.admin_password = generate_password();
            self.save().expect("Failed to save config");
        }
        self.cookie_array = self
            .cookie_array
            .into_iter()
            .map(|x| x.reset())
            .sorted()
            .collect();
        self.cookie_array.dedup();
        self.ip = self.ip.trim().to_string();
        if self.rproxy == Some("".to_string()) {
            self.rproxy = None;
        }
        if self.proxy == Some("".to_string()) {
            self.proxy = None;
        }
        if self.padtxt_file == Some("".to_string()) {
            self.padtxt_file = None;
        }
        let proxy = self.proxy.as_ref().and_then(|p| {
            Proxy::all(p)
                .inspect_err(|e| {
                    error!("Failed to parse proxy: {}", e);
                })
                .ok()
        });
        self.rquest_proxy = proxy;
        self.load_padtxt();
        self
    }
}
