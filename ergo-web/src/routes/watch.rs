//! Watch reads only the instance's configured explorer, under the shared budget.
use crate::{app::AppState, error::ApiError, extract::ApiJson};
use axum::{extract::State, Json};
use ergo_sandbox::{
    map::{explorer::ExplorerSource, source::ChainSource},
    watch::{self, WatchInput, WatchReport},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchRequest {
    pub watches: Vec<WatchInput>,
}

pub async fn watch_route(
    State(state): State<Arc<AppState>>,
    ApiJson(req): ApiJson<WatchRequest>,
) -> Result<Json<Vec<WatchReport>>, ApiError> {
    watch::validate(&req.watches).map_err(ApiError::InvalidInput)?;
    let base = state.cfg.explorer_url.clone();
    state
        .engine
        .run(move || {
            let source = base
                .as_deref()
                .filter(|url| !url.trim().is_empty())
                .map(ExplorerSource::new);
            watch::observe(&req.watches, source.as_ref().map(|s| s as &dyn ChainSource))
        })
        .await
        .ok_or(ApiError::Internal)?
        .map(Json)
        .map_err(ApiError::InvalidInput)
}
