//! `POST /api/v1/play`: apply a transaction to a set of boxes — the one
//! operation of the browser's sandbox chain. Stateless: the request carries
//! the unspent boxes, the response the verdicts and the new boxes.

use axum::extract::State;
use axum::Json;
use ergo_sandbox::play::{apply, PlayRequest, PlayResult};

use crate::app::AppState;
use crate::error::ApiError;
use crate::extract::ApiJson;

pub async fn play(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<PlayRequest>,
) -> Result<Json<PlayResult>, ApiError> {
    let result = state
        .engine
        .run(move || apply(&req))
        .await
        .ok_or(ApiError::Internal)?
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    Ok(Json(result))
}
