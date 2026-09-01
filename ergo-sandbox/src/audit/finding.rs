//! What a lint reports.

use crate::Node;

/// Longest rendered snippet carried on a finding; longer ones are cut with a
/// trailing `…`. Keeps a finding printable on one terminal line.
pub const SNIPPET_MAX: usize = 120;

/// How much a finding should alarm a reader.
///
/// Ordering matters: variants are declared most-severe first so `as u8`
/// sorts findings correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Can cause the script to fail at validation, locking the box.
    High,
    /// Suspicious or fragile; may be intentional.
    Medium,
    /// Informational.
    Low,
}

impl Severity {
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
    /// Stable machine-readable lint id, e.g. `"unchecked-get"`.
    pub lint: &'static str,
    pub severity: Severity,
    /// `Node::id` of the offending node. Lift-local — see `ast::Node::id`.
    pub node_id: u64,
    /// One sentence, specific to this occurrence.
    pub message: String,
    /// The offending subtree rendered back to source, so the finding reads
    /// without a source map or the original source.
    pub snippet: String,
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
