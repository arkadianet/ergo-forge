//! Box marshalling: `ScenarioBox` JSON → evaluator `EvalBox`.
//!
//! Follows the `ergo-validation/src/tx/script/eval_box.rs` conversion shape
//! but takes values from scenario JSON instead of wire boxes. Box ids and
//! creation transaction ids default to zero (synthetic boxes) — extraction
//! of `SELF.id` yields zeros unless the user supplies a real id.

use ergo_ser::register::RegisterValue;
use ergo_sigma::evaluator::EvalBox;

use crate::scenario::ScenarioBox;
use crate::SandboxError;

/// Build an owned `EvalBox` from scenario JSON.
///
/// `default_tree_bytes` is the box's own locking script (canonical ErgoTree
/// wire bytes) used when the scenario box omits its tree — the self-box
/// synthesis case, where the spent box carries the tree under evaluation.
/// When supplied, the scenario's `ergoTree` field is IGNORED for this box.
pub fn build_eval_box(
    field: &'static str,
    sb: &ScenarioBox,
    default_tree_bytes: Option<&[u8]>,
) -> Result<EvalBox, SandboxError> {
    let script_bytes: Vec<u8> = match (&sb.ergo_tree, default_tree_bytes) {
        (Some(hex_str), _) => {
            hex::decode(hex_str.trim()).map_err(|source| SandboxError::Hex { field, source })?
        }
        (None, Some(bytes)) => bytes.to_vec(),
        (None, None) => {
            return Err(SandboxError::Scenario(format!(
                "`{field}` box needs `ergoTree` (only the self box may omit it)"
            )))
        }
    };

    let id: [u8; 32] = match &sb.box_id {
        Some(s) => hex::decode(s.trim())
            .map_err(|source| SandboxError::Hex { field, source })?
            .try_into()
            .map_err(|_| SandboxError::Scenario(format!("`{field}` box id must be 32 bytes")))?,
        None => [0u8; 32],
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

    Ok(EvalBox {
        creation_height: sb.creation_height,
        script_bytes,
        value: sb.value,
        id,
        transaction_id: [0u8; 32],
        output_index: 0,
        registers,
        tokens,
        raw_bytes: Vec::new(),
        register_bytes: Vec::new(),
    })
}
