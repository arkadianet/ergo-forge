//! `POST /api/v1/inspect` — the v1 loop: input → source + findings.

use axum::{extract::State, Json};
use ergo_sandbox::audit;
use ergo_ser::address::NetworkPrefix;

use crate::app::AppState;
use crate::{dto, error::ApiError, extract::ApiJson, input};

/// Parse the optional `network` field. Absent means mainnet; anything other
/// than the two exact spellings is an input error rather than a silent
/// mainnet default — a wrong network yields a wrong address, not a failure.
pub(crate) fn parse_network(raw: Option<&str>) -> Result<NetworkPrefix, ApiError> {
    match raw {
        None | Some("mainnet") => Ok(NetworkPrefix::Mainnet),
        Some("testnet") => Ok(NetworkPrefix::Testnet),
        Some(other) => Err(ApiError::InvalidInput(format!(
            "unknown network {other:?}; expected \"mainnet\" or \"testnet\""
        ))),
    }
}

pub async fn inspect(
    State(state): State<std::sync::Arc<AppState>>,
    ApiJson(req): ApiJson<dto::InspectRequest>,
) -> Result<Json<dto::InspectResponse>, ApiError> {
    let network = parse_network(req.network.as_deref())?;
    let testnet = network == NetworkPrefix::Testnet;

    let bytes = input::resolve(&req.input, network)?;
    let tree_hex = hex::encode(&bytes);

    // The lift recurses ~3 MiB deep; tokio workers have 2 MiB. Never run it on
    // a runtime worker — a deep contract would abort the process. Concurrency
    // is bounded by the limit layer on this route (see `app.rs`), so the
    // number of large-stack threads alive at once is bounded too.
    let bytes_for_task = bytes.clone();
    let result = state
        .engine
        .run(move || {
            let tree = ergo_sandbox::inspect::parse_tree(&bytes_for_task)?;
            let lifted = ergo_sandbox::lift_tree(&tree, testnet);
            let source = ergo_sandbox::decompile::print(&lifted.node);
            let report = audit::audit(&lifted);
            let plain = ergo_sandbox::recognize::plain(&lifted);
            Ok::<_, ergo_sandbox::SandboxError>((source, report, plain))
        })
        .await
        .ok_or(ApiError::Internal)?;

    // The parser's message describes the caller's own bytes (offset, opcode),
    // not server state — it is the useful part of a 400, so it is passed on.
    let (source, report, plain) = result.map_err(|e| ApiError::InvalidInput(e.to_string()))?;
    let (completeness, raw_placeholders, truncated) = dto::completeness_parts(&report);

    Ok(Json(dto::InspectResponse {
        rent: dto::rent_for(&bytes, None),
        plain: plain.paths,
        plain_complete: plain.complete,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_network_is_mainnet() {
        assert_eq!(parse_network(None).unwrap(), NetworkPrefix::Mainnet);
    }

    #[test]
    fn only_exact_spellings_are_accepted() {
        assert_eq!(
            parse_network(Some("testnet")).unwrap(),
            NetworkPrefix::Testnet
        );
        for bad in ["testnet ", "Testnet", "MAINNET", ""] {
            assert!(parse_network(Some(bad)).is_err(), "{bad:?} accepted");
        }
    }
}
