//! Reading **box references** out of one contract's lifted tree.
//!
//! This is the map's half of the edge typing: for every box a tree names, how
//! does the tree establish that box's identity, and does it do arithmetic on
//! that box's reserves? The other half — which node sits at the far end — is
//! graph work and lives in [`super::graph`].
//!
//! Everything structural is borrowed from [`crate::audit::boxrefs`], so the
//! map and the single-tree `unbound-box-reserves` lint cannot drift apart.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::audit::boxrefs::{
    box_key, collect_vals, deref, is_value_op, mentions_positional, nft_slot_ref, reads_in_operand,
    script_of, tokens_receiver, Vals, OPERAND_DEPTH,
};
use crate::audit::children;
use crate::{Node, NodeKind};

/// An identity dimension a binding pins down.
///
/// The uncovered dimensions are where the bug class lives: the USE pool
/// checked its successor's script and token *ids* and never its token
/// *amounts*, so `value` stayed open and the reserves walked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Cover {
    /// The box's script (`propositionBytes`).
    Script,
    /// The token id in `tokens(0)`.
    TokenId,
    /// The nanoERG balance.
    Value,
    /// At least one register.
    Registers,
}

impl Cover {
    /// Lowercase name used in the canonical JSON.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Cover::Script => "script",
            Cover::TokenId => "tokenId",
            Cover::Value => "value",
            Cover::Registers => "registers",
        }
    }
}

/// How a tree establishes the identity of a box it names.
///
/// Declared strongest-first so `Ord` picks the winner for scoring when a site
/// carries more than one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Binding {
    /// `B.tokens(0)._1 == <constant>` — pins a **token id**. A unique box
    /// only with singleton evidence; see [`super::TokenClass`].
    Nft,
    /// `blake2b256(B.propositionBytes) == <constant>`, or the propositionBytes
    /// compared to a literal script. Pins B's **code**; any box with that
    /// script qualifies.
    ScriptHash,
    /// `B.propositionBytes == SELF.propositionBytes`. Pins B's **script bytes
    /// only** — not its value, tokens or registers.
    SelfSuccessor,
    /// B is read via `CONTEXT.dataInputs(n)`. Read for its data rather than
    /// spent; typed by the same rules, and equally unpinned when positional.
    DataInput,
    /// A bare `INPUTS(n)` / `OUTPUTS(n)` with no identity check. Pins
    /// **nothing**: whoever builds the transaction chooses what sits there.
    Positional,
}

impl Binding {
    /// Lowercase name used in the canonical JSON.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Binding::Nft => "nft",
            Binding::ScriptHash => "script-hash",
            Binding::SelfSuccessor => "self-successor",
            Binding::DataInput => "data-input",
            Binding::Positional => "positional",
        }
    }
}

/// One box slot a tree names — `INPUTS(0)`, `OUTPUTS(1)`,
/// `CONTEXT.dataInputs(0)`, `SELF` — with everything the tree says about it.
#[derive(Debug, Clone, Default)]
pub struct BoxRef {
    /// Canonical slot key, the edge's `site`.
    pub key: String,
    /// Lift-local node id of the first occurrence, so a finding can anchor.
    pub node_id: u64,
    /// The slot is read from `CONTEXT.dataInputs`.
    pub data_input: bool,
    /// The slot is `SELF`.
    pub is_self: bool,
    /// Token-id constants asserted at the slot's `tokens(0)` identity.
    pub nft_constants: BTreeSet<String>,
    /// `blake2b256(propositionBytes)` values asserted for the slot. A literal
    /// `propositionBytes == <bytes>` is hashed and recorded here too, so both
    /// spellings resolve against the same index.
    pub script_hashes: BTreeSet<String>,
    /// The slot's script is asserted equal to `SELF`'s.
    pub self_successor: bool,
    /// Identity dimensions the tree pins for this slot.
    pub covers: BTreeSet<Cover>,
    /// The tree does arithmetic or comparison on the slot's reserves —
    /// `.value` or a token amount.
    pub value_math: bool,
}

impl BoxRef {
    /// The binding that wins for scoring: the strongest one asserted.
    #[must_use]
    pub fn strongest(&self) -> Binding {
        if !self.nft_constants.is_empty() {
            Binding::Nft
        } else if !self.script_hashes.is_empty() {
            Binding::ScriptHash
        } else if self.self_successor {
            Binding::SelfSuccessor
        } else if self.data_input {
            Binding::DataInput
        } else {
            Binding::Positional
        }
    }

    /// Does the tree pin this slot's identity at all?
    #[must_use]
    pub fn pinned(&self) -> bool {
        !self.nft_constants.is_empty() || !self.script_hashes.is_empty() || self.self_successor
    }
}

/// Everything one tree says about the boxes it names.
#[derive(Debug, Clone, Default)]
pub struct TreeRefs {
    /// Slots, by canonical key.
    pub slots: BTreeMap<String, BoxRef>,
    /// Every 32-byte `Coll[Byte]` constant in the tree, in first-seen order
    /// deduplicated — the raw material for classification.
    pub constants: BTreeSet<String>,
}

/// A 32-byte `Coll[Byte]` constant's hex, if `n` is one.
///
/// The lift renders byte collections as `fromBase16("<hex>")`; 32 bytes is
/// the width of every identifier on Ergo — box ids, token ids, digests.
#[must_use]
pub fn byte_constant(n: &Node) -> Option<String> {
    let NodeKind::Const(s) = &n.kind else {
        return None;
    };
    let hex = s.strip_prefix("fromBase16(\"")?.strip_suffix("\")")?;
    (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| hex.to_ascii_lowercase())
}

/// Read every box reference out of a lifted tree.
#[must_use]
pub fn tree_refs(root: &Node) -> TreeRefs {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);

    let mut out = TreeRefs::default();
    collect_slots(root, &vals, &mut out);
    collect_constants(root, &mut out.constants);
    scan_bindings(root, &vals, &mut out);
    scan_value_math(root, &vals, &mut out);
    out
}

/// Register the slot `key` if new, and hand it back for mutation.
fn slot(out: &mut TreeRefs, key: String, node_id: u64) -> &mut BoxRef {
    let data_input = key.starts_with("CONTEXT.dataInputs");
    let is_self = key == "SELF";
    out.slots.entry(key.clone()).or_insert_with(|| BoxRef {
        key,
        node_id,
        data_input,
        is_self,
        ..BoxRef::default()
    })
}

fn collect_slots(n: &Node, vals: &Vals, out: &mut TreeRefs) {
    // `deref` resolves a `val` reference to the expression it stands for, so
    // `v1` and its definition `OUTPUTS(0)` register the same slot once.
    if let Some(key) = box_key(n, vals) {
        slot(out, key, deref(n, vals).id);
    }
    for c in children(n) {
        collect_slots(c, vals, out);
    }
}

fn collect_constants(n: &Node, out: &mut BTreeSet<String>) {
    if let Some(hex) = byte_constant(n) {
        out.insert(hex);
    }
    for c in children(n) {
        collect_constants(c, out);
    }
}

/// `blake2b256(x)` — the receiver `x`, if `n` is that call.
fn hash_arg<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
    match &deref(n, vals).kind {
        NodeKind::Global(name, args) if name == "blake2b256" && args.len() == 1 => Some(&args[0]),
        NodeKind::Method(o, name, args) if name == "blake2b256" && args.is_empty() => Some(o),
        _ => None,
    }
}

/// `b.RN` — the box and the register name, if `n` is a register read.
fn register_of<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
    let d = deref(n, vals);
    let (recv, name) = match &d.kind {
        NodeKind::Prop(b, name) => (b, name.as_str()),
        NodeKind::Method(b, name, args) if args.is_empty() => (b, name.as_str()),
        _ => return None,
    };
    let is_reg =
        name.len() >= 2 && name.starts_with('R') && name[1..2].chars().all(|c| c.is_ascii_digit());
    is_reg.then_some(&**recv)
}

/// `b.value`, as an identity read rather than a reserve read.
fn value_of<'a>(n: &'a Node, vals: &Vals<'a>) -> Option<&'a Node> {
    match &deref(n, vals).kind {
        NodeKind::Prop(b, name) if name == "value" => Some(b),
        NodeKind::Method(b, name, args) if name == "value" && args.is_empty() => Some(b),
        _ => None,
    }
}

fn scan_bindings(n: &Node, vals: &Vals, out: &mut TreeRefs) {
    if let NodeKind::Infix("==", a, b) = &n.kind {
        for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
            binding_pair(lhs, rhs, vals, out);
        }
    }
    for c in children(n) {
        scan_bindings(c, vals, out);
    }
}

fn binding_pair(lhs: &Node, rhs: &Node, vals: &Vals, out: &mut TreeRefs) {
    // `B.tokens(0)._1 == <32-byte constant>` — the NFT assertion.
    if let Some((key, node)) = nft_slot_ref(lhs, vals) {
        if let Some(hex) = byte_constant(deref(rhs, vals)) {
            let s = slot(out, key, node.id);
            s.nft_constants.insert(hex);
            s.covers.insert(Cover::TokenId);
            return;
        }
        // Bound to something the spender cannot move (a register of SELF, a
        // hashed literal) still pins the token id, as long as the right-hand
        // side is not itself read off an attacker-placed box.
        if !mentions_positional(rhs, vals, OPERAND_DEPTH) && tokens_receiver(rhs, vals).is_none() {
            let s = slot(out, key, node.id);
            s.covers.insert(Cover::TokenId);
        }
        return;
    }
    // `blake2b256(B.propositionBytes) == <constant>`.
    if let Some(inner) = hash_arg(lhs, vals) {
        if let (Some(bx), Some(hex)) = (script_of(inner, vals), byte_constant(deref(rhs, vals))) {
            if let Some(key) = box_key(bx, vals) {
                let s = slot(out, key, deref(bx, vals).id);
                s.script_hashes.insert(hex);
                s.covers.insert(Cover::Script);
            }
            return;
        }
    }
    if let Some(bx) = script_of(lhs, vals) {
        let Some(key) = box_key(bx, vals) else {
            return;
        };
        let bx_id = deref(bx, vals).id;
        // `B.propositionBytes == SELF.propositionBytes` — the self-successor.
        if let Some(other) = script_of(rhs, vals) {
            if box_key(other, vals).as_deref() == Some("SELF") && key != "SELF" {
                let s = slot(out, key, bx_id);
                s.self_successor = true;
                s.covers.insert(Cover::Script);
            }
            return;
        }
        // `B.propositionBytes == <literal script>` — the same pin, spelled
        // with the bytes rather than their digest. Hashed so both spellings
        // resolve against one index.
        if let NodeKind::Const(c) = &deref(rhs, vals).kind {
            if let Some(hex) = c
                .strip_prefix("fromBase16(\"")
                .and_then(|h| h.strip_suffix("\")"))
            {
                if let Ok(bytes) = hex::decode(hex) {
                    let s = slot(out, key, bx_id);
                    s.script_hashes.insert(hex::encode(
                        ergo_primitives::digest::blake2b256(&bytes).as_bytes(),
                    ));
                    s.covers.insert(Cover::Script);
                }
            }
        }
        return;
    }
    // `B.value == <something the spender cannot choose>` pins the balance.
    if let Some(bx) = value_of(lhs, vals) {
        if !mentions_positional(rhs, vals, OPERAND_DEPTH) {
            if let Some(key) = box_key(bx, vals) {
                let id = deref(bx, vals).id;
                slot(out, key, id).covers.insert(Cover::Value);
            }
        }
        return;
    }
    // `B.R4 == <something the spender cannot choose>` pins a register.
    if let Some(bx) = register_of(lhs, vals) {
        if !mentions_positional(rhs, vals, OPERAND_DEPTH) {
            if let Some(key) = box_key(bx, vals) {
                let id = deref(bx, vals).id;
                slot(out, key, id).covers.insert(Cover::Registers);
            }
        }
    }
}

/// Mark every slot whose reserves feed an arithmetic or comparison operator.
///
/// Same reader the `unbound-box-reserves` lint uses, so "does A do value
/// maths on B" means one thing across the forge.
fn scan_value_math(n: &Node, vals: &Vals, out: &mut TreeRefs) {
    if let NodeKind::Infix(op, a, b) = &n.kind {
        if is_value_op(op) {
            let mut reads = Vec::new();
            reads_in_operand(a, vals, OPERAND_DEPTH, &mut reads);
            reads_in_operand(b, vals, OPERAND_DEPTH, &mut reads);
            for (box_node, _, _) in reads {
                if let Some(key) = box_key(box_node, vals) {
                    let id = deref(box_node, vals).id;
                    slot(out, key, id).value_math = true;
                }
            }
        }
    }
    for c in children(n) {
        scan_value_math(c, vals, out);
    }
}
