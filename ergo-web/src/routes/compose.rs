//! `POST /api/v1/compose` — spending paths (who + conditions) → ErgoScript
//! source with `$name` params; with values, a generated suite (and, with
//! `run: true`, its results).

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{extract::State, Json};
use ergo_sandbox::compose::{compose, validate_public_limits, Spec};
use ergo_sandbox::testsuite::{run, SuiteResult};
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::{error::ApiError, extract::ApiJson};

#[derive(Deserialize)]
pub struct ComposeRequest {
    pub spec: Spec,
    #[serde(default)]
    pub params: BTreeMap<String, ergo_sandbox::TypedValue>,
    /// Run the generated suite too.
    #[serde(default)]
    pub run: bool,
}

#[derive(Serialize)]
pub struct ComposeResponse {
    #[serde(flatten)]
    pub claim: ergo_sandbox::claim::ClaimMetadata,
    pub source: String,
    pub params: Vec<ergo_sandbox::compile::ParamNeed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<ergo_sandbox::testsuite::Suite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<SuiteResult>,
}

pub async fn compose_route(
    State(state): State<Arc<AppState>>,
    ApiJson(req): ApiJson<ComposeRequest>,
) -> Result<Json<ComposeResponse>, ApiError> {
    validate_public_limits(&req.spec, &req.params)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    let should_run = req.run;
    let (composed, results) = state
        .engine
        .run(move || {
            let composed = compose(&req.spec, &req.params)
                .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
            let results = match (&composed.suite, should_run) {
                (Some(suite), true) => {
                    Some(run(suite).map_err(|e| ApiError::InvalidInput(e.to_string()))?)
                }
                _ => None,
            };
            Ok::<_, ApiError>((composed, results))
        })
        .await
        .ok_or(ApiError::Internal)??;
    Ok(Json(ComposeResponse {
        claim: composed.claim,
        source: composed.source,
        params: composed.params,
        suite: composed.suite,
        results,
    }))
}
