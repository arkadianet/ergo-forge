//! Adversarial transaction experiments over the Play engine.
//!
//! Ergo lets the *spender* choose the order and contents of a transaction's
//! inputs, data inputs and outputs. A script that reads a box by position and
//! never checks that box's identity can be fed a decoy the spender placed. This
//! module mutates a drafted Play transaction the way a spender could, reruns
//! every input's script through the **same** [`crate::play::apply`] reducer,
//! and reports which scripts changed verdict.
//!
//! Every result is synthetic and carries `nodeValidated: false`, exactly like
//! Play. Reproducing a historical incident here is a synthetic reconstruction
//! against supplied boxes; it establishes neither node acceptance nor a
//! property violation, and it never asserts a live contract is exploitable.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::audit::boxrefs::{
    as_indexed, box_collection, collect_vals, deref, positional_key, Vals,
};
use crate::audit::visit::children;
use crate::decompile::{lift_tree, Node, NodeKind};
use crate::play::{apply, PlayInput, PlayRequest, PlayResult, PlayTx};
use crate::scenario::{ScenarioBox, TokenAmount, TypedValue};
use crate::SandboxError;

/// One adversarial edit applied to a drafted transaction. Operations are pure
/// and deterministic: each rewrites the request, and the sequence is applied in
/// order before the transaction is evaluated once.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum AttackOp {
    /// Reorder the transaction inputs by a permutation of their current
    /// indices. `order` must be a permutation of `0..inputs.len()`.
    ReorderInputs { order: Vec<usize> },
    /// Reorder the data inputs by a permutation of their current indices.
    ReorderDataInputs { order: Vec<usize> },
    /// Insert a decoy box at input slot `at_index`, auto-populated so every
    /// positional access the script at `target_input` makes against that slot
    /// is satisfied. The decoy carries junk tokens and registers only — never a
    /// binding NFT.
    #[serde(rename_all = "camelCase")]
    InsertDecoy {
        at_index: usize,
        target_input: usize,
    },
    /// Replace the data input at `index` with a look-alike: the same registers,
    /// a fresh box id, and no tokens (so any NFT the original carried is gone).
    SwapDataInput { index: usize },
    /// Prepend `by` junk tokens to the input at `input`, shifting every
    /// `tokens(i)` the script reads by that many slots.
    #[serde(rename_all = "camelCase")]
    ShiftTokenIndices { input: usize, by: usize },
    /// Change one field of an input or output box and rerun. `target` is
    /// `"input"` or `"output"`, `field` is `value`, `tokenAmount:<i>` or
    /// `R<n>`.
    TamperField {
        target: String,
        index: usize,
        field: String,
        value: serde_json::Value,
    },
}

/// One input's verdict before and after the operations were applied.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDiff {
    pub box_id: String,
    pub before: String,
    pub after: String,
    /// True when the script's verdict changed between the two drafts.
    pub changed: bool,
}

/// A drafted transaction plus the adversarial operations to apply to it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackRequest {
    #[serde(flatten)]
    pub draft: PlayRequest,
    #[serde(default)]
    pub operations: Vec<AttackOp>,
}

/// The mutated draft's evaluation, plus the per-input diff against the original
/// draft. Carries the same synthetic `ClaimMetadata` as [`PlayResult`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackResult {
    #[serde(flatten)]
    pub result: PlayResult,
    /// The operations as applied, echoed back for a saved experiment.
    pub applied: usize,
    /// Per-input verdict change. Absent when the operations changed the input
    /// set so the two drafts cannot be aligned by position.
    pub diff: Vec<InputDiff>,
}

/// Apply the operations to the draft and evaluate the result, diffing each
/// input's verdict against the un-mutated draft.
pub fn apply_attack(req: &AttackRequest) -> Result<AttackResult, SandboxError> {
    let before = apply(&req.draft)?;

    let mut mutated = req.draft.clone();
    for op in &req.operations {
        apply_op(&mut mutated, op)?;
    }
    let result = apply(&mutated)?;

    // Align by box id: a diff is only meaningful for inputs present in both
    // drafts at the same identity.
    let before_by_id: BTreeMap<&str, &str> = before
        .inputs
        .iter()
        .map(|r| (r.box_id.as_str(), r.verdict))
        .collect();
    let diff = result
        .inputs
        .iter()
        .filter_map(|r| {
            before_by_id.get(r.box_id.as_str()).map(|b| InputDiff {
                box_id: r.box_id.clone(),
                before: (*b).to_string(),
                after: r.verdict.to_string(),
                changed: *b != r.verdict,
            })
        })
        .collect();

    Ok(AttackResult {
        result,
        applied: req.operations.len(),
        diff,
    })
}

fn permute<T: Clone>(items: &[T], order: &[usize], what: &str) -> Result<Vec<T>, SandboxError> {
    if order.len() != items.len() || order.iter().collect::<BTreeSet<_>>().len() != items.len() {
        return Err(SandboxError::Scenario(format!(
            "{what}: order must be a permutation of 0..{}",
            items.len()
        )));
    }
    order
        .iter()
        .map(|&i| {
            items
                .get(i)
                .cloned()
                .ok_or_else(|| SandboxError::Scenario(format!("{what}: index {i} out of range")))
        })
        .collect()
}

fn apply_op(req: &mut PlayRequest, op: &AttackOp) -> Result<(), SandboxError> {
    match op {
        AttackOp::ReorderInputs { order } => {
            req.tx.inputs = permute(&req.tx.inputs, order, "reorderInputs")?;
        }
        AttackOp::ReorderDataInputs { order } => {
            req.tx.data_inputs = permute(&req.tx.data_inputs, order, "reorderDataInputs")?;
        }
        AttackOp::InsertDecoy {
            at_index,
            target_input,
        } => insert_decoy(req, *at_index, *target_input)?,
        AttackOp::SwapDataInput { index } => swap_data_input(req, *index)?,
        AttackOp::ShiftTokenIndices { input, by } => shift_token_indices(req, *input, *by)?,
        AttackOp::TamperField {
            target,
            index,
            field,
            value,
        } => tamper_field(req, target, *index, field, value)?,
    }
    Ok(())
}

/// The shape a decoy must have to satisfy a script's positional reads: which
/// `tokens(i)` slots and which registers `R4..R9` it accesses (with the type
/// each is read at), and whether it reads `.value`.
#[derive(Default, Debug)]
struct SlotShape {
    max_token_index: Option<usize>,
    /// Register number → the element type it is read at, when the lift
    /// recovered one (the typed `R{n}[T]` accessor or `getReg[T](i)`). `None`
    /// for the v5 `R{n}` property form, whose wire shape carries no type. A
    /// decoy must carry each register at its read type or the reducer errors
    /// on the mismatch before positional identity matters.
    registers: BTreeMap<u8, Option<String>>,
    reads_value: bool,
}

/// Walk the lifted tree of the target input's script and collect the accesses
/// it makes against `INPUTS(slot)`. Reuses the audit box-reference helpers so
/// the decoy is shaped by the same positional analysis the lint uses.
fn slot_shape(tree_hex: &str, slot: usize, testnet: bool) -> SlotShape {
    let mut shape = SlotShape::default();
    let bytes = match hex::decode(tree_hex.trim()) {
        Ok(b) => b,
        Err(_) => return shape,
    };
    let tree = match crate::inspect::parse_tree(&bytes) {
        Ok(t) => t,
        Err(_) => return shape,
    };
    let lifted = lift_tree(&tree, testnet);
    let mut vals = Vals::new();
    collect_vals(&lifted.node, &mut vals);
    let want = format!("INPUTS({slot})");
    walk_slot(&lifted.node, &vals, &want, &mut shape);
    shape
}

/// Does this node name `INPUTS(slot)` (directly or through a `val`)?
fn is_slot(n: &Node, vals: &Vals, want: &str) -> bool {
    positional_key(n, vals).as_deref() == Some(want)
        || box_collection(n, vals).is_none()
            && matches!(&deref(n, vals).kind, NodeKind::Val(_))
            && positional_key(deref(n, vals), vals).as_deref() == Some(want)
}

fn walk_slot(n: &Node, vals: &Vals, want: &str, shape: &mut SlotShape) {
    let d = deref(n, vals);
    // `<slot>.tokens(i)` — an indexed access whose receiver is the slot's tokens.
    if let Some((obj, idx)) = as_indexed(d) {
        if let NodeKind::Prop(inner, name) = &deref(obj, vals).kind {
            if name == "tokens" && is_slot(inner, vals, want) {
                if let Some(i) = literal_index(idx, vals) {
                    shape.max_token_index = Some(shape.max_token_index.map_or(i, |m| m.max(i)));
                }
            }
        }
    }
    match &d.kind {
        // `R4[Coll[Byte]]` — the typed accessor ExtractRegisterAs lifts to
        // (bare, or wrapped by `.get`/`.isDefined`). The element type is in
        // the name; the decoy must carry the register at that type.
        NodeKind::Prop(obj, name) => {
            if let Some((r, tpe)) = typed_register(name) {
                if is_slot(obj, vals, want) {
                    note_register(shape, r, Some(tpe));
                }
            } else if name == "value" && is_slot(obj, vals, want) {
                shape.reads_value = true;
            }
        }
        // `R4` — the v5 property form; the wire carries no element type, so
        // the placeholder falls back to the Long default.
        NodeKind::Method(obj, name, _) => {
            if let Some(r) = name.strip_prefix('R').and_then(|s| s.parse::<u8>().ok()) {
                if is_slot(obj, vals, want) {
                    note_register(shape, r, None);
                }
            }
        }
        // `getReg[Long](i)` — the v6 dynamic form carries the element type on
        // the wire and the register index as its argument.
        NodeKind::GetRegDyn(obj, tpe, args) => {
            if let Some(idx) = args.first().and_then(|a| literal_index(a, vals)) {
                if let Ok(r) = u8::try_from(idx) {
                    if is_slot(obj, vals, want) {
                        note_register(shape, r, Some(tpe.clone()));
                    }
                }
            }
        }
        _ => {}
    }
    for c in children(d) {
        walk_slot(c, vals, want, shape);
    }
}

/// `R4[Coll[Byte]]` → (4, `Coll[Byte]`). The typed register accessor the
/// compiler's ExtractRegisterAs lifts to.
fn typed_register(name: &str) -> Option<(u8, String)> {
    let rest = name.strip_prefix('R')?;
    let (n, tpe) = rest.split_once('[')?;
    let n = n.parse::<u8>().ok()?;
    let tpe = tpe.strip_suffix(']')?;
    Some((n, tpe.to_string()))
}

/// Record a register access, keeping the first recovered type for the slot.
fn note_register(shape: &mut SlotShape, r: u8, tpe: Option<String>) {
    if !(4..=9).contains(&r) {
        return;
    }
    match shape.registers.get_mut(&r) {
        Some(slot) => {
            if slot.is_none() {
                *slot = tpe;
            }
        }
        None => {
            shape.registers.insert(r, tpe);
        }
    }
}

fn literal_index(n: &Node, vals: &Vals) -> Option<usize> {
    match &deref(n, vals).kind {
        NodeKind::Int(i) if *i >= 0 => Some(*i as usize),
        NodeKind::Num(s) => s.trim_end_matches(['L', 'y']).parse::<usize>().ok(),
        _ => None,
    }
}

/// A deterministic 32-byte junk token id, distinct per `(seed, n)`. Never a
/// real NFT: derived from a fixed domain string so it cannot collide with a
/// contract's expected token.
fn junk_token_id(seed: &str, n: usize) -> String {
    let material = format!("ergo-forge/decoy-token/{seed}/{n}");
    hex::encode(ergo_primitives::digest::blake2b256(material.as_bytes()).as_bytes())
}

/// A type-plausible register value. The decoy's job is to let positional reads
/// resolve without an `Option.get` throwing or a type mismatch erroring; the
/// concrete value is a benign placeholder, not an assertion about the
/// contract. Reads whose type the lift could not recover (the v5 `R{n}`
/// property form) fall back to `Long`, as do type names the value parser does
/// not know.
fn placeholder_register(tpe: Option<&str>) -> TypedValue {
    let (tpe, value) = match tpe {
        None | Some("Long") => ("Long", serde_json::json!(0)),
        Some("Int") => ("Int", serde_json::json!(0)),
        Some("Byte") => ("Byte", serde_json::json!(0)),
        Some("Short") => ("Short", serde_json::json!(0)),
        Some("Boolean") => ("Boolean", serde_json::json!(false)),
        Some("BigInt") => ("BigInt", serde_json::json!("0")),
        Some("SigmaProp") => ("SigmaProp", serde_json::json!(true)),
        // The curve generator, mirroring compose's `sample`: a decoy's point
        // only needs to parse.
        Some("GroupElement") => (
            "GroupElement",
            serde_json::json!("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"),
        ),
        // A deterministic all-zero digest; the decoy's tree only needs to build.
        Some("AvlTree") => (
            "AvlTree",
            serde_json::json!({"digest": AVL_DIGEST, "keyLength": 32}),
        ),
        // The empty collection is valid at every element type.
        Some("Coll[Byte]") => ("Coll[Byte]", serde_json::json!("")),
        Some(other) if other.starts_with("Coll[") => (other, serde_json::json!([])),
        Some(_) => ("Long", serde_json::json!(0)),
    };
    TypedValue {
        r#type: tpe.to_string(),
        value,
    }
}

/// The digest of the placeholder `AvlTree` register value: 32 zero bytes.
const AVL_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn insert_decoy(
    req: &mut PlayRequest,
    at_index: usize,
    target_input: usize,
) -> Result<(), SandboxError> {
    if at_index > req.tx.inputs.len() {
        return Err(SandboxError::Scenario(format!(
            "insertDecoy: at_index {at_index} past the input list"
        )));
    }
    let target = req.tx.inputs.get(target_input).ok_or_else(|| {
        SandboxError::Scenario(format!(
            "insertDecoy: target_input {target_input} out of range"
        ))
    })?;
    let tree_hex = box_tree(req, &target.box_id)?;
    let testnet = req.network.as_deref() == Some("testnet");
    let shape = slot_shape(&tree_hex, at_index, testnet);

    let decoy = build_decoy(&shape, at_index);
    let decoy_id = decoy.box_id.clone().expect("decoy has an id");
    // The decoy adds input value, and play::apply requires ERG to balance
    // exactly: return it in a new output so the experiment's only delta is the
    // mutation itself — otherwise `ERG not conserved` sets ok=false and
    // shadows every reader verdict. (A real spender funds a decoy from change
    // the same way; the decoy's junk tokens may simply burn.)
    req.tx.outputs.push(ScenarioBox {
        value: decoy.value,
        ergo_tree: decoy.ergo_tree.clone(),
        tokens: Vec::new(),
        creation_height: 0,
        registers: BTreeMap::new(),
        box_id: None,
        extension: BTreeMap::new(),
    });
    req.boxes.push(decoy);
    req.tx.inputs.insert(
        at_index,
        PlayInput {
            box_id: decoy_id,
            context_vars: BTreeMap::new(),
            secrets: Vec::new(),
            parties: Vec::new(),
        },
    );
    Ok(())
}

/// Token slots a decoy always carries, even when the lift did not recognise a
/// `tokens(i)` access — enough to cover the small indices scripts read.
const DEFAULT_TOKEN_SLOTS: usize = 4;
/// The highest register a decoy always populates, so an unrecognised `R{n}`
/// read up to R6 resolves.
const DEFAULT_MAX_REGISTER: u8 = 6;

fn build_decoy(shape: &SlotShape, at_index: usize) -> ScenarioBox {
    let seed = format!("input-{at_index}");
    // Over-provision: satisfy the slots the walk found, and a small default set
    // for accesses the decompiler could not recover. A superset of tokens and
    // registers is exactly what a decoy would carry; it binds no NFT.
    let slots = shape
        .max_token_index
        .map_or(DEFAULT_TOKEN_SLOTS, |m| (m + 1).max(DEFAULT_TOKEN_SLOTS));
    let tokens: Vec<TokenAmount> = (0..slots)
        .map(|i| TokenAmount {
            id: junk_token_id(&seed, i),
            amount: 1,
        })
        .collect();
    // Every register from R4 through the highest one the shape requires (at
    // least R6): the EvalBox builder expects registers dense from R4, so a
    // gap — R8 present, R7 missing — would fail the build before evaluation.
    let max_reg = shape
        .registers
        .keys()
        .copied()
        .next_back()
        .unwrap_or(DEFAULT_MAX_REGISTER)
        .max(DEFAULT_MAX_REGISTER);
    let registers: BTreeMap<String, TypedValue> = (4..=max_reg)
        .map(|r| {
            let tpe = shape.registers.get(&r).cloned().flatten();
            (format!("R{r}"), placeholder_register(tpe.as_deref()))
        })
        .collect();
    // A trivially satisfiable script so the decoy is spendable in the draft;
    // the point of the experiment is the *other* inputs' scripts.
    let anyone = "10010101d17300".to_string();
    ScenarioBox {
        value: 1_000_000,
        ergo_tree: Some(anyone),
        tokens,
        creation_height: 0,
        registers,
        box_id: Some(junk_token_id(&format!("decoy-box/{seed}"), 0)),
        extension: BTreeMap::new(),
    }
}

fn swap_data_input(req: &mut PlayRequest, index: usize) -> Result<(), SandboxError> {
    let id = req.tx.data_inputs.get(index).cloned().ok_or_else(|| {
        SandboxError::Scenario(format!("swapDataInput: index {index} out of range"))
    })?;
    let original = find_box(req, &id)?.clone();
    let lookalike_id = junk_token_id(&format!("lookalike/{id}"), 0);
    let lookalike = ScenarioBox {
        value: original.value,
        ergo_tree: original.ergo_tree.clone(),
        tokens: Vec::new(), // drop any NFT the original carried
        creation_height: original.creation_height,
        registers: original.registers.clone(),
        box_id: Some(lookalike_id.clone()),
        extension: BTreeMap::new(),
    };
    req.boxes.push(lookalike);
    req.tx.data_inputs[index] = lookalike_id;
    Ok(())
}

/// The widest token shift one experiment may request. Real positional
/// confusions happen in the first few slots, and a box serialises at most 255
/// tokens anyway; the cap keeps a hostile `by` from allocating this route into
/// an out-of-memory kill.
const MAX_TOKEN_SHIFT: usize = 255;

fn shift_token_indices(req: &mut PlayRequest, input: usize, by: usize) -> Result<(), SandboxError> {
    if by > MAX_TOKEN_SHIFT {
        return Err(SandboxError::Scenario(format!(
            "shiftTokenIndices: by {by} exceeds the experiment limit of {MAX_TOKEN_SHIFT}"
        )));
    }
    let id = req
        .tx
        .inputs
        .get(input)
        .map(|i| i.box_id.clone())
        .ok_or_else(|| {
            SandboxError::Scenario(format!("shiftTokenIndices: input {input} out of range"))
        })?;
    let b = find_box_mut(req, &id)?;
    let seed = format!("shift/{id}");
    let mut prepend: Vec<TokenAmount> = (0..by)
        .map(|n| TokenAmount {
            id: junk_token_id(&seed, n),
            amount: 1,
        })
        .collect();
    prepend.append(&mut b.tokens);
    b.tokens = prepend;
    Ok(())
}

fn tamper_field(
    req: &mut PlayRequest,
    target: &str,
    index: usize,
    field: &str,
    value: &serde_json::Value,
) -> Result<(), SandboxError> {
    let b = match target {
        "input" => {
            let id = req
                .tx
                .inputs
                .get(index)
                .map(|i| i.box_id.clone())
                .ok_or_else(|| {
                    SandboxError::Scenario(format!("tamperField: input {index} out of range"))
                })?;
            find_box_mut(req, &id)?
        }
        "output" => req.tx.outputs.get_mut(index).ok_or_else(|| {
            SandboxError::Scenario(format!("tamperField: output {index} out of range"))
        })?,
        other => {
            return Err(SandboxError::Scenario(format!(
                "tamperField: target must be `input` or `output`, got `{other}`"
            )))
        }
    };
    if field == "value" {
        b.value = value.as_i64().ok_or_else(|| {
            SandboxError::Scenario("tamperField value: expected an integer".into())
        })?;
    } else if let Some(i) = field
        .strip_prefix("tokenAmount:")
        .and_then(|s| s.parse::<usize>().ok())
    {
        let amt = value.as_u64().ok_or_else(|| {
            SandboxError::Scenario("tamperField tokenAmount: expected u64".into())
        })?;
        b.tokens
            .get_mut(i)
            .ok_or_else(|| SandboxError::Scenario(format!("tamperField: token {i} out of range")))?
            .amount = amt;
    } else if field.starts_with('R') {
        let tv: TypedValue = serde_json::from_value(value.clone()).map_err(|e| {
            SandboxError::Scenario(format!("tamperField {field}: expected a typed value: {e}"))
        })?;
        b.registers.insert(field.to_string(), tv);
    } else {
        return Err(SandboxError::Scenario(format!(
            "tamperField: unknown field `{field}` (value, tokenAmount:<i>, R<n>)"
        )));
    }
    Ok(())
}

fn box_tree(req: &PlayRequest, box_id: &str) -> Result<String, SandboxError> {
    Ok(find_box(req, box_id)?.ergo_tree.clone().unwrap_or_default())
}

fn find_box<'a>(req: &'a PlayRequest, id: &str) -> Result<&'a ScenarioBox, SandboxError> {
    req.boxes
        .iter()
        .find(|b| {
            b.box_id
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(id))
                .unwrap_or(false)
        })
        .ok_or_else(|| SandboxError::Scenario(format!("box {id}: no such box")))
}

fn find_box_mut<'a>(
    req: &'a mut PlayRequest,
    id: &str,
) -> Result<&'a mut ScenarioBox, SandboxError> {
    req.boxes
        .iter_mut()
        .find(|b| {
            b.box_id
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(id))
                .unwrap_or(false)
        })
        .ok_or_else(|| SandboxError::Scenario(format!("box {id}: no such box")))
}

// Keep the unused-import guard honest across refactors.
#[allow(dead_code)]
fn _tx_type_marker(_: &PlayTx) {}
