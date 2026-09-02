//! `ApiJson<T>`: axum's `Json` extractor with rejections mapped onto
//! `ApiError`, so a malformed or oversized body gets the same JSON error
//! envelope as every other failure instead of axum's plain-text reply.

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    Json,
};

use crate::error::ApiError;

pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(v)) => Ok(ApiJson(v)),
            Err(rej) if rej.status() == StatusCode::PAYLOAD_TOO_LARGE => Err(ApiError::TooLarge),
            Err(rej) => Err(ApiError::InvalidInput(rej.body_text())),
        }
    }
}
