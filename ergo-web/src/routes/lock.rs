use crate::{app::AppState, dto::CompileRequest, error::ApiError, extract::ApiJson};
use axum::{extract::State, Json};
use std::sync::Arc;

pub async fn lock_route(
    State(state): State<Arc<AppState>>,
    ApiJson(req): ApiJson<CompileRequest>,
) -> Result<Json<ergo_sandbox::lockfile::Lockfile>, ApiError> {
    let network = super::inspect::parse_network(req.network.as_deref())?;
    if req.source.trim().is_empty() {
        return Err(ApiError::InvalidInput("source is empty".into()));
    }
    state
        .engine
        .run(move || {
            ergo_sandbox::lockfile::create(
                &req.source,
                &req.params,
                req.tree_version.unwrap_or(3),
                network,
            )
        })
        .await
        .ok_or(ApiError::Internal)?
        .map(Json)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))
}
