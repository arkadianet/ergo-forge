//! Shared reading of **box references** out of a lifted tree.
//!
//! Extracted from `lints::unbound_box_reserves` so the single-tree lint, the
//! protocol map's edge typing, and (later) the drain hunt all agree on what
//! "this expression names a box", "this expression reads a box's reserves"
//! and "this box is pinned by an NFT" mean. One definition, three callers —
//! a divergence here would make the map's set-level finding disagree with the
//! lint that motivated it.
//!
//! The vocabulary:
//!
//! - a **box key** — `"SELF"`, `"INPUTS(0)"`, `"OUTPUTS(1)"`,
//!   `"CONTEXT.dataInputs(0)"` — the only boxes these analyses can name;
//! - a **reserve read** — `.value` or a token *amount* `.tokens(i)._2`;
//! - **value maths** — a reserve read feeding an arithmetic or comparison
//!   operator, looking through casts and nested arithmetic;
//! - an **NFT slot** — the three spellings of `b.tokens(0)`'s identity that a
//!   singleton-NFT check is written in.

use std::collections::HashMap;

use crate::audit::children;
use crate::decompile::Stmt;
use crate::{Node, NodeKind};

/// `val` bindings in the tree, by the name lift assigned them.
pub type Vals<'a> = HashMap<String, &'a Node>;

/// How far to follow wrappers when hunting reserve reads inside an operand.
pub const OPERAND_DEPTH: u32 = 8;

/// Collect every `val`/`def` binding in the tree.
///
/// Names are collected across the whole tree rather than per block; lift
/// assigns globally-unique names, so shadowing does not arise in practice.
pub fn collect_vals<'a>(n: &'a Node, vals: &mut Vals<'a>) {
    if let NodeKind::Block(stmts, _) = &n.kind {
        for st in stmts {
            let (name, expr) = match st {
                Stmt::Val(n, e) | Stmt::Def(n, e) => (n, e),
            };
            vals.insert(name.clone(), expr);
        }
    }
    for c in children(n) {
        collect_vals(c, vals);
    }
}

/// Follow `val` references to the expression they stand for.
pub fn deref<'a>(n: &'a Node, vals: &Vals<'a>) -> &'a Node {
    let mut cur = n;
    // Bounded by the number of bindings: a chain longer than that must revisit
    // a name, so it is cyclic. `val` definitions cannot be cyclic in a tree the
    // lift produced, but a corrupt one must not spin here — and the bound never
    // cuts a legitimate chain short, however deep the tree.
    for _ in 0..=vals.len() {
        match &cur.kind {
            NodeKind::Val(name) => match vals.get(name) {
                Some(d) => cur = d,
                None => return cur,
            },
            _ => return cur,
        }
    }
    cur
}

/// `obj(index)` in either shape the lift produces for `ByIndex`.
pub fn as_indexed(n: &Node) -> Option<(&Node, &Node)> {
    match &n.kind {
        NodeKind::ApplyFn(o, args) if args.len() == 1 => Some((o, &args[0])),
        NodeKind::Index(o, i, _) => Some((o, i)),
        _ => None,
    }
}

/// The collection name a positional box is drawn from, if any.
pub fn box_collection(n: &Node, vals: &Vals) -> Option<&'static str> {
    match &deref(n, vals).kind {
        NodeKind::Leaf("INPUTS") => Some("INPUTS"),
        NodeKind::Leaf("OUTPUTS") => Some("OUTPUTS"),
        NodeKind::Method(o, name, args)
            if name == "dataInputs"
                && args.is_empty()
                && matches!(deref(o, vals).kind, NodeKind::Leaf("CONTEXT")) =>
        {
            Some("CONTEXT.dataInputs")
        }
        NodeKind::Prop(o, name)
            if name == "dataInputs" && matches!(deref(o, vals).kind, NodeKind::Leaf("CONTEXT")) =>
        {
            Some("CONTEXT.dataInputs")
        }
        _ => None,
    }
}

/// Is this index one the *transaction builder* fixes — a literal, or a
/// context variable? A computed index is out of scope.
pub fn is_static_index(n: &Node, vals: &Vals) -> bool {
    matches!(
        deref(n, vals).kind,
        NodeKind::Int(_) | NodeKind::Num(_) | NodeKind::Const(_) | NodeKind::GetVar(..)
    )
}

/// Canonical key for a box obtained by position, e.g. `"INPUTS(0)"`.
pub fn positional_key(n: &Node, vals: &Vals) -> Option<String> {
    let (coll, idx) = as_indexed(deref(n, vals))?;
    let name = box_collection(coll, vals)?;
    if !is_static_index(idx, vals) {
        return None;
    }
    Some(format!(
        "{name}({})",
        crate::decompile::print(deref(idx, vals))
    ))
}

/// Canonical key for the boxes these analyses reason about: positional ones
/// and `SELF`. Anything else is unnamed here.
pub fn box_key(n: &Node, vals: &Vals) -> Option<String> {
    if matches!(deref(n, vals).kind, NodeKind::Leaf("SELF")) {
        return Some("SELF".into());
    }
    positional_key(n, vals)
}

/// Does this expression reach a positionally-obtained box anywhere inside?
///
/// Used to reject an NFT "binding" whose right-hand side is itself
/// attacker-placed: `INPUTS(0).tokens(0)._1 == INPUTS(1).tokens(0)._1` binds
/// neither box to anything real.
pub fn mentions_positional(n: &Node, vals: &Vals, depth: u32) -> bool {
    if depth == 0 {
        return false;
    }
    let d = deref(n, vals);
    if positional_key(d, vals).is_some() {
        return true;
    }
    children(d)
        .into_iter()
        .any(|c| mentions_positional(c, vals, depth - 1))
}

/// If `n` reads a box's reserves: the box, plus how the read is spelled
/// (`".value"`, `".tokens(2)._2"`) so a finding can name it without the
/// lift's synthetic `val` names.
///
/// Two shapes count: the nanoERG balance `b.value`, and a token *amount*
/// `b.tokens(i)._2`. A token *id* (`._1`) is an identity check, not a reserve.
pub fn reserve_read<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<(&'a Node, String)> {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name) if name == "value" => Some((b, ".value".into())),
        NodeKind::Method(b, name, args) if name == "value" && args.is_empty() => {
            Some((b, ".value".into()))
        }
        NodeKind::Prop(t, name) if name == "_2" => {
            let (coll, idx) = as_indexed(deref(t, vals))?;
            let b = tokens_receiver(coll, vals)?;
            Some((
                b,
                format!(".tokens({})._2", crate::decompile::print(deref(idx, vals))),
            ))
        }
        _ => None,
    }
}

/// Operators whose operands are doing value maths.
pub fn is_value_op(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">="
    )
}

/// Reserve reads reachable from an operand, looking through casts, negation
/// and nested arithmetic — `v.toBigInt * 997L` still reads `v`.
///
/// Pushes `(box expression, read expression, spelling)` triples.
pub fn reads_in_operand<'a>(
    n: &'a Node,
    vals: &Vals<'a>,
    depth: u32,
    out: &mut Vec<(&'a Node, &'a Node, String)>,
) {
    if depth == 0 {
        return;
    }
    let d = deref(n, vals);
    if let Some((b, how)) = reserve_read(d, vals) {
        out.push((b, d, how));
        return;
    }
    let descend = match &d.kind {
        NodeKind::Unary(..) => true,
        NodeKind::Infix(op, ..) => is_value_op(op),
        // No-argument methods are the cast/accessor wrappers (`toBigInt`, …).
        NodeKind::Method(_, _, args) => args.is_empty(),
        _ => false,
    };
    if descend {
        for c in children(d) {
            reads_in_operand(c, vals, depth - 1, out);
        }
    }
}

/// The receiver of a `b.tokens` access, in either shape the lift produces.
pub fn tokens_receiver<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
    match &deref(n, vals).kind {
        NodeKind::Method(b, m, args) if m == "tokens" && args.is_empty() => Some(b),
        NodeKind::Prop(b, m) if m == "tokens" => Some(b),
        _ => None,
    }
}

/// An expression that pins down a box's identity by its tokens. Returns the
/// box's key.
///
/// Three spellings, all equivalent for the purpose: an equality on them says
/// the box holds a particular token id in `tokens(0)`, and a singleton NFT
/// lives at exactly one box.
///
/// - `b.tokens(0)._1` — the id alone, the canonical NFT assertion;
/// - `b.tokens(0)` — the whole `(id, amount)` pair;
/// - `b.tokens` — the whole collection, which pins index 0 with it.
pub fn nft_slot_of(n: &Node, vals: &Vals) -> Option<String> {
    nft_slot_ref(n, vals).map(|(k, _)| k)
}

/// Like [`nft_slot_of`], also returning the box expression itself — the map
/// needs the node to key an edge's site by, not only the printed key.
pub fn nft_slot_ref<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<(String, &'a Node)> {
    let d = deref(n, vals);
    // `b.tokens`
    if let Some(b) = tokens_receiver(d, vals) {
        return box_key(b, vals).map(|k| (k, b));
    }
    // `b.tokens(0)._1`, unwrapping the `._1`; `b.tokens(0)` keeps `inner = d`.
    let inner = match &d.kind {
        NodeKind::Prop(t, name) if name == "_1" => deref(t, vals),
        _ => d,
    };
    let (coll, idx) = as_indexed(inner)?;
    if crate::decompile::print(deref(idx, vals)) != "0" {
        return None;
    }
    let b = tokens_receiver(coll, vals)?;
    box_key(b, vals).map(|k| (k, b))
}

/// `b.propositionBytes` and its synonyms — the script a box is locked by.
pub fn script_of<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name)
            if name == "propositionBytes" || name == "ergoTree" || name == "scriptBytes" =>
        {
            Some(b)
        }
        _ => None,
    }
}
