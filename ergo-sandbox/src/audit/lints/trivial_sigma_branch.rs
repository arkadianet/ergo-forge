//! Result alternatives with constant truth or spender-selected authority.
//!
//! Follows only the overall result, block/val/sigmaProp wrappers and direct
//! disjunctions (`||` or literal `anyOf`). Recognises true, zero-threshold
//! `atLeast`, and propositions built entirely from literals and getVar or
//! registers on positional boxes without an immutable identity comparison.
//! Box identity uses the same literal/SELF anchors as `trust_assumptions`.
//!
//! Known limits (deliberate): syntax, not reachability or satisfiability.
//! Conjunctions surrounding a disjunction are not distributed; unrelated if
//! branches and unused vals are not result alternatives. Writable comparisons
//! can still contradict or throw, and writable public keys still require a
//! proof for the selected key. Branch-local identity comparisons may suppress
//! observations. Computed boxes and arbitrary functions remain undecided.
//! Writable predicates must be direct disjuncts; conjunctions are undecided.
//! A complete constant-true result is included because compilation can erase
//! its original disjunction. Box bindings also include recognised successors.
//! LOW accommodates intentional permissionless predicates; silence is no proof
//! of authorisation, and a finding is no proof that a spend succeeds.

use std::collections::HashSet;

use crate::audit::boxrefs::{box_key, collect_vals, deref, Vals};
use crate::audit::{snippet, Finding, Severity};
use crate::{Node, NodeKind};

/// Report result alternatives with no recognised precommitted authority.
#[must_use]
pub fn trivial_sigma_branch(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut bound = super::trust_assumptions::bound_boxes(root, &vals);
    bound.extend(
        super::delegated_reserves::evidence(root, &vals)
            .successors
            .into_keys(),
    );
    let mut out = Vec::new();
    alternatives(root, &vals, &bound, false, 128, &mut out);
    out.sort_by_key(|f| f.node_id);
    out.dedup_by_key(|f| f.node_id);
    out
}

// Some(true): supported expression uses writable data. Some(false): literal.
// None: independent authority, contextual state or an unsupported expression.
fn writable(n: &Node, vals: &Vals, bound: &HashSet<String>, depth: u32) -> Option<bool> {
    if depth == 0 {
        return None;
    }
    let d = deref(n, vals);
    let combine = |nodes: Vec<&Node>| {
        nodes.into_iter().try_fold(false, |taint, c| {
            Some(taint | writable(c, vals, bound, depth - 1)?)
        })
    };
    match &d.kind {
        NodeKind::Bool(_) | NodeKind::Int(_) | NodeKind::Num(_) | NodeKind::Const(_) => Some(false),
        NodeKind::GetVar(..) => Some(true),
        NodeKind::Prop(b, name) if name.starts_with('R') && name.contains('[') => box_key(b, vals)
            .filter(|key| !bound.contains(key))
            .map(|_| true),
        NodeKind::Method(b, name, args)
            if matches!(name.as_str(), "get" | "getOrElse" | "isDefined") =>
        {
            combine(std::iter::once(b.as_ref()).chain(args).collect())
        }
        NodeKind::Global(name, args)
            if matches!(name.as_str(), "sigmaProp" | "proveDlog" | "decodePoint") =>
        {
            combine(args.iter().collect())
        }
        NodeKind::Infix(
            "==" | "!=" | "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/" | "%",
            a,
            b,
        ) => combine(vec![a, b]),
        NodeKind::Unary("!" | "-", b) => writable(b, vals, bound, depth - 1),
        _ => None,
    }
}

fn alternatives(
    n: &Node,
    vals: &Vals,
    bound: &HashSet<String>,
    disjunct: bool,
    depth: u32,
    out: &mut Vec<Finding>,
) {
    if depth == 0 {
        return;
    }
    let d = deref(n, vals);
    match &d.kind {
        NodeKind::Block(_, result) => alternatives(result, vals, bound, disjunct, depth - 1, out),
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => {
            alternatives(&args[0], vals, bound, disjunct, depth - 1, out)
        }
        NodeKind::Infix("||", a, b) => {
            alternatives(a, vals, bound, true, depth - 1, out);
            alternatives(b, vals, bound, true, depth - 1, out);
        }
        NodeKind::Global(name, args) if name == "anyOf" && args.len() == 1 => {
            if let NodeKind::Coll(_, items) = &deref(&args[0], vals).kind {
                for item in items {
                    alternatives(item, vals, bound, true, depth - 1, out);
                }
            }
        }
        _ => {
            let constant = matches!(d.kind, NodeKind::Bool(true))
                || matches!(&d.kind,
                NodeKind::AtLeast(k, _) if matches!(&deref(k, vals).kind, NodeKind::Int(0))
                    || matches!(&deref(k, vals).kind, NodeKind::Num(s) if s == "0"));
            if constant || (disjunct && writable(d, vals, bound, 64) == Some(true)) {
                out.push(Finding {
                    triage: Default::default(),
                    lint: "trivial-sigma-branch",
                    severity: Severity::Low,
                    node_id: d.id,
                    ir_id: None,
                    message: format!("A result alternative {}. Review whether permissionless release or \
                        spender-selected authority is intended; evaluation and proofs can still fail, \
                        and this syntax observation is not evidence of exploitability.", if constant {
                            "is constant true or has a zero signature threshold"
                        } else { "depends only on literals and writable context/register data without a recognised box identity binding" }),
                    snippet: snippet(d),
                });
            }
        }
    }
}
