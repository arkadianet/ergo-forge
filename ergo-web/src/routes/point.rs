//! `POST /api/v1/point`: the public point of a secret — `g^x`, or `h^x`
//! with a base — so a scenario's `secrets` and a script's constants can be
//! made to agree without leaving the page. The secret is used once and
//! not stored; still, this is for test keys, not wallet keys.

use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::ApiJson;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PointRequest {
    /// 32-byte hex scalar.
    pub secret: String,
    /// Optional compressed base point (33-byte hex); `g` when absent.
    #[serde(default)]
    pub base: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PointResponse {
    /// Compressed 33-byte hex.
    pub point: String,
    /// The generator, for scripts that spell it out.
    pub generator: String,
}

pub async fn point(ApiJson(req): ApiJson<PointRequest>) -> Result<Json<PointResponse>, ApiError> {
    let generator = ergo_sandbox::prove::generator_hex();
    let point = match &req.base {
        Some(base) => ergo_sandbox::prove::dht_hex(&generator, base, &req.secret)
            .map(|(_, v)| v)
            .map_err(|e| ApiError::InvalidInput(e.to_string()))?,
        None => ergo_sandbox::prove::pubkey_hex(&req.secret)
            .map_err(|e| ApiError::InvalidInput(e.to_string()))?,
    };
    Ok(Json(PointResponse { point, generator }))
}
