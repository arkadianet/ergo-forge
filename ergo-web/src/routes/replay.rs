//! Offline replay of a supplied P05 bundle; uses the shared engine budget.
use std::sync::Arc;

use axum::{extract::State, Json};
use ergo_sandbox::evidence::replay::{replay, ReplayBundle};
use serde::Serialize;
use serde_json::Value;

use crate::{app::AppState, error::ApiError, extract::ApiJson};

/// API version is independent of the unchanged CLI replay report version.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResponse {
    api_version: u32,
    result: Value,
}

pub async fn replay_route(
    State(state): State<Arc<AppState>>,
    ApiJson(bundle): ApiJson<ReplayBundle>,
) -> Result<Json<ReplayResponse>, ApiError> {
    let result = state
        .engine
        .run(move || replay(&bundle))
        .await
        .ok_or(ApiError::Internal)?;
    Ok(Json(ReplayResponse {
        api_version: 2,
        result,
    }))
}
