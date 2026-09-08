//! Reserves read off a box picked by **position**, with no NFT binding it.
//!
//! This is the class behind the 2026-09-08 USE/Dexy LP drain on mainnet. The
//! LP swap script read the pool box as `INPUTS(0)` and did the constant-
//! product math on `INPUTS(0).value` and `INPUTS(0).tokens(i)._2` — but
//! nothing in the script said *which* box that had to be. A position in the
//! input list is chosen by whoever builds the transaction, so an attacker
//! placed a decoy box (their own, with whatever reserves made the ratio
//! check pass) at index 0 and spent the real pool alongside it. The maths was
//! correct; it was simply applied to the wrong box.
//!
//! The fix, and what this lint looks for, is a **singleton NFT**: a token
//! minted in quantity one, kept in `tokens(0)` of the pool box, and asserted
//! by the script (`INPUTS(0).tokens(0)._1 == poolNft`). Because the token is
//! unique on chain, only the genuine box can satisfy it.
//!
//! A box is reported when all three hold:
//!
//! 1. it is obtained positionally — `INPUTS(n)`, `OUTPUTS(n)` or
//!    `CONTEXT.dataInputs(n)` for a literal or context-variable index,
//!    directly or through a `val`;
//! 2. its **reserves** — `.value`, or a token *amount* `.tokens(i)._2` — feed
//!    an arithmetic or comparison operator. A box merely passed along, or
//!    whose `propositionBytes` are compared, does not count;
//! 3. nowhere in the tree is it bound by NFT: no `tokens(0)._1 == <constant>`,
//!    and no `tokens(0)._1` equality with a box that is itself bound.
//!
//! Two shapes are exempt, because they are how a correct self-validating pool
//! is written:
//!
//! - **SELF** — never positional, so never reported. A script reading its own
//!   reserves is reading the box it guards.
//! - **SELF's successor** — an output whose `propositionBytes` (or
//!   `ergoTree`/`scriptBytes`) are asserted equal to `SELF`'s. The script
//!   itself runs again on that box, so it is not a substitutable decoy.
//!
//! Three spellings of the binding count: `b.tokens(0)._1 == x`, the whole pair
//! `b.tokens(0) == x`, and the whole collection `b.tokens == x.tokens`. All
//! three pin `tokens(0)`, which is where a singleton NFT is kept by convention.
//!
//! Known gaps (deliberate): an NFT held at a token index other than 0, and a
//! box identified by an `exists`/`forall` search over the inputs rather than by
//! a fixed index, are not recognised — such a contract is a false positive.
//! `val` names are collected across the whole tree rather than per block; lift
//! assigns them globally-unique names, so shadowing does not arise in practice.

use std::collections::HashSet;

use crate::audit::boxrefs::{
    box_key, collect_vals, is_value_op, mentions_positional, nft_slot_of, positional_key,
    reads_in_operand, script_of, Vals, OPERAND_DEPTH,
};
use crate::audit::{children, Finding, Severity};
use crate::{Node, NodeKind};

/// Report every positionally-obtained box whose reserves drive value maths
/// without an NFT binding it.
#[must_use]
pub fn unbound_box_reserves(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);

    let bound = bound_boxes(root, &vals);

    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    report(root, &vals, &bound, &mut seen, &mut out);
    out
}

// ── pass 1: what is already bound ────────────────────────────────────────────

/// Boxes the script pins down: NFT-bound ones, `SELF`, and `SELF`'s successor.
///
/// Collected over the whole tree — a binding asserted in one conjunct pins the
/// box for every read of it — then closed transitively, so
/// `A.tokens(0)._1 == B.tokens(0)._1` carries `B`'s binding to `A`.
fn bound_boxes(root: &Node, vals: &Vals) -> HashSet<String> {
    let mut bound: HashSet<String> = HashSet::new();
    bound.insert("SELF".into());
    let mut edges: Vec<(String, String)> = Vec::new();
    scan_bindings(root, vals, &mut bound, &mut edges);

    // Transitive closure over the token-id equalities.
    loop {
        let mut grew = false;
        for (a, b) in &edges {
            if bound.contains(b) && bound.insert(a.clone()) {
                grew = true;
            }
        }
        if !grew {
            return bound;
        }
    }
}

fn scan_bindings(
    n: &Node,
    vals: &Vals,
    bound: &mut HashSet<String>,
    edges: &mut Vec<(String, String)>,
) {
    if let NodeKind::Infix("==", a, b) = &n.kind {
        for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
            if let Some(key) = nft_slot_of(lhs, vals) {
                match nft_slot_of(rhs, vals) {
                    // Bound to another box's NFT slot: inherits its binding.
                    Some(other) => edges.push((key, other)),
                    // Bound to something the spender cannot move — a constant,
                    // a register of SELF — as long as it is not itself read off
                    // a positional box.
                    None => {
                        if !mentions_positional(rhs, vals, OPERAND_DEPTH) {
                            bound.insert(key);
                        }
                    }
                }
            }
            // `OUTPUTS(k).propositionBytes == SELF.propositionBytes`: the box is
            // this very script's successor, not a substitutable decoy.
            if let (Some(x), Some(y)) = (script_of(lhs, vals), script_of(rhs, vals)) {
                if box_key(y, vals).as_deref() == Some("SELF") {
                    if let Some(key) = positional_key(x, vals) {
                        bound.insert(key);
                    }
                }
            }
        }
    }
    for c in children(n) {
        scan_bindings(c, vals, bound, edges);
    }
}

// ── pass 2: report ───────────────────────────────────────────────────────────

fn report(
    n: &Node,
    vals: &Vals,
    bound: &HashSet<String>,
    seen: &mut HashSet<String>,
    out: &mut Vec<Finding>,
) {
    if let NodeKind::Infix(op, a, b) = &n.kind {
        if is_value_op(op) {
            let mut reads = Vec::new();
            reads_in_operand(a, vals, OPERAND_DEPTH, &mut reads);
            reads_in_operand(b, vals, OPERAND_DEPTH, &mut reads);
            for (box_node, read_node, how) in reads {
                let Some(key) = positional_key(box_node, vals) else {
                    continue;
                };
                if bound.contains(&key) || !seen.insert(key.clone()) {
                    continue;
                }
                out.push(Finding {
                    lint: "unbound-box-reserves",
                    severity: Severity::High,
                    node_id: read_node.id,
                    ir_id: None,
                    message: format!(
                        "reserves of {key} drive value math but this box is never bound by a \
                         singleton NFT (no tokens(0)._1 == <nft> check); a transaction can place \
                         a different box at this index. Bind it by its NFT."
                    ),
                    // Spelled from the resolved box key, not `print`: the lift
                    // hoists sub-expressions into `val`s, so the read node
                    // itself renders as `v3.value`, which names nothing.
                    snippet: format!("{key}{how}"),
                });
            }
        }
    }
    for c in children(n) {
        report(c, vals, bound, seen, out);
    }
}
