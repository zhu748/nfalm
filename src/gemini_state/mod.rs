use rquest::{Client, Proxy};

pub enum GeminiApiFormat {
    Gemini,
    OpenAI,
}

pub struct GeminiState {
    pub key: Option<String>,
    pub fake_stream: bool,
    pub vertex: bool,
    // TODO: impl KeyEventSender
    pub proxy: Option<Proxy>,
    pub api_format: GeminiApiFormat,
    pub client: Client,
    pub cache_key: Option<(u64, usize)>,
}

