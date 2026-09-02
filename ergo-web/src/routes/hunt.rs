//! `POST /api/v1/hunt` — the spend hunt: can anyone spend this box with no key?

use axum::Json;
use ergo_sandbox::hunt::{hunt, HuntOptions};

use crate::routes::inspect::parse_network;
use crate::{dto, error::ApiError, extract::ApiJson, input};

pub async fn hunt_route(
    ApiJson(req): ApiJson<dto::HuntRequest>,
) -> Result<Json<dto::HuntResponse>, ApiError> {
    let network = parse_network(req.network.as_deref())?;
    let bytes = input::resolve(&req.input, network)?;
    let tree_hex = hex::encode(&bytes);
    let address = ergo_ser::address::encode_p2s(network, &bytes);

    let opts = HuntOptions {
        height: req.height,
        self_box: req.self_box,
        network: Some(network),
        data_inputs: req.data_inputs,
    };

    // The reducer recurses like the lift does; same large-stack blocking task.
    let bytes_for_task = bytes.clone();
    let result = tokio::task::spawn_blocking(move || {
        ergo_sandbox::decompile::with_large_stack(move || hunt(&bytes_for_task, &opts))
    })
    .await
    .map_err(|_| ApiError::Internal)?;

    // Marshalling errors (bad tree, bad selfBox value) describe the caller's
    // input; script outcomes are inside the Hunt, never errors.
    let h = result.map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    Ok(Json(dto::HuntResponse::from_engine(tree_hex, address, &h)))
}
