//! Untrusted exact-tree derivation proposals. Discovery IDs have no authority here.
use super::relations::RelationProposal;
use ergo_ser::opcode::{node_opcode, preorder};
use serde::{Deserialize, Serialize};

/// Exact root bytes are in P; node IDs are the pinned parser's preorder IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Anchor {
    pub node: u64,
    pub opcode: u8,
    /// Proposed structural rule; the checker rechecks this against exact bytes.
    pub rule: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Derivation {
    pub version: String,
    pub claim_digest: String,
    pub anchors: Vec<Anchor>,
    /// Direct rules only: dependencies are not accepted as premises.
    pub dependencies: Vec<String>,
}
/// This nominates the exact nodes for independent structural checking. It does
/// not infer necessity, consult discovery, or serialize an established status.
pub fn propose(claim: &RelationProposal) -> Result<Derivation, String> {
    let bytes = hex::decode(claim.premises.root_bytes.value().ok_or("missing root")?)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("root byte cap".into());
    }
    let tree = crate::inspect::parse_tree(&bytes).map_err(|e| e.to_string())?;
    let anchors: Vec<_> = preorder(&tree.body)
        .take(10001)
        .map(|(node, e)| Anchor {
            node,
            opcode: node_opcode(e),
            rule: proposed_rule(node_opcode(e)).into(),
        })
        .collect();
    if anchors.len() > 10000 {
        return Err("node cap".into());
    }
    Ok(Derivation {
        version: "necessity-derivation:v1".into(),
        claim_digest: claim.claim_digest(),
        anchors,
        dependencies: vec![],
    })
}

fn proposed_rule(opcode: u8) -> &'static str {
    match opcode {
        0xD8 => "scoped-bindings",
        0xD1 => "required-boolean-root",
        0xED => "conjunction",
        0xEC => "common-alternatives",
        0x95 => "guarded-alternatives",
        0x93 => "positive-equality",
        0x94 => "distinct-identity",
        0xAE => "input-existential",
        _ => "operand-or-no-inference",
    }
}
