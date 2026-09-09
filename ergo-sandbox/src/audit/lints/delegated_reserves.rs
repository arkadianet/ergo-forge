//! Successor identity preserved without local reserve constraints.
//!
//! A vault can require the same script and NFT on its successor while leaving
//! its ERG value and fungible-token amounts to companion contracts. This is a
//! legitimate delegation pattern (including the USE/Dexy bank), but the reader
//! must review those delegates to understand the reserve guarantees.
//!
//! A positional **output** is reported when:
//!
//! 1. its script equals `SELF`'s, or its tokens equal `SELF`'s (a collection,
//!    a same-index pair, or a same-index token id); and
//! 2. no comparison relates its ERG value to `SELF.value`, and/or a preserved
//!    token id has no companion amount comparison or pair/collection equality.
//!
//! Comparisons include `==`, `<`, `<=`, `>` and `>=`, following `val`s, casts
//! and arithmetic with the shared reserve-read machinery. Direct ERG upper
//! bounds alone do not count; arithmetic comparisons involving both values
//! count so constant-product and withdrawal bounds are recognised. A literal
//! dust floor alone does not relate the successor to `SELF`'s reserves.
//!
//! Known limits (deliberate): this is a whole-tree syntactic absence check,
//! not a proof of bounds or spending paths. A comparison in only one branch
//! can suppress a finding; arithmetic signs, cancellation and tautologies
//! are not proved. Token amount comparisons need not preserve the old amount
//! (mint/redeem flows change it). Singleton supply cannot be inferred from
//! source, so every preserved id-only slot is checked, including index 0.
//! Computed indices, searched successors, transitive identity equalities and
//! reserve relationships beyond the shared operand-depth/wrapper support are
//! not resolved. No finding means only that this pattern was not recognised.
//! In particular, Dexy's `bank/buyback.es` combines action-specific successors
//! with token accounting on other outputs; this lint cannot decide its
//! per-action reserve guarantees or establish the supply of its NFT.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::audit::boxrefs::{
    as_indexed, box_key, collect_vals, deref, is_static_index, reads_in_operand, reserve_read,
    script_of, tokens_receiver, Vals, OPERAND_DEPTH,
};
use crate::audit::{children, Finding, Severity};
use crate::{Node, NodeKind};

#[derive(Default)]
struct Successor {
    node_id: u64,
    id_slots: BTreeSet<String>,
}

#[derive(Default)]
struct Evidence {
    successors: BTreeMap<String, Successor>,
    erg_bounds: HashSet<String>,
    amounts: HashSet<(String, String)>,
    token_collections: HashSet<String>,
}

/// Report successors whose identity is preserved but whose reserves are
/// delegated in whole or in part to other transaction requirements.
#[must_use]
pub fn delegated_reserves(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut evidence = Evidence::default();
    scan(root, &vals, &mut evidence);

    evidence
        .successors
        .iter()
        .filter_map(|(key, successor)| {
            let mut missing = Vec::new();
            if !evidence.erg_bounds.contains(key) {
                missing.push("ERG value has no recognised bound relating it to SELF.value".into());
            }
            if !evidence.token_collections.contains(key) {
                for slot in &successor.id_slots {
                    if !evidence.amounts.contains(&(key.clone(), slot.clone())) {
                        missing.push(format!(
                            "tokens({slot})._1 is preserved but tokens({slot})._2 has no recognised companion amount constraint: unbounded unless token slot {slot} is a supply-1 singleton, which this analysis cannot observe"
                        ));
                    }
                }
            }
            if missing.is_empty() {
                return None;
            }
            Some(Finding {
                lint: "delegated-reserves",
                severity: Severity::Medium,
                node_id: successor.node_id,
                ir_id: None,
                message: format!(
                    "{key} preserves SELF's identity, but {}; reserve guarantees for these \
                     assets may depend on singleton supply or other transaction requirements. Review the \
                     authorising inputs and companion contracts; reserve checks may be delegated intentionally \
                     and this is not evidence of exploitability.",
                    missing.join("; ")
                ),
                snippet: key.clone(),
            })
        })
        .collect()
}

/// A token collection, pair or id, preserving the index and projection so
/// unrelated slots (or an id versus an amount) cannot become an identity edge.
fn token_identity(n: &Node, vals: &Vals) -> Option<(String, Option<String>, bool)> {
    let d = deref(n, vals);
    if let Some(b) = tokens_receiver(d, vals) {
        return Some((box_key(b, vals)?, None, false));
    }
    let (pair, id_only) = match &d.kind {
        NodeKind::Prop(t, name) if name == "_1" => (deref(t, vals), true),
        _ => (d, false),
    };
    let (coll, idx) = as_indexed(pair)?;
    if !is_static_index(idx, vals) {
        return None;
    }
    let b = tokens_receiver(coll, vals)?;
    Some((
        box_key(b, vals)?,
        Some(crate::decompile::print(deref(idx, vals))),
        id_only,
    ))
}

fn successor<'a>(e: &'a mut Evidence, key: &str, node: &Node) -> &'a mut Successor {
    e.successors.entry(key.into()).or_insert_with(|| Successor {
        node_id: node.id,
        ..Successor::default()
    })
}

fn identity(lhs: &Node, rhs: &Node, site: &Node, vals: &Vals, e: &mut Evidence) {
    if let (Some(a), Some(b)) = (script_of(lhs, vals), script_of(rhs, vals)) {
        if box_key(b, vals).as_deref() == Some("SELF") {
            if let Some(key) = box_key(a, vals).filter(|k| k.starts_with("OUTPUTS(")) {
                successor(e, &key, site);
            }
        }
    }
    if let (Some((key, slot, id_only)), Some((other, other_slot, other_id_only))) =
        (token_identity(lhs, vals), token_identity(rhs, vals))
    {
        if key.starts_with("OUTPUTS(")
            && other == "SELF"
            && slot == other_slot
            && id_only == other_id_only
        {
            let succ = successor(e, &key, site);
            if let Some(slot) = slot {
                if id_only {
                    succ.id_slots.insert(slot);
                } else {
                    e.amounts.insert((key, slot));
                }
            } else {
                e.token_collections.insert(key);
            }
        }
    }
}

/// A bare value, allowing only numeric casts. Used to distinguish a direct
/// upper bound from the arithmetic relationships we leave to manual review.
fn bare_value(n: &Node, vals: &Vals, depth: u32) -> Option<String> {
    if depth == 0 {
        return None;
    }
    let d = deref(n, vals);
    if let Some((b, how)) = reserve_read(d, vals) {
        return (how == ".value").then(|| box_key(b, vals)).flatten();
    }
    if let NodeKind::Method(b, name, args) = &d.kind {
        if args.is_empty() && matches!(name.as_str(), "toLong" | "toInt" | "toBigInt") {
            return bare_value(b, vals, depth - 1);
        }
    }
    None
}

fn comparison(op: &str, a: &Node, b: &Node, vals: &Vals, e: &mut Evidence) {
    let mut reads = Vec::new();
    reads_in_operand(a, vals, OPERAND_DEPTH, &mut reads);
    reads_in_operand(b, vals, OPERAND_DEPTH, &mut reads);
    let reads: Vec<_> = reads
        .into_iter()
        .filter_map(|(box_node, _, how)| Some((box_key(box_node, vals)?, how)))
        .collect();
    let self_value = reads.iter().any(|(k, how)| k == "SELF" && how == ".value");
    let direct = (
        bare_value(a, vals, OPERAND_DEPTH),
        bare_value(b, vals, OPERAND_DEPTH),
    );
    for (key, how) in reads.iter().filter(|(k, _)| k.starts_with("OUTPUTS(")) {
        if how == ".value" {
            let upper_only = match &direct {
                (Some(lhs), Some(rhs)) if lhs == key && rhs == "SELF" => {
                    matches!(op, "<" | "<=")
                }
                (Some(lhs), Some(rhs)) if lhs == "SELF" && rhs == key => {
                    matches!(op, ">" | ">=")
                }
                _ => false,
            };
            if self_value && !upper_only {
                e.erg_bounds.insert(key.clone());
            }
        } else if let Some(slot) = how
            .strip_prefix(".tokens(")
            .and_then(|s| s.strip_suffix(")._2"))
        {
            e.amounts.insert((key.clone(), slot.into()));
        }
    }
}

fn scan(n: &Node, vals: &Vals, e: &mut Evidence) {
    if let NodeKind::Infix(op, a, b) = &n.kind {
        if *op == "==" {
            identity(a, b, n, vals, e);
            identity(b, a, n, vals, e);
        }
        if matches!(*op, "==" | "<" | "<=" | ">" | ">=") {
            comparison(op, a, b, vals, e);
        }
    }
    for c in children(n) {
        scan(c, vals, e);
    }
}
