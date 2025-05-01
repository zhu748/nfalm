use axum::{
    Extension, Json,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response, Sse},
};
use eventsource_stream::Eventsource;

use crate::{
    api::ApiFormat, error::ClewdrError, types::message::CreateMessageParams,
    utils::transform_stream,
};

/// A custom extractor that unify different api formats
pub struct UnifiedRequestBody(pub CreateMessageParams, pub Extension<FormatInfo>);
#[derive(Debug, Clone)]
pub struct FormatInfo {
    pub stream: bool,
    pub api_format: ApiFormat,
}
impl Default for FormatInfo {
    fn default() -> Self {
        Self {
            stream: false,
            api_format: ApiFormat::Claude,
        }
    }
}

impl<S> FromRequest<S> for UnifiedRequestBody
where
    S: Sync,
{
    type Rejection = ClewdrError;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let uri = req.uri().to_string();
        let Json(mut body) = Json::<CreateMessageParams>::from_request(req, &()).await?;
        if body.model.ends_with("-thinking") {
            body.model = body.model.trim_end_matches("-thinking").to_string();
            body.thinking = Some(Default::default());
        }

        let format = if uri.contains("chat/completions") {
            ApiFormat::OpenAI
        } else {
            ApiFormat::Claude
        };
        let stream = body.stream.unwrap_or_default();
        Ok(Self(
            body,
            Extension(FormatInfo {
                stream,
                api_format: format,
            }),
        ))
    }
}

pub async fn transform_oai_response(resp: Response) -> Response {
    let Some(f) = resp.extensions().get::<FormatInfo>() else {
        return resp;
    };
    if ApiFormat::Claude == f.api_format || !f.stream || resp.status() != 200 {
        return resp;
    }
    let body = resp.into_body();
    let stream = body.into_data_stream().eventsource();
    let stream = transform_stream(stream);
    Sse::new(stream)
        .keep_alive(Default::default())
        .into_response()
}
