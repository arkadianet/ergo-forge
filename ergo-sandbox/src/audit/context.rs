//! Evidence-bearing audit of an explicitly supplied transaction contract set.
//!
//! Protocol-map reachability does not prove co-execution. Callers must resolve
//! the map to the spending scripts of a particular transaction before using
//! this API. Results are conditional on that declared set, not a verdict on
//! every possible transaction spending the audited contract.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::boxrefs::{box_key, collect_vals, deref, positional_key, reserve_read, Vals};
use super::{audit, children, Completeness, Finding};
use crate::map::refs::{required_bindings, Binding, Cover};
use crate::{Lifted, Node, NodeKind};

/// Where a script executes. Extensions share their spending input's SELF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Execution {
    SpendingInput(usize),
    ContextExtension { input_index: usize, variable: u8 },
}

/// A named script required to succeed in the declared transaction. Data-input
/// scripts do not execute and MUST NOT be included. Resolved extensions may be
/// included only when the caller has established that their execution is required.
#[derive(Debug, Clone, Copy)]
pub struct InputContract<'a> {
    pub name: &'a str,
    pub execution: Execution,
    pub lifted: &'a Lifted,
}

/// An explicit co-execution premise, supplied by the transaction's caller.
/// `complete` asserts all spending scripts are included; a partial set never
/// discharges anything. A discovered map alone cannot establish this premise.
pub struct ContractSet<'a> {
    pub inputs: &'a [InputContract<'a>],
    pub complete: bool,
    /// Token id -> archived singleton-emission evidence. Mere names, constants,
    /// and current box amounts are not evidence of a singleton supply.
    pub singleton_tokens: &'a BTreeMap<String, String>,
}

/// A required identity check, attributed to the companion that enforces it.
#[derive(Debug, Clone, Serialize)]
pub struct DischargeEvidence {
    pub reason: &'static str,
    pub companion: String,
    pub companion_execution: Execution,
    pub singleton_evidence: Option<String>,
    pub site: String,
    pub binding: Binding,
    /// NFT constant, script digest, or `SELF.propositionBytes`.
    pub identity: String,
    pub covers: BTreeSet<Cover>,
    pub binding_node_id: u64,
    pub binding_ir_id: Option<u64>,
}

/// Findings remain intact, including their original severity and anchors.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "evidence", rename_all = "camelCase")]
pub enum FindingStatus {
    Active,
    Discharged(DischargeEvidence),
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextFinding {
    #[serde(flatten)]
    pub finding: Finding,
    #[serde(flatten)]
    pub status: FindingStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextAudit {
    pub target_recovered_code: String,
    pub obligations: Vec<super::obligation::Obligation>,
    /// Entire supplied co-execution premise, including unresolved exact identity.
    pub premises: serde_json::Value,
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    pub findings: Vec<ContextFinding>,
    pub completeness: Completeness,
    /// False for incomplete, ambiguous, or mismatched input sets.
    pub context_usable: bool,
}

/// Run the unchanged local detectors, then annotate reserve/provenance findings
/// only where another executing contract necessarily pins the exact literal
/// transaction slot. Context variables are local to each input, so equal
/// printed `getVar` expressions are deliberately never matched.
#[must_use]
pub fn audit_with_contracts(lifted: &Lifted, set: &ContractSet<'_>) -> ContextAudit {
    let local = audit(lifted);
    let mut names = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let usable = set.complete
        && local.completeness == Completeness::Complete
        && execution_layout_valid(set.inputs)
        && set
            .inputs
            .iter()
            .filter(|c| std::ptr::eq(c.lifted, lifted))
            .count()
            == 1
        && set.inputs.iter().all(|c| {
            !c.name.is_empty()
                && names.insert(c.name)
                && indices.insert(c.execution)
                && c.lifted.raw_placeholders == 0
                && !c.lifted.truncated
        });
    let mut companions: Vec<_> = if usable {
        set.inputs
            .iter()
            .filter(|c| !std::ptr::eq(c.lifted, lifted))
            .collect()
    } else {
        Vec::new()
    };
    companions.sort_by_key(|c| (c.execution, c.name));
    let proofs: Vec<_> = companions
        .iter()
        .map(|c| (c, required_bindings(&c.lifted.node)))
        .collect();
    let mut vals = Vals::new();
    collect_vals(&lifted.node, &mut vals);
    let findings: Vec<ContextFinding> = local
        .findings
        .into_iter()
        .map(|finding| {
            let mut status = FindingStatus::Active;
            if matches!(finding.lint, "unbound-box-reserves" | "trust-assumptions") {
                let site = find_node(&lifted.node, finding.node_id)
                    .and_then(|n| finding_site(n, &vals, finding.lint))
                    .filter(|s| literal_slot(s));
                if let Some(site) = site {
                    for (companion, bindings) in &proofs {
                        if let Some(proof) = bindings.iter().find(|p| {
                            p.site == site
                                && (finding.lint != "trust-assumptions"
                                    || p.binding == Binding::Nft)
                                && (p.binding != Binding::Nft
                                    || set
                                        .singleton_tokens
                                        .get(&p.identity)
                                        .is_some_and(|e| !e.is_empty()))
                        }) {
                            status = FindingStatus::Discharged(DischargeEvidence {
                                reason: "required companion binds this literal slot under supplied co-execution",
                                companion: companion.name.into(),
                                companion_execution: companion.execution,
                                singleton_evidence: set
                                    .singleton_tokens
                                    .get(&proof.identity)
                                    .cloned(),
                                site,
                                binding: proof.binding,
                                identity: proof.identity.clone(),
                                covers: [match proof.binding {
                                    Binding::Nft => Cover::TokenId,
                                    _ => Cover::Script,
                                }]
                                .into(),
                                binding_node_id: proof.node_id,
                                binding_ir_id: companion.lifted.ir_ids.get(&proof.node_id).copied(),
                            });
                            break;
                        }
                    }
                }
            }
            ContextFinding { finding, status }
        })
        .collect();
    let mut obligations = local.obligations;
    for obligation in &mut obligations {
        for anchor in &obligation.anchors {
            if let Some(ContextFinding {
                status: FindingStatus::Discharged(evidence),
                ..
            }) = findings
                .iter()
                .find(|f| f.finding.node_id == anchor.node_id && f.finding.lint == anchor.lint)
            {
                obligation
                    .discharges
                    .push(super::obligation::AnchorDischarge {
                        node_id: anchor.node_id,
                        evidence: evidence.clone(),
                    });
            }
        }
        if obligation.discharges.len() == obligation.anchors.len() {
            obligation.status = "conditionally-discharged";
        }
    }
    let mut premise_inputs: Vec<_> = set.inputs.iter().collect();
    premise_inputs.sort_by_key(|c| (c.execution, c.name));
    ContextAudit {
        target_recovered_code: crate::decompile::print(&lifted.node),
        obligations,
        premises: serde_json::json!({
            "complete": set.complete,
            "scope": "conditional on caller-supplied required co-execution; not global safety",
            "exactDeploymentIdentity": "unknown; lifted code is not exact deployment evidence",
            "contracts": premise_inputs.iter().map(|c| serde_json::json!({
                "name": c.name, "execution": c.execution,
                "recoveredCode": crate::decompile::print(&c.lifted.node),
                "rawPlaceholders": c.lifted.raw_placeholders, "truncated": c.lifted.truncated,
            })).collect::<Vec<_>>(),
            "singletonTokens": set.singleton_tokens,
        }),
        claim: crate::claim::ClaimMetadata::STATIC,
        findings,
        completeness: local.completeness,
        context_usable: usable,
    }
}

fn literal_slot(site: &str) -> bool {
    ["INPUTS(", "OUTPUTS(", "CONTEXT.dataInputs("]
        .iter()
        .any(|prefix| {
            site.strip_prefix(prefix)
                .and_then(|s| s.strip_suffix(')'))
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        })
}

fn find_node(n: &Node, id: u64) -> Option<&Node> {
    if n.id == id {
        return Some(n);
    }
    children(n).into_iter().find_map(|c| find_node(c, id))
}

fn finding_site(n: &Node, vals: &Vals, lint: &str) -> Option<String> {
    if lint == "unbound-box-reserves" {
        return reserve_read(n, vals).and_then(|(b, _)| positional_key(b, vals));
    }
    if let NodeKind::Method(recv, name, args) = &n.kind {
        if (name == "get" && args.is_empty()) || (name == "getOrElse" && args.len() == 1) {
            if let NodeKind::Prop(b, _) = &deref(recv, vals).kind {
                return box_key(b, vals);
            }
        }
    }
    None
}

// Completeness is a caller premise, but reject internally inconsistent claims:
// input positions must be contiguous and extensions need their spending script.
fn execution_layout_valid(inputs: &[InputContract<'_>]) -> bool {
    let spending: BTreeSet<_> = inputs
        .iter()
        .filter_map(|c| match c.execution {
            Execution::SpendingInput(i) => Some(i),
            Execution::ContextExtension { .. } => None,
        })
        .collect();
    spending.iter().copied().eq(0..spending.len())
        && inputs.iter().all(|c| match c.execution {
            Execution::SpendingInput(_) => true,
            Execution::ContextExtension { input_index, .. } => spending.contains(&input_index),
        })
}
