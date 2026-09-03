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
#[serde(rename_all = "camelCase")]
pub struct FindingDto {
    pub lint: &'static str,
    pub severity: &'static str,
    pub node_id: u64,
    pub message: String,
    pub snippet: String,
    /// Byte offset into the authored source (compile route only, when the
    /// compiler's source map cites the node). Absent for on-chain reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectResponse {
    /// Storage rent for a minimal box under this contract.
    pub rent: ergo_sandbox::rent::RentEstimate,
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
            offset: None,
            line: None,
            col: None,
        }
    }

    /// Attach a source position from the compiler's map.
    pub fn with_position(mut self, source: &str, offset: u32) -> Self {
        let (line, col) = ergo_compiler::span::line_col(source, offset);
        self.offset = Some(offset);
        self.line = Some(line);
        self.col = Some(col);
        self
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
    /// Read-only data inputs, scenario-JSON box shape (each needs `ergoTree`).
    #[serde(default)]
    pub data_inputs: Vec<ergo_sandbox::ScenarioBox>,
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
    /// Storage rent for the box hunted (the supplied `selfBox` when given,
    /// else a minimal box), with the next collection height when the box's
    /// creation height is known.
    pub rent: ergo_sandbox::rent::RentEstimate,
    pub tree_hex: String,
    pub address: String,
    pub verdict: &'static str,
    pub residuals: Vec<String>,
    pub self_synthetic: bool,
    pub probes: Vec<ProbeDto>,
}

impl HuntResponse {
    pub fn from_engine(
        tree_hex: String,
        address: String,
        h: &Hunt,
        rent: ergo_sandbox::rent::RentEstimate,
    ) -> Self {
        Self {
            rent,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileRequest {
    pub source: String,
    #[serde(default)]
    pub network: Option<String>,
    /// Tree version for compilation (default 3).
    #[serde(default)]
    pub tree_version: Option<u8>,
    /// Compile-time constants: name → typed value.
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, ergo_sandbox::TypedValue>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamStatus {
    pub name: String,
    pub type_hint: Option<String>,
    pub default: Option<String>,
    pub supplied: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResponse {
    /// Storage rent for a minimal box under this contract.
    pub rent: ergo_sandbox::rent::RentEstimate,
    pub tree_hex: String,
    pub p2s: String,
    pub p2sh: String,
    /// The tree decompiled back to source: what consensus will run.
    pub source: String,
    pub completeness: &'static str,
    pub raw_placeholders: usize,
    pub truncated: bool,
    pub findings: Vec<FindingDto>,
    pub params: Vec<ParamStatus>,
    /// True when the source was an EIP-5 `@contract def` template,
    /// instantiated with the given parameters.
    pub template: bool,
    /// True when the compiler's source map aligned with the tree and
    /// findings could be positioned. False for templates (no map yet).
    pub positioned: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleSummary {
    pub id: String,
    pub group: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleDto {
    pub id: String,
    pub group: String,
    pub name: String,
    pub source: String,
    pub params: Vec<ergo_sandbox::compile::ParamNeed>,
    /// True for EIP-5 `@contract def` templates.
    pub template: bool,
    /// The template's name and doc-block description (templates only).
    pub doc: Option<ergo_sandbox::compile::TemplateDoc>,
}

/// Rent for a scenario box under `tree_bytes` (or a minimal box when none).
pub fn rent_for(
    tree_bytes: &[u8],
    b: Option<&ergo_sandbox::ScenarioBox>,
) -> ergo_sandbox::rent::RentEstimate {
    match b {
        Some(b) => {
            let amounts: Vec<u64> = b.tokens.iter().map(|t| t.amount).collect();
            // Each register's canonical serialized constant (type + value),
            // which is what the box serialization carries.
            let regs: Vec<Vec<u8>> = b
                .registers
                .values()
                .map(|tv| match (tv.r#type.as_str(), tv.value.as_str()) {
                    ("raw", Some(h)) => hex::decode(h).unwrap_or_default(),
                    _ => ergo_sandbox::parse_typed_value(&tv.r#type, &tv.value)
                        .ok()
                        .and_then(|(t, v)| {
                            let mut w = ergo_primitives::writer::VlqWriter::new();
                            ergo_ser::sigma_value::write_constant(&mut w, &t, &v)
                                .ok()
                                .map(|_| w.result())
                        })
                        .unwrap_or_default(),
                })
                .collect();
            let created = if b.creation_height > 0 {
                Some(b.creation_height)
            } else {
                None
            };
            ergo_sandbox::rent::estimate(tree_bytes, &amounts, &regs, created)
        }
        None => ergo_sandbox::rent::estimate(tree_bytes, &[], &[], None),
    }
}
