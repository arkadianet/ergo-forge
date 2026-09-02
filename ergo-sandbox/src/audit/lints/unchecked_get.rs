//! `Option.get` with no `isDefined` guard.
//!
//! If the option is empty at validation the script throws and the spend
//! fails; for a contract whose only spending path runs through that read,
//! the box becomes permanently unspendable.
//!
//! Severity follows **who controls the receiver**, found by walking the
//! receiver chain to its root:
//!
//! - a context variable (`getVar[T](n).get`) is supplied by the spender with
//!   the proof — a missing one fails that spend, it cannot lock the box: Low;
//! - a lambda parameter (an element of the collection being iterated —
//!   inputs, outputs, data inputs, or something derived from them) is an
//!   element the spender chose: Medium, fragile rather than locking;
//! - anything else (`SELF`, `OUTPUTS(0)`, a `val`) can be an unmovable fact
//!   about the box: High.
//!
//! Guards held in a `val` are followed: a block statement `val ok = …` whose
//! expression proves receivers defined makes `ok` prove the same receivers
//! wherever it appears in a guard position after its definition. (Single-use
//! vals are inlined by the compiler; this is for the multi-use ones that
//! survive to the tree.)
//!
//! Known gaps (deliberate — see the P3a spec): guards expressed with `||`
//! and negation, and cross-branch reasoning are not recognised, so those
//! produce false positives.

use std::collections::HashMap;

use crate::audit::{children, snippet, Finding, Severity};
use crate::decompile::Stmt;
use crate::{Node, NodeKind};

/// Boolean `val`s in scope that prove receivers defined, by name.
type GuardVals = HashMap<String, Vec<String>>;

/// Report every unguarded `Option.get` in `root`.
#[must_use]
pub fn unchecked_get(root: &Node) -> Vec<Finding> {
    let mut out = Vec::new();
    walk(
        root,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut GuardVals::new(),
        &mut out,
    );
    out
}

/// Is this node `x.get` with no arguments — i.e. `Option::get`?
///
/// Arity is the discriminator: `SCollection::get` (method table 0x0C/0x21)
/// and `AvlTree::get` (0x64/0x0A) both take an index argument.
fn as_option_get(n: &Node) -> Option<&Node> {
    match &n.kind {
        NodeKind::Method(recv, name, args) if name == "get" && args.is_empty() => Some(recv),
        _ => None,
    }
}

/// Receivers this expression proves non-empty, as rendered source text.
///
/// Only conjunctive structure proves anything: a direct `isDefined`, the
/// operands of nested `&&` (a `&&` chain that evaluated makes every operand
/// true), and the content of a `sigmaProp(…)` wrapper — the lift renders
/// every non-root `BoolToSigmaProp` as `Global("sigmaProp", [x])`, which is
/// definitionally transparent (it holds iff `x` holds). `isDefined` under
/// `||` or negation proves nothing — `x.isDefined || y.isDefined` can hold
/// with `x` empty — so those subtrees are not entered: documented
/// false-positive gaps stay false positives, never false negatives.
fn proves_defined(n: &Node, vals: &GuardVals, out: &mut Vec<String>) {
    match &n.kind {
        NodeKind::Method(recv, name, args) if name == "isDefined" && args.is_empty() => {
            out.push(crate::decompile::print(recv));
        }
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => {
            proves_defined(&args[0], vals, out);
        }
        NodeKind::Infix(op, a, b) if *op == "&&" => {
            proves_defined(a, vals, out);
            proves_defined(b, vals, out);
        }
        NodeKind::Val(name) => {
            if let Some(rs) = vals.get(name) {
                out.extend(rs.iter().cloned());
            }
        }
        _ => {}
    }
}

/// The leaf a receiver chain bottoms out in, looking through method calls,
/// properties, indexing and dynamic register reads.
fn receiver_root(n: &Node) -> &Node {
    match &n.kind {
        NodeKind::Method(o, _, _)
        | NodeKind::Prop(o, _)
        | NodeKind::ApplyFn(o, _)
        | NodeKind::GetRegDyn(o, _, _)
        | NodeKind::Index(o, _, _) => receiver_root(o),
        _ => n,
    }
}

/// Severity and message for an unguarded `get` on `recv`, given the lambda
/// parameters in scope.
fn classify(recv: &Node, params: &[String]) -> (Severity, String) {
    match &receiver_root(recv).kind {
        NodeKind::GetVar(id, _) => (
            Severity::Low,
            format!(
                "Option.get on context variable {id} with no isDefined guard — the spender must \
                 supply it or the script throws; not a lock, since the spender controls the \
                 extension."
            ),
        ),
        NodeKind::Val(name) if params.contains(name) => (
            Severity::Medium,
            "Option.get on a collection element with no isDefined guard — the script throws \
             for any element lacking the value; elements are chosen by the spender, so this \
             is fragile rather than locking (unless the element is SELF)."
                .into(),
        ),
        _ => (
            Severity::High,
            "Option.get with no isDefined guard — the script throws if the option is empty, \
             making this spending path unusable."
                .into(),
        ),
    }
}

fn walk(
    n: &Node,
    guarded: &mut Vec<String>,
    params: &mut Vec<String>,
    vals: &mut GuardVals,
    out: &mut Vec<Finding>,
) {
    // Scope-introducing shapes: everything the left/condition proves is
    // available to the right/then branch.
    match &n.kind {
        NodeKind::Infix(op, lhs, rhs) if *op == "&&" => {
            walk(lhs, guarded, params, vals, out);
            let depth = guarded.len();
            proves_defined(lhs, vals, guarded);
            walk(rhs, guarded, params, vals, out);
            guarded.truncate(depth);
            return;
        }
        NodeKind::If(cond, then_b, else_b) => {
            walk(cond, guarded, params, vals, out);
            let depth = guarded.len();
            proves_defined(cond, vals, guarded);
            walk(then_b, guarded, params, vals, out);
            guarded.truncate(depth);
            walk(else_b, guarded, params, vals, out);
            return;
        }
        NodeKind::Lambda(names, body) => {
            let depth = params.len();
            // Parameters are rendered `name: Type`; references are bare names.
            params.extend(
                names
                    .iter()
                    .map(|p| p.split(':').next().unwrap_or(p).trim().to_string()),
            );
            walk(body, guarded, params, vals, out);
            params.truncate(depth);
            return;
        }
        NodeKind::Block(stmts, result) => {
            // Statements bind in order: a val is a guard for everything
            // after its definition in this block, and nothing before.
            let mut added: Vec<String> = Vec::new();
            for st in stmts {
                let (name, expr) = match st {
                    Stmt::Val(n, e) | Stmt::Def(n, e) => (n, e),
                };
                walk(expr, guarded, params, vals, out);
                let mut proves = Vec::new();
                proves_defined(expr, vals, &mut proves);
                if !proves.is_empty() && vals.insert(name.clone(), proves).is_none() {
                    added.push(name.clone());
                }
            }
            walk(result, guarded, params, vals, out);
            for name in added {
                vals.remove(&name);
            }
            return;
        }
        _ => {}
    }

    if let Some(recv) = as_option_get(n) {
        if !guarded.contains(&crate::decompile::print(recv)) {
            let (severity, message) = classify(recv, params);
            out.push(Finding {
                lint: "unchecked-get",
                severity,
                node_id: n.id,
                message,
                snippet: snippet(n),
            });
        }
    }

    for c in children(n) {
        walk(c, guarded, params, vals, out);
    }
}
