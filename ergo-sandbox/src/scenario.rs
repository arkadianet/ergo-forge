//! The JSON scenario model: user-facing describing of a spending context,
//! plus the typed-value parser that lifts user JSON into
//! `SigmaType`/`SigmaValue` pairs, `EvalBox`es, and context variables.
//!
//! This is presentation-layer sugar over the evaluator's owned types. It
//! accepts the value forms a playground user can produce from JSON and
//! rejects everything else explicitly — never guessing.

use std::collections::BTreeMap;

use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};
use num_bigint::BigInt;
use serde::Deserialize;

use crate::SandboxError;

// ── Scenario model ───────────────────────────────────────────────────────────

/// A spending scenario: the tree under evaluation plus everything the
/// evaluator may observe. Deserializable from the workbench JSON.
///
/// Field names are camelCase (`creationHeight`, `contextVars`, …) to match
/// the tooling-API request shapes (`ergoscript-tooling-api.md` §4.3).
#[derive(Debug, Clone, Deserialize)]
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
    #[serde(default)]
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
    /// Message the proof commits to (bytes-to-sign), hex. Default empty —
    /// real spends sign the transaction bytes.
    #[serde(default)]
    pub message: Option<String>,
}

/// Pre-header fields (SPreHeader). All optional; defaults are zero.
#[derive(Debug, Clone, Default, Deserialize)]
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
#[derive(Debug, Clone, Default, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmount {
    /// 32-byte token id, hex.
    pub id: String,
    /// Token amount.
    pub amount: u64,
}

/// A user-supplied typed value: a type name plus a JSON value of the
/// matching shape.
#[derive(Debug, Clone, Deserialize)]
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
                 BigInt, GroupElement, SigmaProp, Coll[T])"
            )))
        }
    })
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
        _ => Err(SandboxError::Scenario(format!(
            "unsupported type name `{tpe}` (supported: Boolean, Byte, Short, Int, Long, BigInt, \
             GroupElement, SigmaProp, Coll[T])"
        ))),
    }
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
    let bytes = user_hex("SigmaProp", value, 33)?;
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

fn value_i64(tpe: &str, value: &serde_json::Value) -> Result<i64, SandboxError> {
    value
        .as_i64()
        .ok_or_else(|| bad(tpe, value, "a JSON integer"))
}

fn bad(tpe: &str, value: &serde_json::Value, expected: &str) -> SandboxError {
    SandboxError::Scenario(format!("type `{tpe}` needs {expected}, got {value}"))
}

fn range(tpe: &str, n: i64, lo: i64, hi: i64) -> SandboxError {
    SandboxError::Scenario(format!("type `{tpe}` value {n} out of range [{lo}, {hi}]"))
}
