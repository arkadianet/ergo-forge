//! Box marshalling: `ScenarioBox` JSON → evaluator `EvalBox`.
//!
//! Follows the `ergo-validation/src/tx/script/eval_box.rs` conversion shape
//! but takes values from scenario JSON instead of wire boxes. A synthetic
//! box is serialized like a real one (candidate, then a zero transaction id
//! and index 0), so `SELF.bytes`, `bytesWithoutRef` and `id` are what the
//! chain would compute for it — `id` is `blake2b256(bytes)` unless the
//! scenario supplies one.

use ergo_primitives::digest::{Digest32, ModifierId};
use ergo_primitives::reader::VlqReader;
use ergo_primitives::writer::VlqWriter;
use ergo_ser::ergo_box::{write_ergo_box, ErgoBox, ErgoBoxCandidate};
use ergo_ser::ergo_tree::read_ergo_tree;
use ergo_ser::register::{write_registers, AdditionalRegisters, RegisterValue};
use ergo_ser::token::Token;
use ergo_sigma::evaluator::EvalBox;

use crate::scenario::ScenarioBox;
use crate::SandboxError;

/// Build an owned `EvalBox` from scenario JSON.
///
/// `default_tree_bytes` is the box's own locking script (canonical ErgoTree
/// wire bytes) used when the scenario box omits its tree — the self-box
/// synthesis case, where the spent box carries the tree under evaluation.
/// When supplied, it takes PRECEDENCE over the scenario's `ergoTree` field
/// (used only for the self box, so `SELF` always exposes the tree under
/// evaluation).
pub fn build_eval_box(
    field: &'static str,
    sb: &ScenarioBox,
    default_tree_bytes: Option<&[u8]>,
) -> Result<EvalBox, SandboxError> {
    let script_bytes: Vec<u8> = match default_tree_bytes {
        Some(bytes) => bytes.to_vec(),
        None => match sb
            .ergo_tree
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(hex_str) => {
                hex::decode(hex_str).map_err(|source| SandboxError::Hex { field, source })?
            }
            None => {
                return Err(SandboxError::Scenario(format!(
                    "`{field}` box needs `ergoTree` (only the self box may omit it)"
                )))
            }
        },
    };

    let given_id: Option<[u8; 32]> = match &sb.box_id {
        Some(s) => Some(
            hex::decode(s.trim())
                .map_err(|source| SandboxError::Hex { field, source })?
                .try_into()
                .map_err(|_| {
                    SandboxError::Scenario(format!("`{field}` box id must be 32 bytes"))
                })?,
        ),
        None => None,
    };

    // Registers R4–R9, dense from R4 (index 0 = R4). BTreeMap iteration is
    // sorted, so R4 < R5 < … < R9 order is enforced by construction.
    let mut registers: [Option<RegisterValue>; 6] = [None, None, None, None, None, None];
    for (slot, (key, tv)) in sb.registers.iter().enumerate() {
        let expected = format!("R{}", 4 + slot);
        if *key != expected {
            return Err(SandboxError::Scenario(format!(
                "`{field}` registers must be dense from R4: expected `{expected}`, found `{key}`"
            )));
        }
        let (tpe, value) = crate::scenario::parse_typed_value(&tv.r#type, &tv.value)?;
        registers[slot] = Some(RegisterValue { tpe, value });
    }

    let mut tokens = Vec::with_capacity(sb.tokens.len());
    for t in &sb.tokens {
        let tid = hex::decode(t.id.trim())
            .map_err(|source| SandboxError::Hex { field, source })?
            .try_into()
            .map_err(|_| SandboxError::Scenario(format!("`{field}` token id must be 32 bytes")))?;
        tokens.push((tid, t.amount));
    }

    // Serialize the box the way the chain would, for `bytes`, `bytesWithoutRef`
    // and (unless given) `id`.
    let ergo_tree = read_ergo_tree(&mut VlqReader::new(&script_bytes))
        .map_err(|e| SandboxError::Scenario(format!("`{field}` ergoTree does not parse: {e}")))?;
    let additional_registers = AdditionalRegisters {
        registers: registers.iter().flatten().cloned().collect(),
    };
    let mut rw = VlqWriter::new();
    write_registers(&mut rw, &additional_registers).map_err(|e| {
        SandboxError::Scenario(format!("`{field}` registers do not serialize: {e}"))
    })?;
    let register_bytes = rw.result();
    let candidate = ErgoBoxCandidate::from_trusted_raw_parts(
        u64::try_from(sb.value)
            .map_err(|_| SandboxError::Scenario(format!("`{field}` value must not be negative")))?,
        ergo_tree,
        script_bytes.clone(),
        sb.creation_height,
        tokens
            .iter()
            .map(|(id, amount)| Token {
                token_id: Digest32::from_bytes(*id),
                amount: *amount,
            })
            .collect(),
        additional_registers,
        register_bytes.clone(),
    );
    let ergo_box = ErgoBox {
        candidate,
        transaction_id: ModifierId::from_bytes([0u8; 32]),
        index: 0,
    };
    let mut w = VlqWriter::new();
    write_ergo_box(&mut w, &ergo_box)
        .map_err(|e| SandboxError::Scenario(format!("`{field}` box does not serialize: {e}")))?;
    let raw_bytes = w.result();
    let id = match given_id {
        Some(id) => id,
        None => *ergo_primitives::digest::blake2b256(&raw_bytes).as_bytes(),
    };

    Ok(EvalBox {
        creation_height: sb.creation_height,
        script_bytes,
        value: sb.value,
        id,
        transaction_id: [0u8; 32],
        output_index: 0,
        registers,
        tokens,
        raw_bytes,
        register_bytes,
    })
}
