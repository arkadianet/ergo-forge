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
    /// A source needs parameters the request did not supply.
    MissingParams(Vec<ergo_sandbox::compile::ParamNeed>),
    /// The compiler rejected the source; `offset` when the error carries one.
    CompileError {
        message: String,
        offset: Option<u32>,
    },
    NotFound(String),
    TooLarge,
    Internal,
}

#[derive(Serialize)]
struct Body {
    error: Inner,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Inner {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_params: Option<Vec<ergo_sandbox::compile::ParamNeed>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut offset = None;
        let mut missing_params = None;
        let (status, code, message) = match self {
            ApiError::InvalidInput(m) => (StatusCode::BAD_REQUEST, "invalid_input", m),
            ApiError::MissingParams(needs) => {
                let names: Vec<&str> = needs.iter().map(|n| n.name.as_str()).collect();
                let m = format!("missing parameters: {}", names.join(", "));
                missing_params = Some(needs);
                (StatusCode::BAD_REQUEST, "missing_params", m)
            }
            ApiError::CompileError { message, offset: o } => {
                offset = o;
                (StatusCode::BAD_REQUEST, "compile_error", message)
            }
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, "not_found", m),
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
                error: Inner {
                    code,
                    message,
                    offset,
                    missing_params,
                },
            }),
        )
            .into_response()
    }
}
