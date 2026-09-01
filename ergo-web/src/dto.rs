//! Wire types. Deliberately separate from the engine's types: the API is a
//! versioned contract and the engine must stay free to change.

use ergo_sandbox::audit::{Audit, Completeness};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct InspectRequest {
    pub input: String,
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Serialize)]
pub struct FindingDto {
    pub lint: &'static str,
    pub severity: &'static str,
    pub node_id: u64,
    pub message: String,
    pub snippet: String,
}

#[derive(Serialize)]
pub struct InspectResponse {
    pub tree_hex: String,
    pub address: String,
    pub source: String,
    pub completeness: &'static str,
    pub raw_placeholders: usize,
    pub truncated: bool,
    pub findings: Vec<FindingDto>,
}

impl FindingDto {
    pub fn from_engine(f: &ergo_sandbox::Finding) -> Self {
        Self {
            lint: f.lint,
            severity: match f.severity {
                ergo_sandbox::Severity::High => "high",
                ergo_sandbox::Severity::Medium => "medium",
                ergo_sandbox::Severity::Low => "low",
            },
            node_id: f.node_id,
            message: f.message.clone(),
            snippet: f.snippet.clone(),
        }
    }
}

/// Split an `Audit` into the flat wire shape.
pub fn completeness_parts(a: &Audit) -> (&'static str, usize, bool) {
    match a.completeness {
        Completeness::Complete => ("complete", 0, false),
        Completeness::Partial {
            raw_placeholders,
            truncated,
        } => ("partial", raw_placeholders, truncated),
    }
}
