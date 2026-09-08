//! Reserves read off a box picked by **position**, with no NFT binding it.
//!
//! This is the class behind the 2026-09-08 USE/Dexy LP drain on mainnet. The
//! LP swap script read the pool box as `INPUTS(0)` and did the constant-
//! product math on `INPUTS(0).value` and `INPUTS(0).tokens(i)._2` — but
//! nothing in the script said *which* box that had to be. A position in the
//! input list is chosen by whoever builds the transaction, so an attacker
//! placed a decoy box (their own, with whatever reserves made the ratio
//! check pass) at index 0 and spent the real pool alongside it. The maths was
//! correct; it was simply applied to the wrong box.
//!
//! The fix, and what this lint looks for, is a **singleton NFT**: a token
//! minted in quantity one, kept in `tokens(0)` of the pool box, and asserted
//! by the script (`INPUTS(0).tokens(0)._1 == poolNft`). Because the token is
//! unique on chain, only the genuine box can satisfy it.
//!
//! A box is reported when all three hold:
//!
//! 1. it is obtained positionally — `INPUTS(n)`, `OUTPUTS(n)` or
//!    `CONTEXT.dataInputs(n)` for a literal or context-variable index,
//!    directly or through a `val`;
//! 2. its **reserves** — `.value`, or a token *amount* `.tokens(i)._2` — feed
//!    an arithmetic or comparison operator. A box merely passed along, or
//!    whose `propositionBytes` are compared, does not count;
//! 3. nowhere in the tree is it bound by NFT: no `tokens(0)._1 == <constant>`,
//!    and no `tokens(0)._1` equality with a box that is itself bound.
//!
//! Two shapes are exempt, because they are how a correct self-validating pool
//! is written:
//!
//! - **SELF** — never positional, so never reported. A script reading its own
//!   reserves is reading the box it guards.
//! - **SELF's successor** — an output whose `propositionBytes` (or
//!   `ergoTree`/`scriptBytes`) are asserted equal to `SELF`'s. The script
//!   itself runs again on that box, so it is not a substitutable decoy.
//!
//! Three spellings of the binding count: `b.tokens(0)._1 == x`, the whole pair
//! `b.tokens(0) == x`, and the whole collection `b.tokens == x.tokens`. All
//! three pin `tokens(0)`, which is where a singleton NFT is kept by convention.
//!
//! Known gaps (deliberate): an NFT held at a token index other than 0, and a
//! box identified by an `exists`/`forall` search over the inputs rather than by
//! a fixed index, are not recognised — such a contract is a false positive.
//! `val` names are collected across the whole tree rather than per block; lift
//! assigns them globally-unique names, so shadowing does not arise in practice.

use std::collections::{HashMap, HashSet};

use crate::audit::{children, Finding, Severity};
use crate::decompile::Stmt;
use crate::{Node, NodeKind};

/// `val` bindings in the tree, by the name lift assigned them.
type Vals<'a> = HashMap<String, &'a Node>;

/// How far to follow wrappers when hunting reserve reads inside an operand.
const OPERAND_DEPTH: u32 = 8;

/// Report every positionally-obtained box whose reserves drive value maths
/// without an NFT binding it.
#[must_use]
pub fn unbound_box_reserves(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);

    let bound = bound_boxes(root, &vals);

    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    report(root, &vals, &bound, &mut seen, &mut out);
    out
}

// ── val bindings ─────────────────────────────────────────────────────────────

fn collect_vals<'a>(n: &'a Node, vals: &mut Vals<'a>) {
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
fn deref<'a>(n: &'a Node, vals: &Vals<'a>) -> &'a Node {
    let mut cur = n;
    // Bounded: `val` definitions cannot be cyclic, but a corrupt tree must
    // not spin here.
    for _ in 0..64 {
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

// ── box identity ─────────────────────────────────────────────────────────────

/// `obj(index)` in either shape the lift produces for `ByIndex`.
fn as_indexed(n: &Node) -> Option<(&Node, &Node)> {
    match &n.kind {
        NodeKind::ApplyFn(o, args) if args.len() == 1 => Some((o, &args[0])),
        NodeKind::Index(o, i, _) => Some((o, i)),
        _ => None,
    }
}

/// The collection name a positional box is drawn from, if any.
fn box_collection(n: &Node, vals: &Vals) -> Option<&'static str> {
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
/// context variable? A computed index is out of scope for this lint.
fn is_static_index(n: &Node, vals: &Vals) -> bool {
    matches!(
        deref(n, vals).kind,
        NodeKind::Int(_) | NodeKind::Num(_) | NodeKind::Const(_) | NodeKind::GetVar(..)
    )
}

/// Canonical key for a box obtained by position, e.g. `"INPUTS(0)"`.
fn positional_key(n: &Node, vals: &Vals) -> Option<String> {
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

/// Canonical key for the boxes this lint reasons about: positional ones and
/// `SELF`. Anything else is unnamed here and never reported.
fn box_key(n: &Node, vals: &Vals) -> Option<String> {
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
fn mentions_positional(n: &Node, vals: &Vals, depth: u32) -> bool {
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

// ── reserve reads ────────────────────────────────────────────────────────────

/// If `n` reads a box's reserves: the box, plus how the read is spelled
/// (`".value"`, `".tokens(2)._2"`) so a finding can name it without the
/// lift's synthetic `val` names.
///
/// Two shapes count: the nanoERG balance `b.value`, and a token *amount*
/// `b.tokens(i)._2`. A token *id* (`._1`) is an identity check, not a reserve.
fn reserve_read<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<(&'a Node, String)> {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name) if name == "value" => Some((b, ".value".into())),
        NodeKind::Method(b, name, args) if name == "value" && args.is_empty() => {
            Some((b, ".value".into()))
        }
        NodeKind::Prop(t, name) if name == "_2" => {
            let (coll, idx) = as_indexed(deref(t, vals))?;
            let b = match &deref(coll, vals).kind {
                NodeKind::Method(b, m, args) if m == "tokens" && args.is_empty() => b,
                NodeKind::Prop(b, m) if m == "tokens" => b,
                _ => return None,
            };
            Some((
                b,
                format!(".tokens({})._2", crate::decompile::print(deref(idx, vals))),
            ))
        }
        _ => None,
    }
}

/// Operators whose operands are doing value maths.
fn is_value_op(op: &str) -> bool {
    matches!(
        op,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">="
    )
}

/// Reserve reads reachable from an operand, looking through casts, negation
/// and nested arithmetic — `v.toBigInt * 997L` still reads `v`.
fn reads_in_operand<'a>(
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

// ── pass 1: what is already bound ────────────────────────────────────────────

/// The receiver of a `b.tokens` access, in either shape the lift produces.
fn tokens_receiver<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
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
fn nft_slot_of(n: &Node, vals: &Vals) -> Option<String> {
    let d = deref(n, vals);
    // `b.tokens`
    if let Some(b) = tokens_receiver(d, vals) {
        return box_key(b, vals);
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
    box_key(tokens_receiver(coll, vals)?, vals)
}

/// `b.propositionBytes` and its synonyms — the script a box is locked by.
fn script_of<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name)
            if name == "propositionBytes" || name == "ergoTree" || name == "scriptBytes" =>
        {
            Some(b)
        }
        _ => None,
    }
}

/// Boxes the script pins down: NFT-bound ones, `SELF`, and `SELF`'s successor.
///
/// Collected over the whole tree — a binding asserted in one conjunct pins the
/// box for every read of it — then closed transitively, so
/// `A.tokens(0)._1 == B.tokens(0)._1` carries `B`'s binding to `A`.
fn bound_boxes(root: &Node, vals: &Vals) -> HashSet<String> {
    let mut bound: HashSet<String> = HashSet::new();
    bound.insert("SELF".into());
    let mut edges: Vec<(String, String)> = Vec::new();
    scan_bindings(root, vals, &mut bound, &mut edges);

    // Transitive closure over the token-id equalities.
    loop {
        let mut grew = false;
        for (a, b) in &edges {
            if bound.contains(b) && bound.insert(a.clone()) {
                grew = true;
            }
        }
        if !grew {
            return bound;
        }
    }
}

fn scan_bindings(
    n: &Node,
    vals: &Vals,
    bound: &mut HashSet<String>,
    edges: &mut Vec<(String, String)>,
) {
    if let NodeKind::Infix("==", a, b) = &n.kind {
        for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
            if let Some(key) = nft_slot_of(lhs, vals) {
                match nft_slot_of(rhs, vals) {
                    // Bound to another box's NFT slot: inherits its binding.
                    Some(other) => edges.push((key, other)),
                    // Bound to something the spender cannot move — a constant,
                    // a register of SELF — as long as it is not itself read off
                    // a positional box.
                    None => {
                        if !mentions_positional(rhs, vals, OPERAND_DEPTH) {
                            bound.insert(key);
                        }
                    }
                }
            }
            // `OUTPUTS(k).propositionBytes == SELF.propositionBytes`: the box is
            // this very script's successor, not a substitutable decoy.
            if let (Some(x), Some(y)) = (script_of(lhs, vals), script_of(rhs, vals)) {
                if box_key(y, vals).as_deref() == Some("SELF") {
                    if let Some(key) = positional_key(x, vals) {
                        bound.insert(key);
                    }
                }
            }
        }
    }
    for c in children(n) {
        scan_bindings(c, vals, bound, edges);
    }
}

// ── pass 2: report ───────────────────────────────────────────────────────────

fn report(
    n: &Node,
    vals: &Vals,
    bound: &HashSet<String>,
    seen: &mut HashSet<String>,
    out: &mut Vec<Finding>,
) {
    if let NodeKind::Infix(op, a, b) = &n.kind {
        if is_value_op(op) {
            let mut reads = Vec::new();
            reads_in_operand(a, vals, OPERAND_DEPTH, &mut reads);
            reads_in_operand(b, vals, OPERAND_DEPTH, &mut reads);
            for (box_node, read_node, how) in reads {
                let Some(key) = positional_key(box_node, vals) else {
                    continue;
                };
                if bound.contains(&key) || !seen.insert(key.clone()) {
                    continue;
                }
                out.push(Finding {
                    lint: "unbound-box-reserves",
                    severity: Severity::High,
                    node_id: read_node.id,
                    ir_id: None,
                    message: format!(
                        "reserves of {key} drive value math but this box is never bound by a \
                         singleton NFT (no tokens(0)._1 == <nft> check); a transaction can place \
                         a different box at this index. Bind it by its NFT."
                    ),
                    // Spelled from the resolved box key, not `print`: the lift
                    // hoists sub-expressions into `val`s, so the read node
                    // itself renders as `v3.value`, which names nothing.
                    snippet: format!("{key}{how}"),
                });
            }
        }
    }
    for c in children(n) {
        report(c, vals, bound, seen, out);
    }
}
