//! Dynamic code bytes without a recognised immutable authentication anchor.
//!
//! Covers Global executeFromVar[T](id), Global/Method deserialize[T] and
//! deserializeTo[T], and Global substConstants(bytes, positions, values).
//! Literal or SELF-rooted source bytes count as anchored. Otherwise an exact
//! byte equality, or a blake2b256/sha256 digest equality, must compare those
//! same bytes to a literal/SELF-rooted value somewhere in the tree. getVar ids,
//! types and input-specific context receivers remain distinct through aliases.
//!
//! Known limits (deliberate): syntax, not enforcement or reachability. Negated,
//! unused and branch-local equalities count; arbitrary/transitive comparisons,
//! authenticated companion data and unknown expressions are undecided.
//! substConstants is a byte-construction hook, not itself execution; this lint
//! checks its template provenance, not the authority of replacement constants.
//! Raw DeserializeRegister placeholders are outside the already-lifted AST.
//! A clean result never proves code safe or authorised on every spending path.

use std::collections::HashSet;

use crate::audit::boxrefs::{collect_vals, deref, Vals};
use crate::audit::{children, snippet, Finding, Severity};
use crate::{Node, NodeKind};

/// Report dynamic code/template sites with no recognised byte authentication.
#[must_use]
pub fn unauthenticated_code_execution(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut authenticated = HashSet::new();
    comparisons(root, &vals, &mut authenticated);
    let mut out = Vec::new();
    scan(root, &vals, &authenticated, &mut out);
    out
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Key {
    Atom(&'static str, String),
    Expr(&'static str, String, Vec<Key>),
}

// Structural rather than a truncated snippet; unsupported expressions never
// acquire an authentication key. Lambda names and cyclic vals remain unknown.
fn key(n: &Node, vals: &Vals, depth: u32) -> Option<Key> {
    if depth == 0 {
        return None;
    }
    let atom = |tag, value| Some(Key::Atom(tag, value));
    let expr = |tag, name, nodes: Vec<&Node>| {
        Some(Key::Expr(
            tag,
            name,
            nodes
                .into_iter()
                .map(|c| key(c, vals, depth - 1))
                .collect::<Option<_>>()?,
        ))
    };
    match &deref(n, vals).kind {
        NodeKind::GetVar(id, tpe) => atom("getVar", format!("{id}:{tpe}")),
        NodeKind::Int(i) => atom("integer", i.to_string()),
        NodeKind::Num(s) => atom("integer", s.clone()),
        NodeKind::Const(s) => atom("constant", s.clone()),
        NodeKind::Bool(b) => atom("boolean", b.to_string()),
        NodeKind::Leaf(s) => atom("leaf", (*s).into()),
        NodeKind::Prop(b, name) => expr("property", name.clone(), vec![b]),
        NodeKind::Method(b, name, args) => expr(
            "method",
            name.clone(),
            std::iter::once(b.as_ref()).chain(args).collect(),
        ),
        NodeKind::GetRegDyn(b, tpe, args) => expr(
            "register",
            tpe.clone(),
            std::iter::once(b.as_ref()).chain(args).collect(),
        ),
        NodeKind::ApplyFn(b, args) => expr(
            "index",
            String::new(),
            std::iter::once(b.as_ref()).chain(args).collect(),
        ),
        NodeKind::Index(b, i, None) => expr("index", String::new(), vec![b, i]),
        NodeKind::Global(name, args) => expr("global", name.clone(), args.iter().collect()),
        NodeKind::Coll(tpe, items) => expr("collection", tpe.clone(), items.iter().collect()),
        _ => None,
    }
}

fn anchor(n: &Node, vals: &Vals) -> bool {
    super::trust_assumptions::anchor(n, vals, 32)
        || matches!(&deref(n, vals).kind, NodeKind::Coll(_, items)
            if items.iter().all(|c| matches!(deref(c, vals).kind,
                NodeKind::Bool(_) | NodeKind::Int(_) | NodeKind::Num(_) | NodeKind::Const(_))))
}

fn comparisons(n: &Node, vals: &Vals, authenticated: &mut HashSet<Key>) {
    if let NodeKind::Infix("==", a, b) = &n.kind {
        for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
            if anchor(rhs, vals) {
                let lhs = deref(lhs, vals);
                let bytes = match &lhs.kind {
                    NodeKind::Global(name, args)
                        if matches!(name.as_str(), "blake2b256" | "sha256") && args.len() == 1 =>
                    {
                        &args[0]
                    }
                    _ => lhs,
                };
                if let Some(k) = key(bytes, vals, 64) {
                    authenticated.insert(k);
                }
            }
        }
    }
    for c in children(n) {
        comparisons(c, vals, authenticated);
    }
}

fn base(name: &str) -> &str {
    name.split('[').next().unwrap_or(name)
}

fn scan(n: &Node, vals: &Vals, authenticated: &HashSet<Key>, out: &mut Vec<Finding>) {
    let mut context_bytes = None;
    let bytes = match &n.kind {
        NodeKind::Global(name, args) if base(name) == "executeFromVar" && args.len() == 1 => {
            let id = match &deref(&args[0], vals).kind {
                NodeKind::Int(id) => Some(*id),
                NodeKind::Num(s) => s.parse().ok(),
                _ => None,
            };
            context_bytes = id.map(|id| Node {
                id: n.id,
                kind: NodeKind::Method(
                    Box::new(Node {
                        id: n.id,
                        kind: NodeKind::GetVar(id, "Coll[Byte]".into()),
                    }),
                    "get".into(),
                    vec![],
                ),
            });
            // Unknown ids still deserve a review; their own expression cannot
            // serve as the code bytes' key or an immutable authentication anchor.
            context_bytes.as_ref()
        }
        NodeKind::Global(name, args)
            if matches!(
                base(name),
                "deserialize" | "deserializeTo" | "substConstants"
            ) =>
        {
            args.first()
        }
        NodeKind::Method(b, name, _) if matches!(base(name), "deserialize" | "deserializeTo") => {
            Some(b.as_ref())
        }
        _ => None,
    };
    let unknown_context = matches!(&n.kind, NodeKind::Global(name, _) if base(name) == "executeFromVar")
        && context_bytes.is_none();
    if unknown_context
        || bytes.is_some_and(|b| {
            !anchor(b, vals) && !key(b, vals, 64).is_some_and(|k| authenticated.contains(&k))
        })
    {
        out.push(Finding {
            triage: Default::default(),
            lint: "unauthenticated-code-execution",
            severity: Severity::Medium,
            node_id: n.id,
            ir_id: None,
            message: "Dynamic code or template bytes have no recognised equality to a literal or \
                SELF-rooted value (directly or by digest). Review the bytes' provenance and \
                authorisation at this execution/construction site; this syntax observation is \
                not evidence of exploitability."
                .into(),
            snippet: snippet(n),
        });
    }
    for c in children(n) {
        scan(c, vals, authenticated, out);
    }
}
