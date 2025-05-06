use std::sync::LazyLock;

use rquest::{Client, ClientBuilder, Proxy};

use crate::{config::KeyStatus, error::ClewdrError, services::key_manager::KeyEventSender};

#[derive(Clone)]
pub enum GeminiApiFormat {
    Gemini,
    OpenAI,
}

static DUMMY_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Clone)]
pub struct GeminiState {
    pub key: Option<KeyStatus>,
    pub fake_stream: bool,
    pub proxy: Option<Proxy>,
    pub event_sender: KeyEventSender,
    pub api_format: GeminiApiFormat,
    pub client: Client,
    pub cache_key: Option<(u64, usize)>,
}

impl GeminiState {
    /// Create a new AppState instance
    pub fn new(tx: KeyEventSender) -> Self {
        GeminiState {
            key: None,
            fake_stream: false,
            event_sender: tx,
            proxy: None,
            api_format: GeminiApiFormat::Gemini,
            client: DUMMY_CLIENT.to_owned(),
            cache_key: None,
        }
    }

    pub async fn request_key(&mut self) -> Result<(), ClewdrError> {
        let key = self.event_sender.request().await?;
        self.key = Some(key.to_owned());
        let client = ClientBuilder::new();
        let client = if let Some(proxy) = self.proxy.to_owned() {
            client.proxy(proxy)
        } else {
            client
        };
        self.client = client.build()?;
        Ok(())
    }
}
