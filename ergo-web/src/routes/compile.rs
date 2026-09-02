//! `POST /api/v1/compile` — source (+ parameters) → tree, addresses, the
//! decompiled round-trip, and findings. The write side of the playground.

use axum::Json;
use ergo_sandbox::audit;
use ergo_sandbox::compile::{compile_with_params, scan_params, ParamError};

use crate::routes::inspect::parse_network;
use crate::{dto, error::ApiError, extract::ApiJson};

pub async fn compile_route(
    ApiJson(req): ApiJson<dto::CompileRequest>,
) -> Result<Json<dto::CompileResponse>, ApiError> {
    let network = parse_network(req.network.as_deref())?;
    let testnet = network == ergo_ser::address::NetworkPrefix::Testnet;
    let tree_version = req.tree_version.unwrap_or(3);
    if req.source.trim().is_empty() {
        return Err(ApiError::InvalidInput("source is empty".into()));
    }

    let source = req.source;
    let needs_for_error = scan_params(&source);
    let params = req.params;
    let result = tokio::task::spawn_blocking(move || {
        ergo_sandbox::decompile::with_large_stack(move || {
            let needs = scan_params(&source);
            let out = compile_with_params(&source, &params, tree_version, network)?;
            let tree = ergo_sandbox::inspect::parse_tree(&out.tree_bytes).map_err(|e| {
                ParamError::Value {
                    name: "<tree>".into(),
                    reason: e.to_string(),
                }
            })?;
            let lifted = ergo_sandbox::lift_tree(&tree, testnet);
            let roundtrip = ergo_sandbox::decompile::print(&lifted.node);
            let report = audit::audit(&lifted);
            let statuses = needs
                .into_iter()
                .map(|n| dto::ParamStatus {
                    supplied: params.contains_key(&n.name),
                    name: n.name,
                    type_hint: n.type_hint,
                })
                .collect();
            Ok::<_, ParamError>((out, roundtrip, report, statuses))
        })
    })
    .await
    .map_err(|_| ApiError::Internal)?;

    let (out, roundtrip, report, statuses) = match result {
        Ok(v) => v,
        Err(ParamError::Missing(names)) => {
            // Carry the scan's type hints so the UI can build the form.
            return Err(ApiError::MissingParams(
                names
                    .into_iter()
                    .map(|name| ergo_sandbox::compile::ParamNeed {
                        type_hint: needs_for_error
                            .iter()
                            .find(|n| n.name == name)
                            .and_then(|n| n.type_hint.clone()),
                        name,
                    })
                    .collect(),
            ));
        }
        Err(ParamError::Value { name, reason }) => {
            return Err(ApiError::InvalidInput(format!(
                "parameter `{name}`: {reason}"
            )))
        }
        Err(ParamError::Compile(e)) => {
            return Err(ApiError::CompileError {
                message: format!("compile failed: {e}"),
                offset: Some(e.pos()),
            })
        }
    };
    let (completeness, raw_placeholders, truncated) = dto::completeness_parts(&report);
    Ok(Json(dto::CompileResponse {
        tree_hex: hex::encode(&out.tree_bytes),
        p2s: out.p2s_address,
        p2sh: out.p2sh_address,
        source: roundtrip,
        completeness,
        raw_placeholders,
        truncated,
        findings: report
            .findings
            .iter()
            .map(dto::FindingDto::from_engine)
            .collect(),
        params: statuses,
    }))
}
