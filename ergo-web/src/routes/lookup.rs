//! `POST /api/v1/lookup` — fetch a box (by id) or an address's unspent boxes
//! from the configured explorer, in the scenario box shape with registers
//! passed through as raw serialized constants. `GET /api/v1/config` tells a
//! client whether this is available.
//!
//! This is the service's only outbound call and it exists only when
//! `EXPLORER_URL` is set. Everything the explorer returns is treated as
//! data: registers are re-parsed by the engine, never trusted as typed.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::app::AppConfig;
use crate::{error::ApiError, extract::ApiJson};

#[derive(Serialize)]
pub struct ConfigDto {
    /// True when `/api/v1/lookup` can fetch chain data.
    pub explorer: bool,
}

pub async fn config(State(cfg): State<Arc<AppConfig>>) -> Json<ConfigDto> {
    Json(ConfigDto {
        explorer: cfg.explorer_url.is_some(),
    })
}

#[derive(Deserialize)]
pub struct LookupRequest {
    /// A box id (64 hex chars) or an address.
    pub input: String,
    /// Most boxes to return for an address (default 20, max 100).
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupResponse {
    /// Current chain height, for a realistic spending height.
    pub height: Option<u32>,
    /// Boxes in the scenario shape (`value`, `ergoTree`, `tokens`,
    /// `creationHeight`, `registers` as `{type: "raw", value: hex}`, `boxId`).
    pub boxes: Vec<serde_json::Value>,
}

pub async fn lookup(
    State(cfg): State<Arc<AppConfig>>,
    ApiJson(req): ApiJson<LookupRequest>,
) -> Result<Json<LookupResponse>, ApiError> {
    let Some(base) = cfg.explorer_url.as_deref() else {
        return Err(ApiError::NotConfigured(
            "chain lookups need EXPLORER_URL; this instance makes no outbound calls".into(),
        ));
    };
    let input = req.input.trim();
    if input.is_empty() {
        return Err(ApiError::InvalidInput("input is empty".into()));
    }
    let limit = req.limit.unwrap_or(20).clamp(1, 100);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| ApiError::Upstream(e.to_string()))?;

    let is_box_id = input.len() == 64 && input.chars().all(|c| c.is_ascii_hexdigit());
    let raw_boxes: Vec<serde_json::Value> = if is_box_id {
        let v = fetch(&client, &format!("{base}/api/v1/boxes/{input}")).await?;
        vec![v]
    } else {
        let v = fetch(
            &client,
            &format!("{base}/api/v1/boxes/unspent/byAddress/{input}?limit={limit}"),
        )
        .await?;
        v.get("items")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let height = fetch(&client, &format!("{base}/api/v1/networkState"))
        .await
        .ok()
        .and_then(|v| v.get("height").and_then(|h| h.as_u64()))
        .map(|h| h as u32);

    let boxes = raw_boxes.iter().map(to_scenario_box).collect();
    Ok(Json(LookupResponse { height, boxes }))
}

async fn fetch(client: &reqwest::Client, url: &str) -> Result<serde_json::Value, ApiError> {
    let r = client
        .get(url)
        .send()
        .await
        .map_err(|e| ApiError::Upstream(format!("explorer request failed: {e}")))?;
    if r.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ApiError::NotFound(
            "no such box or address on the explorer".into(),
        ));
    }
    if !r.status().is_success() {
        return Err(ApiError::Upstream(format!(
            "explorer answered {}",
            r.status()
        )));
    }
    r.json()
        .await
        .map_err(|e| ApiError::Upstream(format!("explorer answered non-JSON: {e}")))
}

/// Explorer box JSON → the scenario box shape. Registers become
/// `{"type": "raw", "value": serializedValue}`; the engine parses them.
fn to_scenario_box(b: &serde_json::Value) -> serde_json::Value {
    let tokens: Vec<serde_json::Value> = b
        .get("assets")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .map(|t| serde_json::json!({ "id": t["tokenId"], "amount": t["amount"] }))
                .collect()
        })
        .unwrap_or_default();
    let mut registers = serde_json::Map::new();
    if let Some(regs) = b.get("additionalRegisters").and_then(|r| r.as_object()) {
        let mut keys: Vec<&String> = regs.keys().collect();
        keys.sort();
        for k in keys {
            let raw = regs[k]
                .get("serializedValue")
                .and_then(|s| s.as_str())
                .or_else(|| regs[k].as_str());
            if let Some(hex) = raw {
                registers.insert(
                    k.clone(),
                    serde_json::json!({ "type": "raw", "value": hex }),
                );
            }
        }
    }
    serde_json::json!({
        "boxId": b["boxId"],
        "value": b["value"],
        "ergoTree": b["ergoTree"],
        "creationHeight": b["creationHeight"],
        "tokens": tokens,
        "registers": registers,
    })
}
