//! Descriptive labels for legacy results, not execution evidence or a case schema.
//! No legacy producer runs full node validation. These labels cannot promote one.
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimMetadata {
    method: &'static str,
    provenance: &'static str,
    node_validated: bool,
}

impl ClaimMetadata {
    pub const fn legacy(method: &'static str, provenance: &'static str) -> Self {
        Self {
            method,
            provenance,
            node_validated: false,
        }
    }
    pub const STATIC: Self = Self::legacy(
        "static-analysis",
        "supplied-code; deployment identity not established",
    );
    pub const SIMULATION: Self = Self::legacy(
        "scenario-simulation",
        "caller-supplied/default context and box material; proofs use a supplied/default message",
    );
    pub const PREFLIGHT: Self = Self::legacy(
        "unsigned-preflight",
        "caller-supplied/generated boxes, IDs and context; signatures not checked",
    );
    pub const INGEST: Self = Self::legacy(
        "source-inference",
        "source with caller overrides or synthetic bindings; deployment identity not established",
    );
}
