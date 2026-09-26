use axum::extract::State;
use axum::Json;
use ergo_sandbox::adversary::{search, SearchReport, SearchRequest};

use crate::app::AppState;
use crate::error::ApiError;
use crate::extract::ApiJson;

pub async fn adversary_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<SearchRequest>,
) -> Result<Json<SearchReport>, ApiError> {
    let result = state
        .engine
        .run(move || search(&req))
        .await
        .ok_or(ApiError::Internal)?
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    Ok(Json(result))
}
