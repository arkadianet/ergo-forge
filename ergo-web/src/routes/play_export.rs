//! `POST /api/v1/play/export`: a stateless synthetic suite/scenario download.
use axum::{extract::State, Json};
use ergo_sandbox::{play, play_export};
use serde::Deserialize;

use crate::{app::AppState, error::ApiError, extract::ApiJson};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    #[serde(flatten)]
    play: play::PlayRequest,
    input_index: usize,
    kind: play_export::ExportKind,
}

pub async fn export_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<ExportRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state
        .engine
        .run(move || {
            if req.input_index >= req.play.tx.inputs.len() {
                return Err(ergo_sandbox::SandboxError::Scenario(
                    "inputIndex is out of range".into(),
                ));
            }
            let result = play::apply(&req.play)?;
            play_export::export(
                &req.play,
                req.input_index,
                result.inputs[req.input_index].verdict,
            )
            .map(|export| export.document(req.kind))
        })
        .await
        .ok_or(ApiError::Internal)?
        .map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    Ok(Json(result))
}
