use axum::{
    Json, RequestExt,
    extract::{FromRequest, Path, Request},
};

use crate::{
    error::ClewdrError, gemini_body::GeminiQuery, gemini_state::GeminiState,
    types::gemini::request::GeminiRequestBody,
};

pub struct GeminiContext {
    pub stream: bool,
    pub path: String,
    pub query: GeminiQuery,
}

pub struct GeminiPreprocess(pub GeminiRequestBody, pub GeminiContext);

impl FromRequest<GeminiState> for GeminiPreprocess {
    type Rejection = ClewdrError;

    async fn from_request(mut req: Request, state: &GeminiState) -> Result<Self, Self::Rejection> {
        let Path(path) = req.extract_parts::<Path<String>>().await?;
        let uri = req.uri().to_string();
        let query = req.extract_parts::<GeminiQuery>().await?;
        let ctx = GeminiContext {
            stream: uri.contains("streamGenerateContent"),
            path,
            query,
        };
        let Json(body) = Json::<GeminiRequestBody>::from_request(req, &()).await?;
        let mut state = state.clone();
        state.update_from_ctx(&ctx);
        if let Some(res) = state.try_from_cache(&body).await {
            return Err(ClewdrError::CacheFound(res));
        }
        Ok(GeminiPreprocess(body, ctx))
    }
}
