//! `POST /api/v1/eval` — run a scenario (contract + spending context) on the
//! consensus reducer: verdict, cost, residual proposition, trace.

use axum::Json;
use ergo_sandbox::{eval_scenario, Scenario};

use crate::{dto, error::ApiError, extract::ApiJson};

pub async fn eval_route(
    ApiJson(scenario): ApiJson<Scenario>,
) -> Result<Json<dto::EvalResponse>, ApiError> {
    // Compile (when `source` is given) and reduce both recurse; same
    // large-stack blocking task as the other engine routes.
    let result = tokio::task::spawn_blocking(move || {
        ergo_sandbox::decompile::with_large_stack(move || eval_scenario(&scenario))
    })
    .await
    .map_err(|_| ApiError::Internal)?;

    // Marshalling and compile errors describe the caller's scenario; a
    // script that ran and failed is a normal outcome inside the response.
    let outcome = result.map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    Ok(Json(dto::EvalResponse::from_engine(outcome)))
}
