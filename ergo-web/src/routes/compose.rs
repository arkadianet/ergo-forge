//! `POST /api/v1/compose` — spending paths (who + conditions) → ErgoScript
//! source with `$name` params; with values, a generated suite (and, with
//! `run: true`, its results).

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{extract::State, Json};
use ergo_sandbox::compose::{compose, Spec};
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
    let composed =
        compose(&req.spec, &req.params).map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    let results = match (&composed.suite, req.run) {
        (Some(suite), true) => {
            let suite = suite.clone();
            let r = state
                .engine
                .run(move || run(&suite))
                .await
                .ok_or(ApiError::Internal)?
                .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
            Some(r)
        }
        _ => None,
    };
    Ok(Json(ComposeResponse {
        source: composed.source,
        params: composed.params,
        suite: composed.suite,
        results,
    }))
}
