//! What a lint reports.

use crate::Node;
use serde::Serialize;

/// Longest rendered snippet carried on a finding; longer ones are cut with a
/// trailing `…`. Keeps a finding printable on one terminal line.
pub const SNIPPET_MAX: usize = 120;

/// Static review priority; context and execution evidence determine any actual harm.
///
/// Ordering matters: variants are declared most-severe first so `as u8`
/// sorts findings correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ReviewPriority {
    /// Review first; failure/locking has not been established.
    High,
    /// Suspicious or fragile; may be intentional.
    Medium,
    /// Informational.
    Low,
}

/// Compatibility name for the original static ranking; never property impact.
pub type Severity = ReviewPriority;

impl ReviewPriority {
    /// Uppercase label for CLI output.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Severity::High => "HIGH",
            Severity::Medium => "MED",
            Severity::Low => "LOW",
        }
    }
}

/// One lint result, anchored to a node in the lifted tree.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Recorded scenario reproduction, initially static-only; never node validation.
    pub triage: super::triage::Triage,
    /// Stable machine-readable lint id, e.g. `"unchecked-get"`.
    pub lint: &'static str,
    /// Legacy compatibility field: static review priority, never claim impact.
    pub severity: Severity,
    /// `Node::id` of the offending node. Lift-local — see `ast::Node::id`.
    pub node_id: u64,
    /// The offending node's id in the shared IR walk
    /// (`ergo_ser::opcode::preorder`), when it stands for an IR node. This is
    /// what a compiler `SourceMap` is keyed by. Filled by `audit()`.
    pub ir_id: Option<u64>,
    /// One sentence, specific to this occurrence.
    pub message: String,
    /// The offending subtree rendered back to source, so the finding reads
    /// without a source map or the original source.
    pub snippet: String,
}

// Keep detector construction unchanged; attach authority labels at serialization.
impl Serialize for Finding {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut record = serializer.serialize_map(None)?;
        record.serialize_entry("method", "static-analysis")?;
        record.serialize_entry(
            "provenance",
            "supplied-code; deployment identity not established",
        )?;
        record.serialize_entry("nodeValidated", &false)?;
        record.serialize_entry("severityMeaning", "review-priority")?;
        record.serialize_entry("triage", &self.triage)?;
        record.serialize_entry("lint", self.lint)?;
        record.serialize_entry("severity", &self.severity)?;
        record.serialize_entry("reviewPriority", &self.severity)?;
        record.serialize_entry("node_id", &self.node_id)?;
        record.serialize_entry("ir_id", &self.ir_id)?;
        record.serialize_entry("message", &self.message)?;
        record.serialize_entry("snippet", &self.snippet)?;
        record.end()
    }
}

/// Render `n` as a one-line snippet, collapsed and length-capped.
#[must_use]
pub fn snippet(n: &Node) -> String {
    let mut s = crate::decompile::print(n);
    if s.contains('\n') {
        s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    if s.chars().count() > SNIPPET_MAX {
        s = s.chars().take(SNIPPET_MAX - 1).collect::<String>() + "…";
    }
    s
}
