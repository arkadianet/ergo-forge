//! The audit layer: static lints over the lifted AST.
//!
//! Lints run on the tree the decompiler recovers, so the same lint serves
//! both authored source (compile, then lift) and a contract pasted from
//! chain. See `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.

pub mod boxrefs;
pub mod context;
pub mod coverage;
pub mod flow;
pub use context::{
    audit_with_contracts, ContextAudit, ContextFinding, ContractSet, DischargeEvidence, Execution,
    FindingStatus, InputContract,
};
pub mod finding;
pub mod lints;
pub mod obligation;
pub mod triage;
pub mod visit;

pub use finding::{bounded, snippet, Finding, ReviewPriority, Severity, SNIPPET_MAX};
pub use visit::children;

use crate::{Lifted, Node};

/// One registered static lint: the id the rest of the tooling names it by, and
/// the pass that runs it.
#[derive(Debug, Clone, Copy)]
pub struct Lint {
    /// Stable id, the same string this pass stamps on every [`Finding`].
    pub id: &'static str,
    /// The pass itself. A lint runs over a [`Lifted`] node, never over raw
    /// bytes, so authored and chain-recovered code take the same path.
    pub run: fn(&Node) -> Vec<Finding>,
}

impl Lint {
    const fn new(id: &'static str, run: fn(&Node) -> Vec<Finding>) -> Self {
        Self { id, run }
    }
}

/// Every lint [`audit`] runs, applied in order. Findings are sorted afterwards.
///
/// This table is the only record of which lints the static audit runs. A lint
/// absent from it did not run, whatever a document, catalogue or observation
/// calls it, so instrumentation that reports availability must read it here
/// rather than keep its own list.
pub const LINTS: &[Lint] = &[
    Lint::new("unchecked-get", lints::unchecked_get),
    Lint::new("unbound-box-reserves", lints::unbound_box_reserves),
    Lint::new("delegated-reserves", lints::delegated_reserves),
    Lint::new("height-guards", lints::height_guards),
    Lint::new("unconstrained-outputs", lints::unconstrained_outputs),
    Lint::new("successor-field-drift", lints::successor_field_drift),
    Lint::new("trivial-sigma-branch", lints::trivial_sigma_branch),
    Lint::new(
        "unauthenticated-code-execution",
        lints::unauthenticated_code_execution,
    ),
    Lint::new("trust-assumptions", lints::trust_assumptions),
    Lint::new("upgrade-hook", lints::upgrade_hook),
];

/// The registered lint ids, in run order.
pub fn registered_lints() -> impl Iterator<Item = &'static str> {
    LINTS.iter().map(|lint| lint.id)
}

/// Whether `lint` is one of the lints [`audit`] runs.
#[must_use]
pub fn is_registered_lint(lint: &str) -> bool {
    LINTS.iter().any(|registered| registered.id == lint)
}

/// Recovery coverage of the lifted representation, not property/audit completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Completeness {
    /// Every construct lifted; the fixed static observations ran over that recovery.
    Complete,
    /// The lift left raw placeholders or hit the depth ceiling. Part of the
    /// contract was not analysed — absence of findings proves nothing.
    Partial {
        raw_placeholders: usize,
        truncated: bool,
    },
}

impl Completeness {
    /// The recovery state as a stable lower-case wire label, `"complete"` or
    /// `"partial"`, the spelling `inspect` and the checklist already use.
    ///
    /// Reporting surfaces carry this label instead of the enum itself, because
    /// the enum's own serialization would spell the same fact a second,
    /// conflicting way inside one response.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial { .. } => "partial",
        }
    }
}

/// The result of auditing one lifted tree.
#[derive(Debug, Clone)]
pub struct Audit {
    pub obligations: Vec<obligation::Obligation>,
    /// Sorted most-severe first, then by node id — deterministic output.
    pub findings: Vec<Finding>,
    pub completeness: Completeness,
}

/// Run the bounded spender-controlled flow pass and attach IR anchors.
///
/// This is separate from [`audit`] so existing absence-lint result sets stay
/// stable; callers that want the review obligations can request them
/// explicitly.
#[must_use]
pub fn flow_findings(lifted: &Lifted) -> Vec<Finding> {
    lints::flow_paths(&lifted.node)
        .into_iter()
        .map(|mut finding| {
            finding.ir_id = lifted.ir_ids.get(&finding.node_id).copied();
            finding
        })
        .collect()
}

/// Run every lint over a lifted tree.
///
/// Total: cannot fail. Malformed input was rejected earlier, at `parse_tree`.
#[must_use]
pub fn audit(lifted: &Lifted) -> Audit {
    let mut findings: Vec<Finding> = LINTS
        .iter()
        .flat_map(|lint| (lint.run)(&lifted.node))
        .collect();
    for f in &mut findings {
        f.ir_id = lifted.ir_ids.get(&f.node_id).copied();
    }
    findings.sort_by_key(|f| (f.severity, f.node_id));
    Audit {
        obligations: obligation::group(lifted, &findings),
        findings,
        completeness: if lifted.raw_placeholders == 0 && !lifted.truncated {
            Completeness::Complete
        } else {
            Completeness::Partial {
                raw_placeholders: lifted.raw_placeholders,
                truncated: lifted.truncated,
            }
        },
    }
}
