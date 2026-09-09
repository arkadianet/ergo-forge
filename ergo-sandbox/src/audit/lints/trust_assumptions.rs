//! Data-input register provenance without a recognised identity comparison.
//!
//! A positional data input is reported once when its register is extracted
//! with `get` or `getOrElse`, but nowhere in the tree is its box id or a
//! literal-index token id compared to a literal or a value rooted in SELF.
//! A direct `tokens.exists(t => t._1 == expected)` also counts, including
//! Rosen's any-token-slot guard. The expected value may follow `val`s,
//! register extraction and literal indexing; context variables and other
//! positional boxes are not anchors. Script equality alone does not select
//! a particular box or token lineage.
//!
//! This is a review obligation about provenance, not an assertion that the
//! register value is unconstrained. For example, Chaincash's note checks a
//! reserve's owner key, allowing a family of reserves rather than a fixed NFT.
//!
//! Known limits (deliberate): a whole-tree syntactic absence check. An identity
//! equality in only one branch (or used under negation) suppresses a finding;
//! its enforcement is not proved. Token singleton supply, freshness, register
//! validity, signatures and AVL proofs are not decided. Computed box/token
//! indices, searched boxes, transitive identity bindings, compound `exists`
//! predicates and wrappers beyond the bounded recogniser are not resolved.
//! Only extracted data-input registers are covered, not arbitrary SELF/input
//! registers, presence tests alone, dynamic register access or context values.
//! Silence means this pattern was not recognised, not that trust is justified.

use std::collections::{BTreeMap, HashSet};

use crate::audit::boxrefs::{
    as_indexed, box_key, collect_vals, deref, tokens_receiver, Vals, OPERAND_DEPTH,
};
use crate::audit::{children, Finding, Severity};
use crate::{Node, NodeKind};

/// Report extracted data-input registers whose box provenance needs review.
#[must_use]
pub fn trust_assumptions(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut bound = HashSet::new();
    let mut reads = BTreeMap::new();
    scan(root, &vals, &mut bound, &mut reads);
    reads
        .into_iter()
        .filter(|(key, _)| !bound.contains(key))
        .map(|(key, (node_id, register))| Finding {
            lint: "trust-assumptions",
            severity: Severity::Medium,
            node_id,
            ir_id: None,
            message: format!(
                "{key} supplies {register} without a recognised box-id or token-id comparison \
                 to a literal or SELF-rooted value. Review the intended data provider and \
                 its authentication; other value or key checks may intentionally suffice, \
                 and this is not evidence of exploitability."
            ),
            snippet: format!("{key}.{register}"),
        })
        .collect()
}

fn literal_index(n: &Node, vals: &Vals) -> bool {
    match &deref(n, vals).kind {
        NodeKind::Int(i) => *i >= 0,
        NodeKind::Num(s) => s.parse::<u32>().is_ok(),
        _ => false,
    }
}

/// An explicitly recognised immutable anchor, never an arbitrary expression
/// merely because no positional read was found inside it.
fn anchor(n: &Node, vals: &Vals, depth: u32) -> bool {
    if depth == 0 {
        return false;
    }
    let d = deref(n, vals);
    match &d.kind {
        NodeKind::Const(_)
        | NodeKind::Int(_)
        | NodeKind::Num(_)
        | NodeKind::Bool(_)
        | NodeKind::Leaf("SELF") => true,
        NodeKind::Prop(b, _) => anchor(b, vals, depth - 1),
        NodeKind::Method(b, name, args)
            if matches!(name.as_str(), "get" | "tokens") && args.is_empty() =>
        {
            anchor(b, vals, depth - 1)
        }
        _ => {
            as_indexed(d).is_some_and(|(b, i)| literal_index(i, vals) && anchor(b, vals, depth - 1))
        }
    }
}

fn identity(n: &Node, vals: &Vals) -> Option<String> {
    let NodeKind::Prop(b, name) = &deref(n, vals).kind else {
        return None;
    };
    if name == "id" {
        return box_key(b, vals);
    }
    if name != "_1" {
        return None;
    }
    let (coll, idx) = as_indexed(deref(b, vals))?;
    if !literal_index(idx, vals) {
        return None;
    }
    box_key(tokens_receiver(coll, vals)?, vals)
}

/// The lift unwraps two-field tuple lambdas into `(id, amount)` parameters.
fn token_parameter(n: &Node, params: &[String], vals: &Vals) -> bool {
    let (param, value) = match params {
        [param] => match &deref(n, vals).kind {
            NodeKind::Prop(t, field) if field == "_1" => (param, &**t),
            _ => return false,
        },
        [id, _amount] => (id, deref(n, vals)),
        _ => return false,
    };
    let name = param.split(':').next().unwrap_or(param).trim();
    matches!(&value.kind, NodeKind::Val(v) if v == name)
}

fn scan(
    n: &Node,
    vals: &Vals,
    bound: &mut HashSet<String>,
    reads: &mut BTreeMap<String, (u64, String)>,
) {
    if let NodeKind::Infix("==", a, b) = &n.kind {
        for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
            if let Some(key) = identity(lhs, vals) {
                if anchor(rhs, vals, OPERAND_DEPTH) {
                    bound.insert(key);
                }
            }
        }
    }
    if let NodeKind::Method(recv, name, args) = &n.kind {
        if (name == "get" && args.is_empty()) || (name == "getOrElse" && args.len() == 1) {
            if let NodeKind::Prop(b, register) = &deref(recv, vals).kind {
                if register.starts_with('R') && register.contains('[') {
                    if let Some(key) =
                        box_key(b, vals).filter(|k| k.starts_with("CONTEXT.dataInputs("))
                    {
                        reads.entry(key).or_insert((n.id, register.clone()));
                    }
                }
            }
        }
        if name == "exists" && args.len() == 1 {
            if let Some(key) = tokens_receiver(recv, vals).and_then(|b| box_key(b, vals)) {
                if let NodeKind::Lambda(params, body) = &deref(&args[0], vals).kind {
                    if let NodeKind::Infix("==", a, b) = &deref(body, vals).kind {
                        for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
                            if token_parameter(lhs, params, vals)
                                && anchor(rhs, vals, OPERAND_DEPTH)
                            {
                                bound.insert(key.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    for c in children(n) {
        scan(c, vals, bound, reads);
    }
}
