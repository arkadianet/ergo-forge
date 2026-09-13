//! Positional output checks with no recognised constraint on the output tail.
//!
//! Reports once per tree if a comparison consumes a specific OUTPUTS index,
//! but no upper/equality size bound, total ERG sum comparison, whole-output
//! equality, or `forall` comparison over outputs (or their tail) is present.
//! A size lower bound and `exists` do not constrain additional outputs.
//!
//! Known limits (deliberate): whole-tree syntax, not reachability. An unused,
//! negated or branch-local comparison can suppress a finding. Only direct size
//! bounds, additive folds and simple universal comparisons are recognised;
//! computed indices and more elaborate accounting are undecided. Even a size
//! bound does not prove payment adequacy. Ordinary fee/change and composable
//! orders intentionally leave a tail: LOW is an observation for review, never
//! a vulnerability verdict. Silence never proves all value exits constrained.

use std::collections::BTreeMap;

use crate::audit::boxrefs::{box_collection, collect_vals, deref, positional_key, Vals};
use crate::audit::{children, Finding, Severity};
use crate::{Node, NodeKind};

/// Report positional output constraints whose remaining outputs need review.
#[must_use]
pub fn unconstrained_outputs(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut sites = BTreeMap::new();
    let mut bounded = false;
    scan(root, &vals, &mut sites, &mut bounded);
    if bounded || sites.is_empty() {
        return vec![];
    }
    let node_id = *sites.values().min().expect("nonempty");
    vec![Finding {
        triage: Default::default(),
        lint: "unconstrained-outputs",
        severity: Severity::Low,
        node_id,
        ir_id: None,
        message: format!(
            "Specific outputs ({}) are compared, but no output-count upper bound, total-value \
             comparison or universal tail constraint is recognised. Additional fee or change \
             outputs may be intentional value exits. Review transaction-wide accounting; \
             this syntax observation is not evidence of exploitability.",
            sites.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
        snippet: "OUTPUTS: positional checks without a recognised tail bound".into(),
    }]
}

fn outputs(n: &Node, vals: &Vals) -> bool {
    box_collection(n, vals) == Some("OUTPUTS")
}

fn mentions_outputs(n: &Node, vals: &Vals, depth: u32) -> bool {
    if depth == 0 {
        return true; // Unknown cannot establish an independent bound.
    }
    let d = deref(n, vals);
    outputs(d, vals)
        || matches!(d.kind, NodeKind::Raw(_) | NodeKind::Val(_))
        || children(d)
            .into_iter()
            .any(|c| mentions_outputs(c, vals, depth - 1))
}

fn size(n: &Node, vals: &Vals) -> bool {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name) => name == "size" && outputs(b, vals),
        NodeKind::Method(b, name, args) => name == "size" && args.is_empty() && outputs(b, vals),
        _ => false,
    }
}

fn parameter(n: &Node, name: &str, vals: &Vals) -> bool {
    matches!(&deref(n, vals).kind, NodeKind::Val(v) if v == name.split(':').next().unwrap_or(name).trim())
}

fn value_of(n: &Node, param: &str, vals: &Vals) -> bool {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name) => name == "value" && parameter(b, param, vals),
        NodeKind::Method(b, name, args) => {
            name == "value" && args.is_empty() && parameter(b, param, vals)
        }
        _ => false,
    }
}

fn total_value(n: &Node, vals: &Vals) -> bool {
    let NodeKind::Method(coll, name, args) = &deref(n, vals).kind else {
        return false;
    };
    if name != "fold" || args.len() != 2 {
        return false;
    }
    let NodeKind::Lambda(params, body) = &deref(&args[1], vals).kind else {
        return false;
    };
    let [acc, item] = params.as_slice() else {
        return false;
    };
    let NodeKind::Infix("+", a, b) = &deref(body, vals).kind else {
        return false;
    };
    let mapped = match &deref(coll, vals).kind {
        NodeKind::Method(source, method, f)
            if method == "map" && outputs(source, vals) && f.len() == 1 =>
        {
            matches!(&deref(&f[0], vals).kind, NodeKind::Lambda(p, body)
                if p.len() == 1 && value_of(body, &p[0], vals))
        }
        _ => false,
    };
    [(&**a, &**b), (&**b, &**a)].into_iter().any(|(a, b)| {
        parameter(a, acc, vals)
            && ((outputs(coll, vals) && value_of(b, item, vals))
                || (mapped && parameter(b, item, vals)))
    })
}

fn tail(n: &Node, vals: &Vals) -> bool {
    outputs(n, vals)
        || matches!(&deref(n, vals).kind,
        NodeKind::Method(b, name, args) if outputs(b, vals) && name == "slice" && args.len() == 2
            && size(&args[1], vals))
}

fn uses_parameter(n: &Node, param: &str, vals: &Vals, depth: u32) -> bool {
    depth > 0
        && (parameter(n, param, vals)
            || children(deref(n, vals))
                .into_iter()
                .any(|c| uses_parameter(c, param, vals, depth - 1)))
}

fn constrained_predicate(n: &Node, param: &str, vals: &Vals) -> bool {
    match &deref(n, vals).kind {
        NodeKind::Infix("==" | "<" | "<=" | ">" | ">=", a, b) => {
            uses_parameter(a, param, vals, 32) != uses_parameter(b, param, vals, 32)
        }
        NodeKind::Infix("&&", a, b) => {
            constrained_predicate(a, param, vals) || constrained_predicate(b, param, vals)
        }
        _ => false,
    }
}

fn positions(n: &Node, vals: &Vals, sites: &mut BTreeMap<String, u64>, depth: u32) {
    if depth == 0 {
        return;
    }
    let d = deref(n, vals);
    if let Some(key) = positional_key(d, vals).filter(|k| k.starts_with("OUTPUTS(")) {
        sites.entry(key).or_insert(d.id);
        return;
    }
    for c in children(d) {
        positions(c, vals, sites, depth - 1);
    }
}

fn scan(n: &Node, vals: &Vals, sites: &mut BTreeMap<String, u64>, bounded: &mut bool) {
    if let NodeKind::Infix(op @ ("==" | "!=" | "<" | "<=" | ">" | ">="), a, b) = &n.kind {
        positions(a, vals, sites, 64);
        positions(b, vals, sites, 64);
        for (lhs, rhs, reversed) in [(&**a, &**b, false), (&**b, &**a, true)] {
            let upper = *op == "=="
                || (!reversed && matches!(*op, "<" | "<="))
                || (reversed && matches!(*op, ">" | ">="));
            if !mentions_outputs(rhs, vals, 64)
                && ((upper && (size(lhs, vals) || total_value(lhs, vals)))
                    || (outputs(lhs, vals) && *op == "=="))
            {
                *bounded = true;
            }
        }
    }
    if let NodeKind::Method(coll, name, args) = &n.kind {
        if name == "forall" && tail(coll, vals) && args.len() == 1 {
            if let NodeKind::Lambda(params, body) = &deref(&args[0], vals).kind {
                if params.len() == 1 && constrained_predicate(body, &params[0], vals) {
                    *bounded = true;
                }
            }
        }
    }
    for c in children(n) {
        scan(c, vals, sites, bounded);
    }
}
