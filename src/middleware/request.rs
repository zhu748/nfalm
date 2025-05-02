use std::sync::LazyLock;

use axum::{
    Extension, Json,
    extract::{FromRequest, Request},
    response::IntoResponse,
};

use crate::{
    api::ApiFormat,
    error::ClewdrError,
    state::ClientState,
    types::message::{ContentBlock, CreateMessageParams, Message, Role},
};

use super::transform_oai_response;

/// A custom extractor that unify different api formats
pub struct Preprocess(pub CreateMessageParams, pub Extension<FormatInfo>);
#[derive(Debug, Clone)]
pub struct FormatInfo {
    pub stream: bool,
    pub api_format: ApiFormat,
}

/// Exact test message send by SillyTavern
static TEST_MESSAGE_CLAUDE: LazyLock<Message> = LazyLock::new(|| {
    Message::new_blocks(
        Role::User,
        vec![ContentBlock::Text {
            text: "Hi".to_string(),
        }],
    )
});

static TEST_MESSAGE_OAI: LazyLock<Message> = LazyLock::new(|| Message::new_text(Role::User, "Hi"));

impl FromRequest<ClientState> for Preprocess {
    type Rejection = ClewdrError;

    async fn from_request(req: Request, state: &ClientState) -> Result<Self, Self::Rejection> {
        let uri = req.uri().to_string();
        let Json(mut body) = Json::<CreateMessageParams>::from_request(req, &()).await?;
        if body.model.ends_with("-thinking") {
            body.model = body.model.trim_end_matches("-thinking").to_string();
            body.thinking = Some(Default::default());
        }

        if !body.stream.unwrap_or_default()
            && (body.messages == vec![TEST_MESSAGE_CLAUDE.to_owned()]
                || body.messages == vec![TEST_MESSAGE_OAI.to_owned()])
        {
            // respond with a test message
            return Err(ClewdrError::TestMessage);
        }
        let stream = body.stream.unwrap_or_default();
        let format = if uri.contains("chat/completions") {
            ApiFormat::OpenAI
        } else {
            ApiFormat::Claude
        };
        let mut state = state.clone();
        state.api_format = format;
        state.stream = stream;
        let info = FormatInfo {
            stream,
            api_format: format,
        };
        if let Some(mut r) = state.try_from_cache(body.to_owned()).await {
            r.extensions_mut().insert(info.to_owned());
            let r = transform_oai_response(r).await.into_response();
            return Err(ClewdrError::CacheFound(r));
        }
        Ok(Self(body, Extension(info)))
    }
}
