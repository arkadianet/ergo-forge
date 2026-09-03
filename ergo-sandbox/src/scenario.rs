//! The JSON scenario model: user-facing describing of a spending context,
//! plus the typed-value parser that lifts user JSON into
//! `SigmaType`/`SigmaValue` pairs, `EvalBox`es, and context variables.
//!
//! This is presentation-layer sugar over the evaluator's owned types. It
//! accepts the value forms a playground user can produce from JSON and
//! rejects everything else explicitly — never guessing.

use std::collections::BTreeMap;

use ergo_ser::address::NetworkPrefix;
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

use crate::SandboxError;

// ── Scenario model ───────────────────────────────────────────────────────────

/// A spending scenario: the tree under evaluation plus everything the
/// evaluator may observe. Deserializable from the workbench JSON.
///
/// Field names are camelCase (`creationHeight`, `contextVars`, …) to match
/// the tooling-API request shapes (`ergoscript-tooling-api.md` §4.3).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    /// ErgoTree wire bytes, hex. Mutually exclusive with `source`.
    #[serde(default)]
    pub tree: Option<String>,
    /// ErgoScript source, compiled with `treeVersion`. Mutually exclusive
    /// with `tree`.
    #[serde(default)]
    pub source: Option<String>,
    /// Tree version for `source` compilation (default 0).
    #[serde(default)]
    pub tree_version: u8,
    /// Network for `source` compilation address encodings: `mainnet` or
    /// `testnet` (default `mainnet`).
    #[serde(default)]
    pub network: Option<String>,
    /// Spending height (CONTEXT.HEIGHT).
    pub height: u32,
    /// The box being spent. Defaults to a synthetic box whose tree is the
    /// evaluated tree and whose value is 0.
    #[serde(default)]
    pub self_box: Option<ScenarioBox>,
    /// Alternative to `selfBox`: SELF is `inputs[selfIndex]`, and `inputs`
    /// is the whole input list in transaction order — how a real
    /// transaction is validated (`CONTEXT.INPUTS(selfIndex) == SELF`). That
    /// input may omit `ergoTree` (it is the tree under evaluation).
    #[serde(default)]
    pub self_index: Option<usize>,
    /// Transaction inputs (CONTEXT.INPUTS). Index 0 is conventionally SELF;
    /// the evaluator sees whatever is provided.
    #[serde(default)]
    pub inputs: Vec<ScenarioBox>,
    /// Transaction outputs (CONTEXT.OUTPUTS).
    #[serde(default)]
    pub outputs: Vec<ScenarioBox>,
    /// Read-only data inputs.
    #[serde(default)]
    pub data_inputs: Vec<ScenarioBox>,
    /// Context variables (the spending proof's extension), keyed by var id.
    /// JSON keys are strings ("0"); accepted as such even where serde's
    /// `flatten` (test-suite cases) cannot convert integer keys itself.
    #[serde(default, deserialize_with = "de_var_map")]
    pub context_vars: BTreeMap<u8, TypedValue>,
    /// Miner public key, 33-byte hex (0xAC). Defaults to all-zero.
    #[serde(default)]
    pub miner_pubkey: Option<String>,
    /// Pre-header fields. Defaults to all-zero.
    #[serde(default)]
    pub pre_header: Option<PreHeader>,
    /// Block-cost budget for the evaluation (default
    /// [`crate::eval::DEFAULT_COST_LIMIT`]).
    #[serde(default)]
    pub cost_limit: Option<u64>,
    /// Activated script version (`blockHeaderVersion - 1`; default 3).
    #[serde(default)]
    pub activated_script_version: Option<u8>,
    /// Optional spending proof, hex. When present the outcome additionally
    /// reports proof verification against `message`.
    #[serde(default)]
    pub proof: Option<String>,
    /// Secrets the spender holds. When the script reduces to a sigma
    /// proposition and no `proof` is given, the sandbox PRODUCES a proof
    /// with the node's wallet prover and verifies it like a supplied one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<crate::prove::SecretSpec>,
    /// AVL+ trees built by a real prover; see [`crate::avl`]. Typed values
    /// refer to them as `"@avl.name"`, `"@avl.name.after"`,
    /// `"@avl.name.proof"`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub avl: BTreeMap<String, crate::avl::AvlSpec>,
    /// Message the proof commits to (bytes-to-sign), hex. Default empty —
    /// real spends sign the transaction bytes.
    #[serde(default)]
    pub message: Option<String>,
}

/// Deserialize `{"0": …, "1": …}` (or integer keys) into a `u8`-keyed map.
fn de_var_map<'de, D>(d: D) -> Result<BTreeMap<u8, TypedValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let raw: BTreeMap<String, TypedValue> = BTreeMap::deserialize(d)?;
    raw.into_iter()
        .map(|(k, v)| {
            k.trim()
                .parse::<u8>()
                .map(|id| (id, v))
                .map_err(|_| D::Error::custom(format!("context var id `{k}` is not 0..=255")))
        })
        .collect()
}

/// Pre-header fields (SPreHeader). All optional; defaults are zero.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreHeader {
    /// Block version byte.
    #[serde(default)]
    pub version: Option<u8>,
    /// Parent header id, 32-byte hex.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Timestamp, milliseconds.
    #[serde(default)]
    pub timestamp: Option<u64>,
    /// Encoded difficulty (nBits).
    #[serde(default)]
    pub n_bits: Option<u64>,
    /// Miner votes, exactly 3 bytes.
    #[serde(default)]
    pub votes: Option<[u8; 3]>,
}

/// A box as the scenario describes it. Registers are the R4–R9 block;
/// the mandatory R0–R3 (value, ergoTree, tokens, creationInfo) are
/// populated from their dedicated fields.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioBox {
    /// Box value in nanoErg (R0).
    #[serde(default)]
    pub value: i64,
    /// ErgoTree wire bytes, hex (R1). Optional: omitted (or empty) on the
    /// self box means "the tree under evaluation".
    #[serde(default)]
    pub ergo_tree: Option<String>,
    /// Token ids and amounts (R2).
    #[serde(default)]
    pub tokens: Vec<TokenAmount>,
    /// Creation height (R3).
    #[serde(default)]
    pub creation_height: u32,
    /// Additional registers R4–R9. Keys are `R4`…`R9`; presence must be
    /// dense from R4 upward (an `EvalBox` invariant).
    #[serde(default)]
    pub registers: BTreeMap<String, TypedValue>,
    /// Box id, 32-byte hex. Defaults to all-zero (synthetic box).
    #[serde(default)]
    pub box_id: Option<String>,
}

/// A (token id, amount) pair.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmount {
    /// 32-byte token id, hex.
    pub id: String,
    /// Token amount.
    pub amount: u64,
}

/// A user-supplied typed value: a type name plus a JSON value of the
/// matching shape.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TypedValue {
    /// Type name, e.g. `Int`, `Long`, `BigInt`, `SigmaProp`,
    /// `GroupElement`, `Coll[Byte]`, `Coll[Int]`, `Coll[Coll[Byte]]`.
    #[serde(rename = "type")]
    pub r#type: String,
    /// The value in its JSON form.
    pub value: serde_json::Value,
}

// ── Value parsing ────────────────────────────────────────────────────────────

/// Parse a user type name into a `SigmaType`. Supports the base types and
/// arbitrarily nested collections (`Coll[T]` for any `T`, including
/// another `Coll[…]`).
fn parse_type_name(tpe: &str) -> Result<SigmaType, SandboxError> {
    if let Some(elem) = tpe.strip_prefix("Coll[").and_then(|s| s.strip_suffix(']')) {
        return Ok(SigmaType::SColl(Box::new(parse_type_name(elem)?)));
    }
    parse_base_type(tpe)
}

fn parse_base_type(tpe: &str) -> Result<SigmaType, SandboxError> {
    Ok(match tpe {
        "Boolean" => SigmaType::SBoolean,
        "Byte" => SigmaType::SByte,
        "Short" => SigmaType::SShort,
        "Int" => SigmaType::SInt,
        "Long" => SigmaType::SLong,
        "BigInt" => SigmaType::SBigInt,
        "GroupElement" => SigmaType::SGroupElement,
        "SigmaProp" => SigmaType::SSigmaProp,
        other => {
            return Err(SandboxError::Scenario(format!(
                "unsupported type name `{other}` (supported: Boolean, Byte, Short, Int, Long, \
                 BigInt, GroupElement, SigmaProp, Coll[T], raw)"
            )))
        }
    })
}

/// `{"type": "raw", "value": "<hex>"}`: a serialized constant exactly as a
/// node or explorer reports a register (`serializedValue`), type descriptor
/// first. This is how a real box's registers reach a scenario untouched.
fn parse_raw_constant(value: &serde_json::Value) -> Result<(SigmaType, SigmaValue), SandboxError> {
    let hex_str = value
        .as_str()
        .ok_or_else(|| SandboxError::Scenario("raw value must be a hex string".into()))?;
    let bytes = hex::decode(hex_str.trim()).map_err(|source| SandboxError::Hex {
        field: "raw",
        source,
    })?;
    let mut r = ergo_primitives::reader::VlqReader::new(&bytes);
    let pair = ergo_ser::sigma_value::read_constant(&mut r)
        .map_err(|e| SandboxError::Scenario(format!("raw constant does not parse: {e:?}")))?;
    if !r.is_empty() {
        return Err(SandboxError::Scenario(
            "raw constant has trailing bytes".into(),
        ));
    }
    Ok(pair)
}

/// Parse a user (type name, JSON value) pair into a typed sigma constant.
///
/// Supported types: `Boolean`, `Byte`, `Short`, `Int`, `Long`, `BigInt`
/// (decimal string), `GroupElement` (33-byte hex), `SigmaProp` (`true`,
/// `false`, or 33-byte pubkey hex → `ProveDlog`), and `Coll[T]` for any
/// `T` — including nested `Coll[Coll[T]]` (`Coll[Byte]` from a hex
/// string; everything else from JSON arrays).
pub fn parse_typed_value(
    tpe: &str,
    value: &serde_json::Value,
) -> Result<(SigmaType, SigmaValue), SandboxError> {
    if let Some(elem) = tpe.strip_prefix("Coll[").and_then(|s| s.strip_suffix(']')) {
        return parse_coll(elem, value);
    }
    if tpe == "raw" {
        return parse_raw_constant(value);
    }
    match tpe {
        "Boolean" => Ok((
            SigmaType::SBoolean,
            SigmaValue::Boolean(
                value
                    .as_bool()
                    .ok_or_else(|| bad(tpe, value, "a JSON boolean"))?,
            ),
        )),
        "Byte" => {
            let n = value_i64(tpe, value)?;
            i8::try_from(n).map_err(|_| range(tpe, n, i8::MIN as i64, i8::MAX as i64))?;
            Ok((SigmaType::SByte, SigmaValue::Byte(n as i8)))
        }
        "Short" => {
            let n = value_i64(tpe, value)?;
            i16::try_from(n).map_err(|_| range(tpe, n, i16::MIN as i64, i16::MAX as i64))?;
            Ok((SigmaType::SShort, SigmaValue::Short(n as i16)))
        }
        "Int" => {
            let n = value_i64(tpe, value)?;
            i32::try_from(n).map_err(|_| range(tpe, n, i32::MIN as i64, i32::MAX as i64))?;
            Ok((SigmaType::SInt, SigmaValue::Int(n as i32)))
        }
        "Long" => Ok((SigmaType::SLong, SigmaValue::Long(value_i64(tpe, value)?))),
        "BigInt" => {
            let s = value
                .as_str()
                .ok_or_else(|| bad(tpe, value, "a decimal string"))?;
            let n = s
                .parse::<BigInt>()
                .map_err(|_| bad(tpe, value, "a decimal string"))?;
            Ok((SigmaType::SBigInt, SigmaValue::BigInt(n)))
        }
        "GroupElement" => {
            let bytes = user_hex(tpe, value, 33)?;
            let mut ge = [0u8; 33];
            ge.copy_from_slice(&bytes);
            Ok((
                SigmaType::SGroupElement,
                SigmaValue::GroupElement(ergo_primitives::group_element::GroupElement::from_bytes(
                    ge,
                )),
            ))
        }
        "SigmaProp" => parse_sigma_prop(value),
        "AvlTree" => parse_avl_tree(value),
        _ => Err(SandboxError::Scenario(format!(
            "unsupported type name `{tpe}` (supported: Boolean, Byte, Short, Int, Long, BigInt, \
             GroupElement, SigmaProp, AvlTree, Coll[T])"
        ))),
    }
}

/// `{"digest": hex (33 bytes), "keyLength": n, "valueLength": n?,
/// "insertAllowed": bool, "updateAllowed": bool, "removeAllowed": bool}`;
/// the flags default to true. `"@avl.name"` strings are resolved before
/// this runs (see [`crate::avl`]).
fn parse_avl_tree(value: &serde_json::Value) -> Result<(SigmaType, SigmaValue), SandboxError> {
    let obj = value.as_object().ok_or_else(|| {
        bad("AvlTree", value, "an object {digest, keyLength, valueLength?, insertAllowed?, updateAllowed?, removeAllowed?} or \"@avl.name\"")
    })?;
    let digest_hex = obj
        .get("digest")
        .and_then(|d| d.as_str())
        .ok_or_else(|| bad("AvlTree.digest", value, "hex"))?;
    let digest = hex::decode(digest_hex.trim())
        .map_err(|e| SandboxError::Scenario(format!("AvlTree digest hex: {e}")))?;
    let key_length = obj.get("keyLength").and_then(|k| k.as_i64()).unwrap_or(32);
    let value_length_opt = obj
        .get("valueLength")
        .and_then(|k| k.as_i64())
        .map(|n| n as i32);
    let flag = |k: &str| obj.get(k).and_then(|b| b.as_bool()).unwrap_or(true);
    Ok((
        SigmaType::SAvlTree,
        SigmaValue::AvlTree(ergo_ser::sigma_value::AvlTreeData {
            digest,
            insert_allowed: flag("insertAllowed"),
            update_allowed: flag("updateAllowed"),
            remove_allowed: flag("removeAllowed"),
            key_length: key_length as i32,
            value_length_opt,
        }),
    ))
}

fn parse_coll(
    elem: &str,
    value: &serde_json::Value,
) -> Result<(SigmaType, SigmaValue), SandboxError> {
    // Coll[Byte] accepts a hex string OR a JSON array of integers.
    if elem == "Byte" {
        if let Some(s) = value.as_str() {
            let bytes = hex::decode(s.trim())
                .map_err(|e| SandboxError::Scenario(format!("Coll[Byte] hex: {e}")))?;
            return Ok((
                SigmaType::SColl(Box::new(SigmaType::SByte)),
                SigmaValue::Coll(CollValue::Bytes(bytes)),
            ));
        }
    }
    let items = value.as_array().ok_or_else(|| {
        bad(
            &format!("Coll[{elem}]"),
            value,
            "a JSON array (or hex, for Coll[Byte])",
        )
    })?;
    if elem == "Boolean" {
        let mut bits = Vec::with_capacity(items.len());
        for it in items {
            bits.push(
                it.as_bool()
                    .ok_or_else(|| bad(elem, it, "a JSON boolean"))?,
            );
        }
        return Ok((
            SigmaType::SColl(Box::new(SigmaType::SBoolean)),
            SigmaValue::Coll(CollValue::BoolBits(bits)),
        ));
    }
    let mut vals = Vec::with_capacity(items.len());
    for it in items {
        // parse_typed_value(elem, …) already guarantees the element's type.
        let (_, v) = parse_typed_value(elem, it)?;
        vals.push(v);
    }
    Ok((
        SigmaType::SColl(Box::new(parse_type_name(elem)?)),
        SigmaValue::Coll(CollValue::Values(vals)),
    ))
}

fn parse_sigma_prop(value: &serde_json::Value) -> Result<(SigmaType, SigmaValue), SandboxError> {
    if let Some(b) = value.as_bool() {
        return Ok((
            SigmaType::SSigmaProp,
            SigmaValue::SigmaProp(SigmaBoolean::TrivialProp(b)),
        ));
    }
    // A 33-byte pubkey hex, or a P2PK address (mainnet or testnet), whose
    // tree is `0008cd` + the key. A script address is not a key: refused.
    let bytes = match value.as_str().map(str::trim) {
        Some(s) if s.len() != 66 || hex::decode(s).is_err() => {
            let tree = ergo_ser::address::decode_address_to_tree_bytes(s, NetworkPrefix::Mainnet)
                .or_else(|_| {
                    ergo_ser::address::decode_address_to_tree_bytes(s, NetworkPrefix::Testnet)
                })
                .map_err(|e| {
                    SandboxError::Scenario(format!(
                        "`SigmaProp` needs a 33-byte pubkey hex or a P2PK address: {e:?}"
                    ))
                })?;
            match tree.as_slice() {
                [0x00, 0x08, 0xcd, pk @ ..] if pk.len() == 33 => pk.to_vec(),
                _ => {
                    return Err(SandboxError::Scenario(
                        "`SigmaProp` address is a script (P2S/P2SH) address, not a key".into(),
                    ))
                }
            }
        }
        _ => user_hex("SigmaProp", value, 33)?,
    };
    let mut pk = [0u8; 33];
    pk.copy_from_slice(&bytes);
    Ok((
        SigmaType::SSigmaProp,
        SigmaValue::SigmaProp(SigmaBoolean::ProveDlog(
            ergo_primitives::group_element::GroupElement::from_bytes(pk),
        )),
    ))
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Extract a fixed-length hex payload from a JSON string value.
fn user_hex(tpe: &str, value: &serde_json::Value, want: usize) -> Result<Vec<u8>, SandboxError> {
    let s = value
        .as_str()
        .ok_or_else(|| bad(tpe, value, &format!("a {want}-byte hex string")))?;
    let bytes =
        hex::decode(s.trim()).map_err(|e| SandboxError::Scenario(format!("`{tpe}` hex: {e}")))?;
    if bytes.len() != want {
        return Err(SandboxError::Scenario(format!(
            "`{tpe}` needs {want} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// A JSON integer, or a decimal string — the exact form for a `Long` beyond
/// JavaScript's safe-integer range, which a JSON number would round.
fn value_i64(tpe: &str, value: &serde_json::Value) -> Result<i64, SandboxError> {
    if let Some(n) = value.as_i64() {
        return Ok(n);
    }
    value
        .as_str()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .ok_or_else(|| bad(tpe, value, "a JSON integer (or a decimal string)"))
}

fn bad(tpe: &str, value: &serde_json::Value, expected: &str) -> SandboxError {
    SandboxError::Scenario(format!("type `{tpe}` needs {expected}, got {value}"))
}

fn range(tpe: &str, n: i64, lo: i64, hi: i64) -> SandboxError {
    SandboxError::Scenario(format!("type `{tpe}` value {n} out of range [{lo}, {hi}]"))
}
