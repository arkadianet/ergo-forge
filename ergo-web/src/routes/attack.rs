//! `POST /api/v2/attack`: apply adversarial operations to a drafted Play
//! transaction and report which input scripts changed verdict. Synthetic, like
//! Play — the response carries `nodeValidated: false` and asserts nothing about
//! any deployed contract.

use axum::extract::State;
use axum::Json;
use ergo_sandbox::attack::{apply_attack, AttackRequest, AttackResult};

use crate::app::AppState;
use crate::error::ApiError;
use crate::extract::ApiJson;

pub async fn attack_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<AttackRequest>,
) -> Result<Json<AttackResult>, ApiError> {
    let result = state
        .engine
        .run(move || apply_attack(&req))
        .await
        .ok_or(ApiError::Internal)?
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    Ok(Json(result))
}
