//! Storage rent, the one rule that applies to EVERY box regardless of its
//! script: after a storage period (mainnet 1,051,200 blocks, about four
//! years) a miner may spend a box without satisfying its script, paying
//! itself a fee of `bytes × factor` (mainnet 1,250,000 nanoERG per byte)
//! and recreating the box with the same script, tokens and registers minus
//! the fee — or, when the value cannot cover the fee, taking the whole box.
//! (`ergo-validation/src/tx/script/storage_rent_check.rs` in the node.)
//!
//! Users of any contract should know this: a near-empty box is swept, and
//! every box is shaved by the fee each period.

use serde::Serialize;

/// Mainnet storage period in blocks (votable; frozen snapshot).
pub const STORAGE_PERIOD: u32 = 1_051_200;
/// Mainnet storage fee factor, nanoERG per byte per period (votable).
pub const STORAGE_FEE_FACTOR: i32 = 1_250_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentEstimate {
    /// Serialized box size the fee is charged on (an estimate of the
    /// canonical `ErgoBox` serialization: value, tree, height, tokens,
    /// registers, transaction id and index).
    pub box_bytes: usize,
    /// Fee per period, nanoERG. A box holding less than this is swept.
    pub fee_nanoerg: u64,
    pub period_blocks: u32,
    pub fee_factor: i32,
    /// Creation height + one period, when the creation height is known.
    pub next_collection_height: Option<u32>,
}

fn vlq_len(mut v: u64) -> usize {
    let mut n = 1;
    while v >= 0x80 {
        v >>= 7;
        n += 1;
    }
    n
}

/// Estimate the rent for a box with this tree, these token amounts, and
/// these serialized register values. The value is assumed to take a
/// five-byte VLQ (an ERG-scale amount); the estimate is within a few
/// bytes of the node's own serialization.
pub fn estimate(
    tree_bytes: &[u8],
    token_amounts: &[u64],
    register_values: &[Vec<u8>],
    creation_height: Option<u32>,
) -> RentEstimate {
    let value_len = 5;
    let height_len = vlq_len(creation_height.unwrap_or(1_500_000) as u64);
    let tokens_len = 1 + token_amounts
        .iter()
        .map(|a| 32 + vlq_len(*a))
        .sum::<usize>();
    let regs_len = 1 + register_values.iter().map(Vec::len).sum::<usize>();
    let tx_ref = 32 + 2;
    let box_bytes = value_len + tree_bytes.len() + height_len + tokens_len + regs_len + tx_ref;
    // The node multiplies as i32 with wrapping (consensus quirk); for any
    // box under the wrap point (~1,718 bytes) this is the plain product.
    let fee_nanoerg = (box_bytes as i32).wrapping_mul(STORAGE_FEE_FACTOR).max(0) as u64;
    RentEstimate {
        box_bytes,
        fee_nanoerg,
        period_blocks: STORAGE_PERIOD,
        fee_factor: STORAGE_FEE_FACTOR,
        next_collection_height: creation_height.map(|h| h.saturating_add(STORAGE_PERIOD)),
    }
}

/// Rent for the context box, or minimal synthetic SELF, using the frozen estimate.
pub fn estimate_for(tree_bytes: &[u8], b: Option<&crate::ScenarioBox>) -> RentEstimate {
    match b {
        Some(b) => {
            let amounts: Vec<u64> = b.tokens.iter().map(|t| t.amount).collect();
            // Each register's canonical serialized constant (type + value),
            // which is what the box serialization carries.
            let regs: Vec<Vec<u8>> = b
                .registers
                .values()
                .map(|tv| match (tv.r#type.as_str(), tv.value.as_str()) {
                    ("raw", Some(h)) => hex::decode(h).unwrap_or_default(),
                    _ => crate::parse_typed_value(&tv.r#type, &tv.value)
                        .ok()
                        .and_then(|(t, v)| {
                            let mut w = ergo_primitives::writer::VlqWriter::new();
                            ergo_ser::sigma_value::write_constant(&mut w, &t, &v)
                                .ok()
                                .map(|_| w.result())
                        })
                        .unwrap_or_default(),
                })
                .collect();
            let created = if b.creation_height > 0 {
                Some(b.creation_height)
            } else {
                None
            };
            crate::rent::estimate(tree_bytes, &amounts, &regs, created)
        }
        None => crate::rent::estimate(tree_bytes, &[], &[], None),
    }
}

/// Static Read text and its evidence basis. The cited accepted P03 vector is
/// distinct from this estimate; no future claim or collection is predicted.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentLine {
    pub provenance: crate::checklist::Provenance,
    pub node_validated: bool,
    pub text: &'static str,
    pub basis: [&'static str; 3],
    pub estimate: RentEstimate,
}

pub fn read_line(estimate: RentEstimate) -> RentLine {
    RentLine {
        provenance: crate::checklist::Provenance::Static,
        node_validated: false,
        text: "After four years, anyone may claim this box for the rent through the miner storage-rent path. Static estimate under frozen mainnet parameters; collection is not predicted.",
        basis: ["ergo-sandbox/src/rent.rs::STORAGE_PERIOD", "ergo-sandbox/src/rent.rs::STORAGE_FEE_FACTOR", "P03:storage-rent-acceptance"],
        estimate,
    }
}
