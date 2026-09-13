use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::{app::AppState, dto::CompileRequest, error::ApiError, extract::ApiJson};

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub target: String,
    #[serde(flatten)]
    pub compile: CompileRequest,
}

pub async fn verify_route(
    State(state): State<Arc<AppState>>,
    ApiJson(req): ApiJson<VerifyRequest>,
) -> Result<Json<ergo_sandbox::verify::VerifyReport>, ApiError> {
    let network = super::inspect::parse_network(req.compile.network.as_deref())?;
    if req.compile.source.trim().is_empty() {
        return Err(ApiError::InvalidInput("source is empty".into()));
    }
    state
        .engine
        .run(move || {
            ergo_sandbox::verify::verify(
                &req.target,
                &req.compile.source,
                &req.compile.params,
                req.compile.tree_version.unwrap_or(3),
                network,
            )
        })
        .await
        .ok_or(ApiError::Internal)?
        .map(Json)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))
}
