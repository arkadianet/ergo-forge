//! `Option.get` with no `isDefined` guard.
//!
//! If the option is empty at validation the script throws and the spend
//! fails; for a contract whose only spending path runs through that read,
//! the box becomes permanently unspendable.
//!
//! Known gaps (deliberate — see the P3a spec): guards expressed with `||`
//! and negation, guards held in an enclosing `val`, and cross-branch
//! reasoning are not recognised, so those produce false positives.

use crate::audit::{children, snippet, Finding, Severity};
use crate::{Node, NodeKind};

/// Report every unguarded `Option.get` in `root`.
#[must_use]
pub fn unchecked_get(root: &Node) -> Vec<Finding> {
    let mut out = Vec::new();
    walk(root, &mut Vec::new(), &mut out);
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
fn proves_defined(n: &Node, out: &mut Vec<String>) {
    match &n.kind {
        NodeKind::Method(recv, name, args) if name == "isDefined" && args.is_empty() => {
            out.push(crate::decompile::print(recv));
        }
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => {
            proves_defined(&args[0], out);
        }
        NodeKind::Infix(op, a, b) if *op == "&&" => {
            proves_defined(a, out);
            proves_defined(b, out);
        }
        _ => {}
    }
}

fn walk(n: &Node, guarded: &mut Vec<String>, out: &mut Vec<Finding>) {
    // Scope-introducing shapes: everything the left/condition proves is
    // available to the right/then branch.
    match &n.kind {
        NodeKind::Infix(op, lhs, rhs) if *op == "&&" => {
            walk(lhs, guarded, out);
            let depth = guarded.len();
            proves_defined(lhs, guarded);
            walk(rhs, guarded, out);
            guarded.truncate(depth);
            return;
        }
        NodeKind::If(cond, then_b, else_b) => {
            walk(cond, guarded, out);
            let depth = guarded.len();
            proves_defined(cond, guarded);
            walk(then_b, guarded, out);
            guarded.truncate(depth);
            walk(else_b, guarded, out);
            return;
        }
        _ => {}
    }

    if let Some(recv) = as_option_get(n) {
        if !guarded.contains(&crate::decompile::print(recv)) {
            out.push(Finding {
                lint: "unchecked-get",
                severity: Severity::High,
                node_id: n.id,
                message: "Option.get with no isDefined guard — the script throws if the option is \
                     empty, making this spending path unusable."
                    .into(),
                snippet: snippet(n),
            });
        }
    }

    for c in children(n) {
        walk(c, guarded, out);
    }
}
