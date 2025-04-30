use axum::{
    Json,
    extract::{FromRequest, Request},
};

use crate::{error::ClewdrError, types::message::CreateMessageParams};

/// A custom extractor that unify different api formats
pub struct UnifiedRequestBody(pub CreateMessageParams);

impl<S> FromRequest<S> for UnifiedRequestBody
where
    S: Sync,
{
    type Rejection = ClewdrError;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let Json(mut body) = Json::<CreateMessageParams>::from_request(req, &()).await?;
        if body.model.ends_with("-thinking") {
            body.model = body.model.trim_end_matches("-thinking").to_string();
            body.thinking = Some(Default::default());
        } else if body.thinking.is_some() {
            body.thinking = Some(Default::default());
        }
        Ok(Self(body))
    }
}
