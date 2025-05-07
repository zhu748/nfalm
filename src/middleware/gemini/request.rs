use axum::{
    Json, RequestExt,
    extract::{FromRequest, Path, Request},
};

use crate::{
    config::CLEWDR_CONFIG, error::ClewdrError, gemini_body::GeminiQuery, gemini_state::GeminiState,
    types::gemini::request::GeminiRequestBody,
};

pub struct GeminiContext {
    pub model: String,
    pub vertex: bool,
    pub stream: bool,
    pub path: String,
    pub query: GeminiQuery,
}

pub struct GeminiPreprocess(pub GeminiRequestBody, pub GeminiContext);

impl FromRequest<GeminiState> for GeminiPreprocess {
    type Rejection = ClewdrError;

    async fn from_request(mut req: Request, state: &GeminiState) -> Result<Self, Self::Rejection> {
        let Path(path) = req.extract_parts::<Path<String>>().await?;
        let vertex = req.uri().to_string().contains("vertex");
        if vertex && !CLEWDR_CONFIG.load().vertex.validate() {
            return Err(ClewdrError::BadRequest(
                "Vertex is not configured".to_string(),
            ));
        }
        let mut model = path
            .split('/')
            .last()
            .map(|s| s.split_once(':').map(|s| s.0).unwrap_or(s).to_string());
        if vertex {
            model = CLEWDR_CONFIG.load().vertex.model_id.to_owned().or(model)
        }
        let Some(model) = model else {
            return Err(ClewdrError::BadRequest(
                "Model not found in path or vertex config".to_string(),
            ));
        };
        let query = req.extract_parts::<GeminiQuery>().await?;
        let ctx = GeminiContext {
            vertex,
            model,
            stream: path.contains("streamGenerateContent"),
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
