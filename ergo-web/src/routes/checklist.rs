//! Checklist jobs share inspect's input resolution, network and engine budget.
use axum::{extract::State, Json};
use ergo_sandbox::checklist::{checklist, Checklist};

use crate::{app::AppState, dto::ChecklistRequest, error::ApiError, extract::ApiJson, input};

pub async fn checklist_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<ChecklistRequest>,
) -> Result<Json<Checklist>, ApiError> {
    let network = super::inspect::parse_network(req.network.as_deref())?;
    let bytes = match (&req.input, &req.source) {
        (Some(input), None) => Some(input::resolve(input, network)?),
        (None, Some(source)) if source.len() > input::MAX_INPUT_CHARS => {
            return Err(ApiError::TooLarge)
        }
        (None, Some(_)) => None,
        _ => {
            return Err(ApiError::InvalidInput(
                "supply exactly one of input or source".into(),
            ))
        }
    };
    state
        .engine
        .run(move || {
            let bytes = match bytes {
                Some(bytes) => bytes,
                None => {
                    ergo_sandbox::compile_source(
                        req.source.as_deref().expect("source checked"),
                        3,
                        network,
                    )
                    .map_err(|e| e.to_string())?
                    .tree_bytes
                }
            };
            checklist(&bytes, network, &req.artifacts)
        })
        .await
        .ok_or(ApiError::Internal)?
        .map(Json)
        .map_err(ApiError::InvalidInput)
}
