//! `POST /api/v1/validate-tx` — will this unsigned transaction validate?
//! Boxes the request does not carry are fetched from the explorer when one
//! is configured; otherwise they are reported missing.

use std::sync::Arc;

use axum::{extract::State, Json};
use ergo_sandbox::txcheck::{check, TxCheck, TxRequest};

use crate::app::AppState;
use crate::{error::ApiError, extract::ApiJson};

pub async fn validate_tx(
    State(state): State<Arc<AppState>>,
    ApiJson(mut req): ApiJson<TxRequest>,
) -> Result<Json<TxCheck>, ApiError> {
    if let Some(base) = state.cfg.explorer_url.as_deref() {
        let have: std::collections::HashSet<String> = req
            .boxes
            .iter()
            .filter_map(|b| b["boxId"].as_str().map(|s| s.to_lowercase()))
            .collect();
        let wanted: Vec<String> = req
            .tx
            .inputs
            .iter()
            .chain(req.tx.data_inputs.iter())
            .map(|i| i.box_id.to_lowercase())
            .filter(|id| !have.contains(id))
            .collect();
        if !wanted.is_empty() {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| ApiError::Upstream(e.to_string()))?;
            for id in wanted {
                if id.len() != 64 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
                    continue; // left missing; the check reports it
                }
                if let Ok(v) = crate::routes::lookup::fetch_box(&client, base, &id).await {
                    req.boxes.push(v);
                }
            }
            if req.height.is_none() {
                req.height = crate::routes::lookup::fetch_height(&client, base).await;
            }
        }
    }
    let result = state
        .engine
        .run(move || check(&req))
        .await
        .ok_or(ApiError::Internal)?;
    result
        .map(Json)
        .map_err(|e| ApiError::InvalidInput(e.to_string()))
}
