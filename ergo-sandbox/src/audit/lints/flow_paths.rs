//! Review obligations for spender-chosen values reaching security-sensitive
//! sites, adapted from [`crate::audit::flow`].
//!
//! The flow engine decides *where a spender-chosen value arrives*, and this
//! adapter decides how to say it. Every finding is a review priority: a MED or
//! LOW line means "look here first", never that a path can be exploited, and
//! silence means the pattern was not recognised, not that the contract is safe.
//!
//! Each flow is reported once, anchored to its sink site, and named by both
//! ends: the spender-controlled source and what the sink does with it. The
//! source and sink names are the analysis's own vocabulary, not the lift's
//! synthetic `val` names, so a finding reads without a source map.
//!
//! Where the analysis hit a bound, the message says so: absence of further
//! flows under a bound is not evidence of absence.

use crate::audit::flow::{self, Flow, SinkKind};
use crate::audit::{bounded, children, snippet, Finding, Severity};
use crate::Node;

/// Report spender-controlled values reaching security-sensitive sites.
#[must_use]
pub fn flow_paths(root: &Node) -> Vec<Finding> {
    let report = flow::analyze(root);
    let limit_note = report.limits.note();
    report
        .flows
        .iter()
        .map(|observed| Finding {
            triage: Default::default(),
            lint: "flow-paths",
            severity: review_priority(observed.kind),
            node_id: observed.site_id,
            // Filled by `audit()` from the lift's IR map; a direct lint call has
            // no IR walk of its own and must not invent one.
            ir_id: None,
            message: message(observed, limit_note.as_deref()),
            snippet: site_snippet(root, observed)
                .unwrap_or_else(|| bounded(format!("{} ← {}", observed.sink, observed.source))),
        })
        .collect()
}

/// Review priority follows what the sink decides, not an estimate of impact.
///
/// Code bytes and reserve bounds are the two places a spender-chosen value can
/// change what the script means; identity, ordering and index selection are
/// reported for review but are routinely written on purpose.
fn review_priority(kind: SinkKind) -> Severity {
    match kind {
        SinkKind::CodeBytes | SinkKind::ReserveGuard => Severity::Medium,
        SinkKind::IdentityGuard | SinkKind::SelfGuard | SinkKind::CollectionIndex => Severity::Low,
        SinkKind::Unresolved => Severity::Low,
    }
}

/// What arrived, what to review, what this is not, and whether a bound applied.
fn message(observed: &Flow, bounded: Option<&str>) -> String {
    let Flow {
        kind,
        sink,
        source,
        bounded: flow_cut,
        ..
    } = observed;
    let observed_text = match kind {
        SinkKind::CodeBytes => format!(
            "{sink} are built from a spender-supplied value ({source}). Review where those bytes \
             come from and what authorises them; dynamic code may be the contract's whole design, \
             and this syntax observation is not evidence of exploitability."
        ),
        SinkKind::ReserveGuard => format!(
            "{sink} is compared or ordered against a spender-supplied value ({source}). A bound \
             the spender can move is ordinary in withdrawal-style contracts. Review the direction \
             and sign, any independent cap (a token amount, a companion contract) and the other \
             conditions on this path; this syntax observation is not evidence of exploitability."
        ),
        SinkKind::IdentityGuard => format!(
            "A spender-supplied value ({source}) is the other side of an equality with {sink}. \
             Equalities of this shape are commonly intentional — a successor script, token or box \
             identity check. Review which other requirements constrain it; this syntax observation \
             is not evidence of exploitability."
        ),
        SinkKind::SelfGuard => format!(
            "A spender-supplied value ({source}) is ordered against {sink}, which the box being \
             spent fixes. Review whether that direction and sign are what the author intended; \
             this syntax observation is not evidence of exploitability."
        ),
        SinkKind::CollectionIndex => format!(
            "A spender-supplied value ({source}) selects which element of {sink} the script goes \
             on to reason about. Computed and searched indices are common. Review what the selected \
             element is then allowed to prove; this syntax observation is not evidence of \
             exploitability."
        ),
        SinkKind::Unresolved => format!(
            "The analysis reached its hard tree bound before it could name a security-sensitive \
             site; the source behind {source} and the sink remain unresolved. Review the tree with a \
             larger explicit bound; this syntax observation is not evidence of exploitability."
        ),
    };
    let bounded_text = match (*flow_cut, bounded) {
        (true, Some(note)) => format!(
            " Bounded: the source behind this path was not named within the analysis bound \
             ({note}), so the value named here may understate what reaches the sink."
        ),
        (true, None) => concat!(
            " Bounded: the source behind this path was not named within the analysis bound, so ",
            "the value named here may understate what reaches the sink."
        )
        .to_owned(),
        (false, Some(note)) => {
            format!(" Bounded: this analysis reached a ceiling ({note}); it is a lower bound.")
        }
        (false, None) => String::new(),
    };
    format!("{observed_text}{bounded_text}")
}

/// The sink site's source rendering, so the finding reads without a source map.
///
/// Falls back to the flow's own two ends when the id does not resolve, which
/// keeps the finding printable even if a caller hands in an inconsistent tree.
fn site_snippet(root: &Node, observed: &Flow) -> Option<String> {
    node_at(root, observed.site_id).map(snippet)
}

/// The node a flow anchors to. Lift ids are unique within one decompilation.
fn node_at(n: &Node, id: u64) -> Option<&Node> {
    let mut stack = vec![n];
    while let Some(node) = stack.pop() {
        if node.id == id {
            return Some(node);
        }
        stack.extend(children(node));
    }
    None
}
