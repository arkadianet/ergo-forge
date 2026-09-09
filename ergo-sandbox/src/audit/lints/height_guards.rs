//! Height conditions that deserve a review of the intended release policy.
//!
//! Two deliberately narrow patterns are reported:
//!
//! - A complete result, or a direct disjunct of it, is only `HEIGHT > h` or
//!   `HEIGHT >= h` for a nonnegative literal below `Int.MaxValue`. This is
//!   informational: a permissionless release may be exactly what was intended.
//! - An `if` whose condition contains a HEIGHT comparison selects syntactically
//!   identical branches. The height does not select different requirements.
//!
//! Known limits (deliberate): follows lifted `val`s, block results and
//! `sigmaProp`, but does not distribute conjunctions or infer missing bounds
//! from names such as "deadline". Ordinary owner-authorised timelocks are not
//! reported. Dynamic deadlines, arithmetic bounds, negation and computed
//! disjunction collections are undecided. Identical-branch checks use rendered
//! syntax after resolving the branch's outer alias, not semantic equivalence;
//! raw subtrees are excluded. The condition or enclosing block can still fail
//! during evaluation, so a finding is not a proof that a spend succeeds.

use crate::audit::boxrefs::{collect_vals, deref, Vals, OPERAND_DEPTH};
use crate::audit::{children, snippet, Finding, Severity};
use crate::{Node, NodeKind};

/// Report height-only releases and height conditionals with identical outcomes.
#[must_use]
pub fn height_guards(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut out = Vec::new();
    release(root, &vals, OPERAND_DEPTH, &mut out);
    scan(root, &vals, &mut out);
    out.sort_by_key(|f| f.node_id);
    out.dedup_by_key(|f| f.node_id);
    out
}

fn finding(n: &Node, severity: Severity, message: &str) -> Finding {
    Finding {
        triage: Default::default(),
        lint: "height-guards",
        severity,
        node_id: n.id,
        ir_id: None,
        message: message.into(),
        snippet: snippet(n),
    }
}

fn literal(n: &Node, vals: &Vals) -> Option<i64> {
    match &deref(n, vals).kind {
        NodeKind::Int(i) => Some(*i),
        NodeKind::Num(s) => s.parse().ok(),
        _ => None,
    }
}

fn release(n: &Node, vals: &Vals, depth: u32, out: &mut Vec<Finding>) {
    if depth == 0 {
        return;
    }
    let d = deref(n, vals);
    match &d.kind {
        NodeKind::Block(_, result) => release(result, vals, depth - 1, out),
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => {
            release(&args[0], vals, depth - 1, out);
        }
        NodeKind::Infix("||", a, b) => {
            release(a, vals, depth - 1, out);
            release(b, vals, depth - 1, out);
        }
        NodeKind::Global(name, args) if name == "anyOf" && args.len() == 1 => {
            if let NodeKind::Coll(_, items) = &deref(&args[0], vals).kind {
                for item in items {
                    release(item, vals, depth - 1, out);
                }
            }
        }
        NodeKind::Infix(op, a, b) => {
            let bound = if matches!(*op, ">" | ">=")
                && matches!(deref(a, vals).kind, NodeKind::Leaf("HEIGHT"))
            {
                literal(b, vals)
            } else if matches!(*op, "<" | "<=")
                && matches!(deref(b, vals).kind, NodeKind::Leaf("HEIGHT"))
            {
                literal(a, vals)
            } else {
                None
            };
            if bound.is_some_and(|h| (0..i64::from(i32::MAX)).contains(&h)) {
                out.push(finding(
                    d,
                    Severity::Low,
                    "a result alternative consists only of a HEIGHT lower bound; after that \
                     bound it adds no authorisation or expiry requirement. Review whether a \
                     permissionless release is intended; enclosing evaluation can still fail \
                     and this is not evidence of exploitability.",
                ));
            }
        }
        _ => {}
    }
}

fn height_comparison(n: &Node, vals: &Vals, depth: u32) -> bool {
    if depth == 0 {
        return false;
    }
    let d = deref(n, vals);
    if let NodeKind::Infix("==" | "!=" | "<" | "<=" | ">" | ">=", a, b) = &d.kind {
        if matches!(deref(a, vals).kind, NodeKind::Leaf("HEIGHT"))
            || matches!(deref(b, vals).kind, NodeKind::Leaf("HEIGHT"))
        {
            return true;
        }
    }
    children(d)
        .into_iter()
        .any(|c| height_comparison(c, vals, depth - 1))
}

fn understood(n: &Node, vals: &Vals, depth: u32) -> bool {
    if depth == 0 {
        return false;
    }
    let d = deref(n, vals);
    if let NodeKind::Val(name) = &d.kind {
        // Unbound names include lambda parameters. A name still present in
        // the binding map after deref indicates a cycle, which is undecided.
        return !vals.contains_key(name);
    }
    !matches!(d.kind, NodeKind::Raw(_))
        && children(d)
            .into_iter()
            .all(|c| understood(c, vals, depth - 1))
}

fn scan(n: &Node, vals: &Vals, out: &mut Vec<Finding>) {
    if let NodeKind::If(c, t, e) = &n.kind {
        let (t, e) = (deref(t, vals), deref(e, vals));
        if height_comparison(c, vals, OPERAND_DEPTH)
            && understood(t, vals, OPERAND_DEPTH)
            && understood(e, vals, OPERAND_DEPTH)
            && crate::decompile::print(t) == crate::decompile::print(e)
        {
            out.push(finding(
                n,
                Severity::Medium,
                "a conditional containing a HEIGHT comparison selects identical branches; \
                 the height does not select different spending requirements here. Review \
                 whether a time-dependent restriction was intended; evaluating the condition \
                 can still fail and this is not evidence of exploitability.",
            ));
        }
    }
    for c in children(n) {
        scan(c, vals, out);
    }
}
