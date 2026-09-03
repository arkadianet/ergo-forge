//! `POST /api/v1/test` — run a contract test suite (contract + named
//! scenarios with expected verdicts). Body is the suite; the response is
//! the per-case table.

use axum::{extract::State, Json};
use ergo_sandbox::testsuite::{run, Suite, SuiteError, SuiteResult};

use crate::app::AppState;
use crate::{error::ApiError, extract::ApiJson};

pub async fn test_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(suite): ApiJson<Suite>,
) -> Result<Json<SuiteResult>, ApiError> {
    let result = state
        .engine
        .run(move || run(&suite))
        .await
        .ok_or(ApiError::Internal)?;
    match result {
        Ok(r) => Ok(Json(r)),
        Err(SuiteError::Compile(ergo_sandbox::compile::ParamError::Compile(e))) => {
            Err(ApiError::CompileError {
                message: format!("compile failed: {e}"),
                offset: Some(e.pos()),
            })
        }
        Err(e) => Err(ApiError::InvalidInput(e.to_string())),
    }
}
