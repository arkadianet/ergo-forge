//! Conservative presentation grouping, never an additional detector or discharge.
use super::{children, Finding, ReviewPriority};
use crate::{Lifted, Node, NodeKind};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorDischarge {
    pub node_id: u64,
    pub evidence: super::DischargeEvidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Obligation {
    pub key: String,
    pub category: String,
    pub review_priority: ReviewPriority,
    pub status: &'static str,
    /// Full original observations, including reproduction and every IR anchor.
    pub anchors: Vec<Finding>,
    pub discharges: Vec<AnchorDischarge>,
}

fn find(n: &Node, id: u64) -> Option<&Node> {
    if n.id == id {
        return Some(n);
    }
    children(n).into_iter().find_map(|n| find(n, id))
}

// Only literal context variables are grouped: same variable and type in the
// same script means the same witness-schema obligation. No alias/delegation
// inference, printed-snippet equality, or grouping of unknown expressions.
#[must_use]
pub fn group(lifted: &Lifted, findings: &[Finding]) -> Vec<Obligation> {
    let mut groups: BTreeMap<String, Obligation> = BTreeMap::new();
    for (occurrence, f) in findings.iter().enumerate() {
        let receiver = find(&lifted.node, f.node_id).and_then(|n| match &n.kind {
            NodeKind::Method(recv, method, args)
                if f.lint == "unchecked-get" && method == "get" && args.is_empty() =>
            {
                if let NodeKind::GetVar(id, ty) = &recv.kind {
                    (!ty.is_empty() && (0..=255).contains(id))
                        .then(|| format!("witness-schema:{id}:{ty}"))
                } else {
                    None
                }
            }
            _ => None,
        });
        let key = format!(
            "obligation-v1:{}:{}",
            f.lint,
            receiver.unwrap_or_else(|| format!("unrecognized:{}:{occurrence}", f.node_id))
        );
        let g = groups.entry(key.clone()).or_insert_with(|| Obligation {
            key,
            category: f.lint.into(),
            review_priority: f.severity,
            status: "unresolved",
            anchors: vec![],
            discharges: vec![],
        });
        g.review_priority = g.review_priority.min(f.severity);
        g.anchors.push(f.clone());
    }
    groups.into_values().collect()
}
