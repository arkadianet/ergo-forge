//! `POST /api/v1/inspect` — the v1 loop: input → source + findings.

use axum::Json;
use ergo_sandbox::audit;
use ergo_ser::address::NetworkPrefix;

use crate::{dto, error::ApiError, input};

pub async fn inspect(
    Json(req): Json<dto::InspectRequest>,
) -> Result<Json<dto::InspectResponse>, ApiError> {
    let testnet = matches!(req.network.as_deref(), Some("testnet"));
    let network = if testnet {
        NetworkPrefix::Testnet
    } else {
        NetworkPrefix::Mainnet
    };

    let bytes = input::resolve(&req.input, network)?;
    let tree_hex = hex::encode(&bytes);

    // The lift recurses ~3 MiB deep; tokio workers have 2 MiB. Never run it on
    // a runtime worker — a deep contract would abort the process.
    let bytes_for_task = bytes.clone();
    let result = tokio::task::spawn_blocking(move || {
        ergo_sandbox::decompile::with_large_stack(move || {
            let tree = ergo_sandbox::inspect::parse_tree(&bytes_for_task)?;
            let lifted = ergo_sandbox::lift_tree(&tree, testnet);
            let source = ergo_sandbox::decompile::print(&lifted.node);
            let report = audit::audit(&lifted);
            Ok::<_, ergo_sandbox::SandboxError>((source, report))
        })
    })
    .await
    .map_err(|_| ApiError::Internal)?;

    let (source, report) = result.map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    let (completeness, raw_placeholders, truncated) = dto::completeness_parts(&report);

    Ok(Json(dto::InspectResponse {
        address: ergo_ser::address::encode_p2s(network, &bytes),
        tree_hex,
        source,
        completeness,
        raw_placeholders,
        truncated,
        findings: report
            .findings
            .iter()
            .map(dto::FindingDto::from_engine)
            .collect(),
    }))
}
