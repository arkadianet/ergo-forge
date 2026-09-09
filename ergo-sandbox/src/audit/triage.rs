//! Reproducible, bounded confirmation of a finding in a declared contract set.
use serde::{Deserialize, Serialize};

use crate::drain::{drain_hunt, DrainReport, DrainRequest, DrainRole, DrainVerdict};
use crate::txcheck::{check, TxCheck};

pub const BOUNDED_WARNING: &str =
    "Absence of a result under a bound is not evidence of absence. Not-reproduced does not mean safe.";

/// Fields are private: only a completed local hunt can create confirmation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Triage {
    state: &'static str,
    consensus_reducer_consulted: bool,
    explanation: &'static str,
    evidence: TriageEvidence,
}

impl Default for Triage {
    fn default() -> Self {
        Self {
            state: "unconfirmed",
            consensus_reducer_consulted: false,
            explanation: "Static finding only; the consensus reducer has not been consulted.",
            evidence: TriageEvidence::StaticOnly,
        }
    }
}

impl Triage {
    pub fn state(&self) -> &'static str {
        self.state
    }

    pub fn evidence(&self) -> Option<&Evidence> {
        match &self.evidence {
            TriageEvidence::StaticOnly => None,
            TriageEvidence::DrainHunt(evidence) => Some(evidence),
        }
    }
}

/// Static-only evidence explicitly records the analysis method; the enclosing
/// finding carries its lint, node, message and source snippet.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum TriageEvidence {
    StaticOnly,
    DrainHunt(Box<Evidence>),
}

/// The exact inputs, objective, budget, traversal and accepted witness.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub request: TriageRequest,
    pub hunt: DrainReport,
    pub reducer_verdict: Option<TxCheck>,
}

/// Select one real static finding on a protected/companion spending input.
/// The full request embeds the contract bytes; no network lookup is involved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriageRequest {
    pub input_index: usize,
    pub lint: String,
    pub node_id: u64,
    pub drain: DrainRequest,
}

/// Re-audit the selected contract so stale/nonexistent finding anchors fail.
/// Confirmation is scoped to this contract set and objective, not a proof
/// that the selected lint is the unique cause of the extraction.
pub fn triage(req: &TriageRequest) -> Result<super::Finding, String> {
    let input = req
        .drain
        .inputs
        .get(req.input_index)
        .ok_or("finding input is out of range")?;
    if !matches!(input.role, DrainRole::Protected | DrainRole::Companion) {
        return Err("finding input must be protected or companion".into());
    }
    let bytes = hex::decode(
        input
            .box_
            .ergo_tree
            .as_deref()
            .ok_or("finding input needs ergoTree")?,
    )
    .map_err(|e| e.to_string())?;
    let tree = crate::inspect::parse_tree(&bytes).map_err(|e| e.to_string())?;
    let lifted = crate::lift_tree(&tree, false);
    let mut finding = super::audit(&lifted)
        .findings
        .into_iter()
        .find(|f| f.lint == req.lint && f.node_id == req.node_id)
        .ok_or("finding anchor does not exist on the selected contract")?;
    let hunt = drain_hunt(&req.drain).map_err(|e| e.to_string())?;
    let reducer_verdict = if matches!(hunt.verdict, DrainVerdict::Drainable) {
        let hit = hunt
            .best
            .as_ref()
            .ok_or("drainable report has no witness")?;
        let verdict = check(&hit.witness.tx_request).map_err(|e| e.to_string())?;
        if !verdict.valid {
            return Err("drain witness failed reducer replay".into());
        }
        Some(verdict)
    } else {
        None
    };
    let (state, explanation) = match hunt.verdict {
        DrainVerdict::Drainable => ("confirmed",
            "This contract set reached the declared extraction objective. This does not establish that this lint alone caused it."),
        DrainVerdict::NotUnderProbes if hunt.oracle_calls > 0 => ("not-reproduced", BOUNDED_WARNING),
        _ => ("unconfirmed", "The hunt did not produce a usable bounded verdict; inspect its notes and objective."),
    };
    finding.triage = Triage {
        state,
        consensus_reducer_consulted: hunt.oracle_calls > 0,
        explanation,
        evidence: TriageEvidence::DrainHunt(Box::new(Evidence {
            request: req.clone(),
            hunt,
            reducer_verdict,
        })),
    };
    Ok(finding)
}
