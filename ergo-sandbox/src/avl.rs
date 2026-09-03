//! AVL+ trees for scenarios: a real prover (`ergo_avltree_rust`, the crate
//! whose verifier the node's evaluator uses) builds the tree a box's
//! register claims, performs the spender's operations, and yields the
//! digest before, the digest after, and the proof — so a script that
//! calls `tree.insert(...)`, `tree.get(...)` or `tree.remove(...)` can be
//! exercised against authentic inputs instead of hand-made bytes.
//!
//! In a scenario, `"avl": {"name": {...}}` declares trees, and typed
//! values refer to them: `{"type": "AvlTree", "value": "@avl.name"}` (the
//! tree before the operations; `"@avl.name.after"` after them) and
//! `{"type": "Coll[Byte]", "value": "@avl.name.proof"}` (also
//! `.digest` / `.digestAfter` as bytes).

use std::collections::BTreeMap;

use bytes::Bytes;
use ergo_avltree_rust::authenticated_tree_ops::AuthenticatedTreeOps;
use ergo_avltree_rust::batch_avl_prover::BatchAVLProver;
use ergo_avltree_rust::batch_node::{AVLTree, Node, NodeHeader};
use ergo_avltree_rust::operation::{KeyValue, Operation};
use serde::{Deserialize, Serialize};

use crate::scenario::{Scenario, ScenarioBox, TypedValue};
use crate::SandboxError;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvlSpec {
    /// Key length in bytes (32 is usual).
    #[serde(default = "thirty_two")]
    pub key_length: usize,
    /// Fixed value length, or variable when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_length: Option<usize>,
    /// The tree's contents before the operations: `[keyHex, valueHex]`.
    #[serde(default)]
    pub entries: Vec<(String, String)>,
    /// What the spender does to it, in order; the proof covers all of them.
    #[serde(default)]
    pub operations: Vec<AvlOp>,
    #[serde(default = "yes")]
    pub insert_allowed: bool,
    #[serde(default = "yes")]
    pub update_allowed: bool,
    #[serde(default = "yes")]
    pub remove_allowed: bool,
}

fn thirty_two() -> usize {
    32
}
fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AvlOp {
    Insert { key: String, value: String },
    Update { key: String, value: String },
    InsertOrUpdate { key: String, value: String },
    Remove { key: String },
    Lookup { key: String },
}

/// What the prover produced for one declared tree.
#[derive(Debug, Clone)]
pub struct AvlBuilt {
    pub digest_before: Vec<u8>,
    pub digest_after: Vec<u8>,
    pub proof: Vec<u8>,
    pub spec: AvlSpec,
}

fn err(msg: impl Into<String>) -> SandboxError {
    SandboxError::Scenario(msg.into())
}

fn hex_bytes(what: &str, s: &str) -> Result<Bytes, SandboxError> {
    hex::decode(s.trim())
        .map(Bytes::from)
        .map_err(|e| err(format!("avl {what} hex: {e}")))
}

fn resolver(d: &[u8; 32]) -> Node {
    Node::LabelOnly(NodeHeader::new(Some(*d), None))
}

fn operation(spec: &AvlSpec, op: &AvlOp) -> Result<Operation, SandboxError> {
    let key = |k: &str| -> Result<Bytes, SandboxError> {
        let b = hex_bytes("key", k)?;
        if b.len() != spec.key_length {
            return Err(err(format!(
                "avl key is {} bytes, the tree's keys are {}",
                b.len(),
                spec.key_length
            )));
        }
        Ok(b)
    };
    let kv = |k: &str, v: &str| -> Result<KeyValue, SandboxError> {
        let value = hex_bytes("value", v)?;
        if let Some(n) = spec.value_length {
            if value.len() != n {
                return Err(err(format!(
                    "avl value is {} bytes, the tree's values are {n}",
                    value.len()
                )));
            }
        }
        Ok(KeyValue {
            key: key(k)?,
            value,
        })
    };
    Ok(match op {
        AvlOp::Insert { key: k, value } => Operation::Insert(kv(k, value)?),
        AvlOp::Update { key: k, value } => Operation::Update(kv(k, value)?),
        AvlOp::InsertOrUpdate { key: k, value } => Operation::InsertOrUpdate(kv(k, value)?),
        AvlOp::Remove { key: k } => Operation::Remove(key(k)?),
        AvlOp::Lookup { key: k } => Operation::Lookup(key(k)?),
    })
}

/// Build the tree, apply the operations, keep the artefacts.
pub fn build(spec: &AvlSpec) -> Result<AvlBuilt, SandboxError> {
    if spec.key_length == 0 {
        return Err(err("avl keyLength must be positive"));
    }
    let tree = AVLTree::new(resolver, spec.key_length, spec.value_length);
    let mut prover = BatchAVLProver::new(tree, false);
    for (k, v) in &spec.entries {
        let op = operation(
            spec,
            &AvlOp::Insert {
                key: k.clone(),
                value: v.clone(),
            },
        )?;
        prover
            .perform_one_operation(&op)
            .map_err(|e| err(format!("avl entry {k}: {e:?}")))?;
    }
    // Seal the initial contents into the tree (the proof of building it is
    // nobody's business).
    let _ = prover.generate_proof();
    let digest_before = prover
        .digest()
        .ok_or_else(|| err("avl: no digest"))?
        .to_vec();
    for op in &spec.operations {
        let o = operation(spec, op)?;
        prover
            .perform_one_operation(&o)
            .map_err(|e| err(format!("avl operation {op:?}: {e:?}")))?;
    }
    let proof = prover.generate_proof().to_vec();
    let digest_after = prover
        .digest()
        .ok_or_else(|| err("avl: no digest"))?
        .to_vec();
    Ok(AvlBuilt {
        digest_before,
        digest_after,
        proof,
        spec: spec.clone(),
    })
}

fn tree_json(b: &AvlBuilt, after: bool) -> serde_json::Value {
    serde_json::json!({
        "digest": hex::encode(if after { &b.digest_after } else { &b.digest_before }),
        "keyLength": b.spec.key_length,
        "valueLength": b.spec.value_length,
        "insertAllowed": b.spec.insert_allowed,
        "updateAllowed": b.spec.update_allowed,
        "removeAllowed": b.spec.remove_allowed,
    })
}

fn substitute(tv: &mut TypedValue, built: &BTreeMap<String, AvlBuilt>) -> Result<(), SandboxError> {
    let Some(s) = tv.value.as_str() else {
        return Ok(());
    };
    let Some(rest) = s.strip_prefix("@avl.") else {
        return Ok(());
    };
    let (name, field) = match rest.split_once('.') {
        Some((n, f)) => (n, f),
        None => (rest, ""),
    };
    let b = built
        .get(name)
        .ok_or_else(|| err(format!("`{s}`: no avl tree named `{name}` in the scenario")))?;
    tv.value = match (tv.r#type.as_str(), field) {
        ("AvlTree", "" | "before") => tree_json(b, false),
        ("AvlTree", "after") => tree_json(b, true),
        ("Coll[Byte]", "proof") => serde_json::json!(hex::encode(&b.proof)),
        ("Coll[Byte]", "digest") => serde_json::json!(hex::encode(&b.digest_before)),
        ("Coll[Byte]", "digestAfter") => serde_json::json!(hex::encode(&b.digest_after)),
        (t, f) => {
            return Err(err(format!(
                "`{s}`: `{f}` is not something an avl tree gives a {t} (AvlTree: `@avl.name`, `.after`; Coll[Byte]: `.proof`, `.digest`, `.digestAfter`)"
            )))
        }
    };
    Ok(())
}

fn substitute_box(
    b: &mut ScenarioBox,
    built: &BTreeMap<String, AvlBuilt>,
) -> Result<(), SandboxError> {
    for tv in b.registers.values_mut() {
        substitute(tv, built)?;
    }
    Ok(())
}

/// The scenario with every `@avl.` reference replaced by what the prover
/// produced. A scenario without trees comes back unchanged.
pub fn resolved(sc: &Scenario) -> Result<Scenario, SandboxError> {
    if sc.avl.is_empty() {
        return Ok(sc.clone());
    }
    let mut built = BTreeMap::new();
    for (name, spec) in &sc.avl {
        built.insert(name.clone(), build(spec)?);
    }
    let mut out = sc.clone();
    if let Some(b) = out.self_box.as_mut() {
        substitute_box(b, &built)?;
    }
    for list in [&mut out.inputs, &mut out.outputs, &mut out.data_inputs] {
        for b in list.iter_mut() {
            substitute_box(b, &built)?;
        }
    }
    for tv in out.context_vars.values_mut() {
        substitute(tv, &built)?;
    }
    Ok(out)
}
