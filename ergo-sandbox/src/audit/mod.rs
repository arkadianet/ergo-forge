//! The audit layer: static lints over the lifted AST.
//!
//! Lints run on the tree the decompiler recovers, so the same lint serves
//! both authored source (compile, then lift) and a contract pasted from
//! chain. See `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.

pub mod finding;
pub mod visit;

pub use finding::{snippet, Finding, Severity, SNIPPET_MAX};
pub use visit::children;

use crate::{Lifted, Node};

/// Every lint, applied in order. Findings are sorted afterwards.
const LINTS: &[fn(&Node) -> Vec<Finding>] = &[];

/// Whether the audit saw the whole contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completeness {
    /// Every construct lifted; findings cover the whole tree.
    Complete,
    /// The lift left raw placeholders or hit the depth ceiling. Part of the
    /// contract was not analysed — absence of findings proves nothing.
    Partial {
        raw_placeholders: usize,
        truncated: bool,
    },
}

/// The result of auditing one lifted tree.
#[derive(Debug, Clone)]
pub struct Audit {
    /// Sorted most-severe first, then by node id — deterministic output.
    pub findings: Vec<Finding>,
    pub completeness: Completeness,
}

/// Run every lint over a lifted tree.
///
/// Total: cannot fail. Malformed input was rejected earlier, at `parse_tree`.
#[must_use]
pub fn audit(lifted: &Lifted) -> Audit {
    let mut findings: Vec<Finding> = LINTS.iter().flat_map(|lint| lint(&lifted.node)).collect();
    findings.sort_by_key(|f| (f.severity, f.node_id));
    Audit {
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
