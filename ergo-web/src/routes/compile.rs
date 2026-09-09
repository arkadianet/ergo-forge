//! `POST /api/v1/compile` — source (+ parameters) → tree, addresses, the
//! decompiled round-trip, and findings. The write side of the playground.
//!
//! Findings are positioned in the authored source through the compiler's
//! P5-B source map and the lift's shared IR ids, when the map aligns with
//! the tree (`SourceMap::aligns_with`). Templates have no map yet.

use axum::{extract::State, Json};
use ergo_sandbox::audit;
use ergo_sandbox::compile::{compile_with_params_and_map, is_template, scan_params, ParamError};

use crate::app::AppState;
use crate::routes::inspect::parse_network;
use crate::{dto, error::ApiError, extract::ApiJson};

type Compiled = (
    ergo_sandbox::compile::CompileOutput,
    String,
    audit::Audit,
    Vec<dto::FindingDto>,
    Vec<dto::ParamStatus>,
    bool,
    ergo_sandbox::recognize::Plain,
);

pub async fn compile_route(
    State(state): State<std::sync::Arc<AppState>>,
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
    let template = is_template(&source);
    let substituted = may_substitute(&source, &params);
    let result = state
        .engine
        .run(move || {
            let needs = scan_params(&source);
            let (out, map) = compile_with_params_and_map(&source, &params, tree_version, network)?;
            let tree = ergo_sandbox::inspect::parse_tree(&out.tree_bytes).map_err(|e| {
                ParamError::Value {
                    name: "<tree>".into(),
                    reason: e.to_string(),
                }
            })?;
            let lifted = ergo_sandbox::lift_tree(&tree, testnet);
            let roundtrip = ergo_sandbox::decompile::print(&lifted.node);
            let report = audit::audit(&lifted);
            let plain = ergo_sandbox::recognize::plain(&lifted);
            let walk: Vec<u8> = ergo_ser::opcode::preorder(&tree.body)
                .map(|(_, e)| ergo_ser::opcode::node_opcode(e))
                .collect();
            let map = map.filter(|m| m.aligns_with(walk.iter().copied()));
            let positioned = map.is_some();
            let findings: Vec<dto::FindingDto> = report
                .findings
                .iter()
                .map(|f| {
                    let d = dto::FindingDto::from_engine(f);
                    match (map.as_ref(), f.ir_id) {
                        (Some(m), Some(ir)) => match m.offset(ir) {
                            Some(off) => d.with_position(&source, off),
                            None => d,
                        },
                        _ => d,
                    }
                })
                .collect();
            let statuses = needs
                .into_iter()
                .map(|n| dto::ParamStatus {
                    supplied: params.contains_key(&n.name),
                    name: n.name,
                    type_hint: n.type_hint,
                    default: n.default,
                })
                .collect();
            Ok::<Compiled, ParamError>((
                out, roundtrip, report, findings, statuses, positioned, plain,
            ))
        })
        .await
        .ok_or(ApiError::Internal)?;

    let (out, roundtrip, report, findings, statuses, positioned, plain) = match result {
        Ok(v) => v,
        Err(ParamError::Missing(names)) => {
            // Carry the scan's type hints and defaults so the UI can build the form.
            return Err(ApiError::MissingParams(
                names
                    .into_iter()
                    .map(|name| {
                        let hint = needs_for_error.iter().find(|n| n.name == name);
                        ergo_sandbox::compile::ParamNeed {
                            type_hint: hint.and_then(|n| n.type_hint.clone()),
                            default: hint.and_then(|n| n.default.clone()),
                            description: hint.and_then(|n| n.description.clone()),
                            name,
                        }
                    })
                    .collect(),
            ));
        }
        Err(ParamError::Value { name, reason }) => {
            return Err(ApiError::InvalidInput(format!(
                "parameter `{name}`: {reason}"
            )))
        }
        Err(ParamError::Compile(e)) => return Err(compiler_error(e, substituted)),
    };
    let (completeness, raw_placeholders, truncated) = dto::completeness_parts(&report);
    Ok(Json(dto::CompileResponse {
        rent: dto::rent_for(&out.tree_bytes, None),
        plain: plain.paths,
        plain_complete: plain.complete,
        tree_hex: hex::encode(&out.tree_bytes),
        p2s: out.p2s_address,
        p2sh: out.p2sh_address,
        source: roundtrip,
        completeness,
        raw_placeholders,
        truncated,
        findings,
        params: statuses,
        template,
        positioned,
    }))
}

/// Preserve the engine offset, distinguishing real source starts (including
/// zero) from the post-typecheck sentinel. Never infer a phase from a message.
pub(super) fn compiler_error(e: ergo_compiler::CompileError, substituted: bool) -> ApiError {
    use ergo_compiler::CompileError;
    let (phase, positioned) = match &e {
        CompileError::Parse(_) => ("parse", true),
        CompileError::Bind(_) => ("bind", true),
        CompileError::Type(_) => ("type", true),
        CompileError::Root { .. } => ("root", false),
        CompileError::Emit(_) => ("emit", false),
        CompileError::Serializer { .. } => ("serializer", false),
        CompileError::Write(_) => ("write", false),
    };
    ApiError::CompileError {
        message: format!("compile failed: {e}"),
        offset: Some(e.pos()),
        phase,
        offset_source: if !positioned {
            "unavailable"
        } else if substituted {
            "substitutedSource"
        } else {
            "source"
        },
    }
}

// The public sandbox API has no substitution origin map. Conservatively
// withhold authored positions when quoted parameters may be replaced.
pub(super) fn may_substitute(
    source: &str,
    params: &std::collections::BTreeMap<String, ergo_sandbox::scenario::TypedValue>,
) -> bool {
    !is_template(source)
        && source.split('"').skip(1).step_by(2).any(|literal| {
            params.iter().any(|(name, value)| {
                matches!(value.r#type.as_str(), "String" | "Coll[Byte]" | "raw")
                    && (literal.contains(&format!("${name}")) || literal == name)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn write_failure_keeps_zero_without_claiming_a_source_position() {
        let error = ergo_compiler::CompileError::Write(ergo_ser::error::WriteError::InvalidData(
            "test serialization failure".into(),
        ));
        let response = compiler_error(error, false).into_response();
        assert_eq!(response.status(), 400);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["phase"], "write");
        assert_eq!(body["error"]["offset"], 0);
        assert_eq!(body["error"]["offsetSource"], "unavailable");
    }
}
