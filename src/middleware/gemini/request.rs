use axum::{
    Json, RequestExt,
    extract::{FromRequest, Path, Request},
};

use crate::{
    error::ClewdrError, gemini_body::GeminiQuery, types::gemini::request::GeminiRequestBody,
};

pub struct GeminiContext {
    pub stream: bool,
    pub path: String,
    pub query: GeminiQuery,
}

pub struct GeminiPreprocess(pub GeminiRequestBody, pub GeminiContext);

impl<S> FromRequest<S> for GeminiPreprocess
where
    S: Send + Sync,
{
    type Rejection = ClewdrError;

    async fn from_request(mut req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let Path(path) = req.extract_parts::<Path<String>>().await?;
        let uri = req.uri().to_string();
        let query = req.extract_parts::<GeminiQuery>().await?;
        let ctx = GeminiContext {
            stream: uri.contains("streamGenerateContent"),
            path,
            query,
        };
        let Json(body) = Json::<GeminiRequestBody>::from_request(req, &()).await?;
        // body.hash(&mut hasher);
        Ok(GeminiPreprocess(body, ctx))
    }
}
