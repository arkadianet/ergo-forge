//! Wire types. Deliberately separate from the engine's types: the API is a
//! versioned contract and the engine must stay free to change.

use ergo_sandbox::audit::{Audit, Completeness};
use ergo_sandbox::hunt::{Hunt, HuntVerdict, OutputShape};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntRequest {
    pub input: String,
    #[serde(default)]
    pub network: Option<String>,
    /// Base spending height; default near the mainnet tip.
    #[serde(default)]
    pub height: Option<u32>,
    /// The box being spent, in the scenario-JSON box shape. Without it SELF
    /// is synthetic and the response says so.
    #[serde(default)]
    pub self_box: Option<ergo_sandbox::ScenarioBox>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeDto {
    pub height: u32,
    pub output: &'static str,
    pub verdict: &'static str,
    pub reduced_to: Option<String>,
    pub error: Option<String>,
    pub cost: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntResponse {
    pub tree_hex: String,
    pub address: String,
    pub verdict: &'static str,
    pub residuals: Vec<String>,
    pub self_synthetic: bool,
    pub probes: Vec<ProbeDto>,
}

impl HuntResponse {
    pub fn from_engine(tree_hex: String, address: String, h: &Hunt) -> Self {
        Self {
            tree_hex,
            address,
            verdict: match h.verdict {
                HuntVerdict::SpendableByAnyone => "spendableByAnyone",
                HuntVerdict::MovableByAnyone => "movableByAnyone",
                HuntVerdict::RequiresProof => "requiresProof",
                HuntVerdict::NotUnderProbes => "notUnderProbes",
            },
            residuals: h.residuals.clone(),
            self_synthetic: h.self_synthetic,
            probes: h
                .probes
                .iter()
                .map(|p| ProbeDto {
                    height: p.height,
                    output: match p.output {
                        OutputShape::Attacker => "attacker",
                        OutputShape::Preserve => "preserve",
                    },
                    verdict: verdict_str(p.verdict),
                    reduced_to: p.reduced_to.clone(),
                    error: p.error.clone(),
                    cost: p.cost,
                })
                .collect(),
        }
    }
}

/// `POST /api/v1/eval` response: the sandbox outcome in wire form. The
/// request body is the scenario JSON itself (`ergo_sandbox::Scenario`),
/// which is already the workbench's wire model.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalResponse {
    pub verdict: &'static str,
    pub error: Option<String>,
    pub cost: u64,
    pub cost_limit: u64,
    pub reduced_to: Option<String>,
    pub trace: Vec<TraceDto>,
    pub tree_hex: String,
    pub address: String,
}

#[derive(Serialize)]
pub struct TraceDto {
    pub label: String,
    pub value: String,
}

pub fn verdict_str(v: ergo_sandbox::Verdict) -> &'static str {
    match v {
        ergo_sandbox::Verdict::Pass => "pass",
        ergo_sandbox::Verdict::Fail => "fail",
        ergo_sandbox::Verdict::Error => "error",
        ergo_sandbox::Verdict::NeedsProof => "needsProof",
        ergo_sandbox::Verdict::ProofAccepted => "proofAccepted",
        ergo_sandbox::Verdict::ProofRejected => "proofRejected",
    }
}

impl EvalResponse {
    pub fn from_engine(o: ergo_sandbox::EvalOutcome) -> Self {
        Self {
            verdict: verdict_str(o.verdict),
            error: o.error,
            cost: o.cost,
            cost_limit: o.cost_limit,
            reduced_to: o.reduced_to,
            trace: o
                .trace
                .into_iter()
                .map(|t| TraceDto {
                    label: t.label,
                    value: t.value,
                })
                .collect(),
            tree_hex: o.tree_hex,
            address: o.p2s_address,
        }
    }
}
