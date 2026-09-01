//! Generic structural traversal over the lifted AST.

use crate::decompile::Stmt;
use crate::{Node, NodeKind};

/// Direct children of `n`, in source order.
///
/// Exhaustive by construction: adding a `NodeKind` variant breaks this match,
/// which is the point — a new shape must not be silently skipped by lints.
#[must_use]
pub fn children(n: &Node) -> Vec<&Node> {
    match &n.kind {
        NodeKind::Unary(_, a) => vec![a],
        NodeKind::Infix(_, a, b) => vec![a, b],
        NodeKind::Method(o, _, args) | NodeKind::GetRegDyn(o, _, args) => {
            let mut v = vec![&**o];
            v.extend(args);
            v
        }
        NodeKind::ApplyFn(o, args) => {
            let mut v = vec![&**o];
            v.extend(args);
            v
        }
        NodeKind::Prop(o, _) => vec![o],
        NodeKind::Coll(_, items) | NodeKind::Tuple(items) => items.iter().collect(),
        NodeKind::Global(_, args) => args.iter().collect(),
        NodeKind::Lambda(_, b) => vec![b],
        NodeKind::If(c, t, e) => vec![c, t, e],
        NodeKind::AtLeast(k, c) => vec![k, c],
        NodeKind::Index(a, b, d) => {
            let mut v = vec![&**a, &**b];
            if let Some(d) = d {
                v.push(d);
            }
            v
        }
        NodeKind::Block(stmts, result) => {
            let mut v: Vec<&Node> = stmts
                .iter()
                .map(|s| match s {
                    Stmt::Val(_, e) | Stmt::Def(_, e) => e,
                })
                .collect();
            v.push(result);
            v
        }
        NodeKind::Raw(_)
        | NodeKind::Bool(_)
        | NodeKind::Int(_)
        | NodeKind::Num(_)
        | NodeKind::Const(_)
        | NodeKind::Val(_)
        | NodeKind::GetVar(..)
        | NodeKind::Leaf(_) => vec![],
    }
}
