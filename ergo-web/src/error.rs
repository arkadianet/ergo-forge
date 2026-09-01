//! API errors. Never leaks internal detail to the client.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    InvalidInput(String),
    TooLarge,
    Internal,
}

#[derive(Serialize)]
struct Body {
    error: Inner,
}
#[derive(Serialize)]
struct Inner {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::InvalidInput(m) => (StatusCode::BAD_REQUEST, "invalid_input", m),
            ApiError::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "too_large",
                "request body too large".to_string(),
            ),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal error".to_string(),
            ),
        };
        (
            status,
            Json(Body {
                error: Inner { code, message },
            }),
        )
            .into_response()
    }
}
