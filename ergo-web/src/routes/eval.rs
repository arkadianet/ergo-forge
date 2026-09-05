//! `POST /api/v1/eval` — run a scenario (contract + spending context) on the
//! consensus reducer: verdict, cost, residual proposition, trace.

use axum::{extract::State, Json};
use ergo_sandbox::{eval_scenario, Scenario};

use crate::app::AppState;
use crate::{dto, error::ApiError, extract::ApiJson};

pub async fn eval_route(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(scenario): ApiJson<Scenario>,
) -> Result<Json<dto::EvalResponse>, ApiError> {
    // Compile (when `source` is given) and reduce both recurse; same
    // large-stack blocking task as the other engine routes.
    let source = scenario.source.clone();
    let source_for_task = source.clone();
    let params = scenario.params.clone();
    let tree_version = scenario.tree_version;
    let network = scenario.network.clone();
    let result = state
        .engine
        .run(move || {
            let outcome = eval_scenario(&scenario)?;
            // Position the values in the source through the compiler's
            // source map, when the scenario came from source and the map
            // aligns with the tree that ran.
            let mut positions: std::collections::HashMap<u64, u32> = Default::default();
            if let Some(src) = &source_for_task {
                let net = match network.as_deref() {
                    Some("testnet") => ergo_ser::address::NetworkPrefix::Testnet,
                    _ => ergo_ser::address::NetworkPrefix::Mainnet,
                };
                if let Ok((out, Some(map))) = ergo_sandbox::compile::compile_with_params_and_map(
                    src,
                    &params,
                    tree_version,
                    net,
                ) {
                    let walk: Vec<u8> = ergo_ser::opcode::preorder(&out.ergo_tree.body)
                        .map(|(_, e)| ergo_ser::opcode::node_opcode(e))
                        .collect();
                    if map.aligns_with(walk.iter().copied()) {
                        for v in &outcome.values {
                            if let Some(off) = map.offset(v.ir_id) {
                                positions.insert(v.ir_id, off);
                            }
                        }
                    }
                }
            }
            Ok::<_, ergo_sandbox::SandboxError>((outcome, positions))
        })
        .await
        .ok_or(ApiError::Internal)?;

    // Marshalling and compile errors describe the caller's scenario; a
    // script that ran and failed is a normal outcome inside the response.
    let (outcome, positions) = result.map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    let mut resp = dto::EvalResponse::from_engine(outcome);
    if let Some(src) = &source {
        for v in resp.values.iter_mut() {
            if let Some(&off) = positions.get(&v.ir_id) {
                let (line, col) = ergo_compiler::span::line_col(src, off);
                v.offset = Some(off);
                v.line = Some(line);
                v.col = Some(col);
            }
        }
    }
    Ok(Json(resp))
}
