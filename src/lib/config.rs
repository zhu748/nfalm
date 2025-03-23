use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};

const CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // Cookie configurations
    pub cookie: String,
    pub cookie_array: Vec<String>,
    pub wasted_cookie: Vec<String>,
    pub unknown_models: Vec<String>,

    // Network settings
    pub cookie_counter: u32,
    pub cookie_index: u32,
    pub proxy_password: String,
    pub ip: String,
    pub port: u16,
    pub local_tunnel: bool,

    // Performance settings
    pub buffer_size: u32,
    pub system_interval: u32,

    // Proxy configurations
    pub rproxy: String,
    pub api_rproxy: String,

    // Token handling
    pub placeholder_token: String,
    pub placeholder_byte: String,

    // Prompt templates
    pub prompt_experiment_first: String,
    pub prompt_experiment_next: String,
    pub personality_format: String,
    pub scenario_format: String,

    // Nested settings section
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub renew_always: bool,
    pub retry_regenerate: bool,
    pub prompt_experiments: bool,
    pub system_experiments: bool,
    pub prevent_imperson: bool,
    pub all_samples: bool,
    pub no_samples: bool,
    pub strip_assistant: bool,
    pub strip_human: bool,
    pub pass_params: bool,
    pub clear_flags: bool,
    pub preserve_chats: bool,
    pub log_messages: bool,
    pub full_colon: bool,
    pub padtxt: String,
    pub xml_plot: bool,
    pub skip_restricted: bool,
    pub artifacts: bool,
    pub superfetch: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cookie: String::new(),
            cookie_array: Vec::new(),
            wasted_cookie: Vec::new(),
            unknown_models: Vec::new(),
            cookie_counter: 3,
            cookie_index: 0,
            proxy_password: String::new(),
            ip: "127.0.0.1".to_string(),
            port: 8444,
            local_tunnel: false,
            buffer_size: 1,
            system_interval: 3,
            rproxy: String::new(),
            api_rproxy: String::new(),
            placeholder_token: String::new(),
            placeholder_byte: String::new(),
            prompt_experiment_first: String::new(),
            prompt_experiment_next: String::new(),
            personality_format: "{{char}}'s personality: {{personality}}".to_string(),
            scenario_format: "Dialogue scenario: {{scenario}}".to_string(),
            settings: Settings::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            renew_always: true,
            retry_regenerate: false,
            prompt_experiments: true,
            system_experiments: true,
            prevent_imperson: true,
            all_samples: false,
            no_samples: false,
            strip_assistant: false,
            strip_human: false,
            pass_params: false,
            clear_flags: true,
            preserve_chats: false,
            log_messages: true,
            full_colon: true,
            padtxt: "1000,1000,15000".to_string(),
            xml_plot: true,
            skip_restricted: false,
            artifacts: false,
            superfetch: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let file_string = std::fs::read_to_string(CONFIG_PATH);
        match file_string {
            Ok(file_string) => {
                let config: Config = toml::de::from_str(&file_string)?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default_config = Config::default();
                default_config.save()?;
                println!("Default config file created at {}", CONFIG_PATH);
                println!("{}", "SET YOUR COOKIE THERE".green());
                Ok(default_config)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self) -> Result<()> {
        // Check if the config directory exists, if not create it
        if !std::path::Path::new("config").exists() {
            std::fs::create_dir("config")?;
        }
        // Save the config to a file
        let config_string = toml::ser::to_string(self)?;
        std::fs::write(CONFIG_PATH, config_string)?;
        Ok(())
    }

    pub fn trim(mut self) -> Self {
        // trim and remove non-ASCII characters from cookie
        self.cookie = trim_cookie(&self.cookie);
        self.cookie_array = self.cookie_array.iter().map(|c| trim_cookie(c)).collect();
        self.wasted_cookie = self.wasted_cookie.iter().map(|c| trim_cookie(c)).collect();
        self.unknown_models = self
            .unknown_models
            .iter()
            .map(|c| c.trim().to_string())
            .collect();
        self.ip = self.ip.trim().to_string();
        self.rproxy = self.rproxy.trim().to_string();
        self.api_rproxy = self
            .api_rproxy
            .trim()
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .to_string();
        self.settings.padtxt = self.settings.padtxt.trim().to_string();
        self
    }
}

fn trim_cookie(cookie: &str) -> String {
    cookie
        .chars()
        .filter(|c| c.is_ascii())
        .collect::<String>()
        .trim()
        .to_string()
}
