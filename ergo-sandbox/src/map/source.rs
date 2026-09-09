//! The chain source: what the map is allowed to ask a chain about.
//!
//! Deliberately small and deliberately *pull*-shaped, so an explorer, a node
//! with the blockchain indexer, and a **recorded fixture** are
//! interchangeable. The fixture implementation is what makes the acceptance
//! criteria run in CI with no network.
//!
//! Everything a source returns is **data**: re-parsed by the engine, never
//! trusted as typed — the same discipline `ergo-web`'s `lookup` route applies
//! to explorer responses.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Why a chain query produced no answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// The chain has no such box / token / transaction.
    #[error("not found: {0}")]
    NotFound(String),
    /// This source cannot answer this kind of query at all — the explorer
    /// has no `blake2b256(propositionBytes)` index, for instance. Distinct
    /// from `NotFound`: it is a gap in the *source*, not in the chain, and
    /// the map reports it as an unresolved edge rather than an absence.
    #[error("unsupported by this chain source: {0}")]
    Unsupported(&'static str),
    /// Transport or decoding failure.
    #[error("chain source failed: {0}")]
    Backend(String),
}

/// One token entry on a box, in `tokens` index order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainToken {
    /// Token id, lowercase hex.
    pub id: String,
    /// Amount held by the box.
    pub amount: u64,
}

/// A box as the map needs it. A strict subset of what any source returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainBox {
    /// Box id, lowercase hex.
    pub box_id: String,
    /// Serialized ErgoTree — the box's `propositionBytes` — as hex.
    pub ergo_tree: String,
    /// nanoERG.
    pub value: u64,
    /// Tokens in index order.
    #[serde(default)]
    pub tokens: Vec<ChainToken>,
    /// R4–R9 as serialized constant hex, preserved without decoding. Accepts
    /// node hex strings and explorer objects containing `serializedValue`.
    /// Older recordings without this field cannot recover omitted registers.
    #[serde(
        default,
        rename = "additionalRegisters",
        alias = "registers",
        deserialize_with = "deserialize_registers"
    )]
    pub registers: BTreeMap<String, String>,
    /// Height the box was created at.
    #[serde(default)]
    pub creation_height: u32,
    /// Height the box entered the chain at. Half of the canonical selection
    /// order `(inclusionHeight, boxId)`; sources that cannot supply it leave
    /// it 0, which degrades the order to box id alone — still deterministic.
    #[serde(default)]
    pub inclusion_height: u32,
}

fn deserialize_registers<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    parse_registers(&value).map_err(serde::de::Error::custom)
}

/// A decoded-only register cannot be losslessly reconstructed. Report that
/// source limitation explicitly instead of silently dropping the entry.
pub(super) fn parse_registers(
    value: &serde_json::Value,
) -> Result<BTreeMap<String, String>, String> {
    if value.is_null() {
        return Ok(BTreeMap::new());
    }
    let entries = value
        .as_object()
        .ok_or_else(|| "additionalRegisters must be an object".to_string())?;
    entries
        .iter()
        .map(|(name, value)| {
            value
                .as_str()
                .or_else(|| {
                    value
                        .get("serializedValue")
                        .and_then(serde_json::Value::as_str)
                })
                .map(|hex| (name.clone(), hex.to_string()))
                .ok_or_else(|| {
                    format!(
                        "register {name} cannot be represented as raw hex: missing serializedValue"
                    )
                })
        })
        .collect()
}

impl ChainBox {
    /// Canonical selection key: `(inclusionHeight, boxId)` ascending. The
    /// order is a property of the *map*, never of a source's response order.
    #[must_use]
    pub fn order_key(&self) -> (u32, &str) {
        (self.inclusion_height, self.box_id.as_str())
    }
}

/// What a token is, as far as identity goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    /// Token id, lowercase hex.
    pub id: String,
    /// Total minted. `1` is the singleton case — the only one that lets a
    /// token id identify a unique box.
    pub emission_amount: u64,
}

impl TokenInfo {
    /// A singleton: exactly one unit exists, so holding it identifies a box.
    #[must_use]
    pub fn is_singleton(&self) -> bool {
        self.emission_amount == 1
    }
}

/// One page of boxes, plus how many the source says there are in total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// The boxes in this page, in whatever order the source produced them.
    /// The map re-sorts; callers must not depend on this order.
    pub items: Vec<ChainBox>,
    /// Total matching boxes the source knows of, when it says. `None` means
    /// "unknown", and the map then reports truncation only when a page
    /// filled to its limit.
    #[serde(default)]
    pub total: Option<usize>,
}

/// The boxes a transaction touched — the frontier a transaction seed gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxBoxes {
    /// Boxes the transaction spent.
    pub inputs: Vec<ChainBox>,
    /// Boxes it created.
    pub outputs: Vec<ChainBox>,
}

/// What the map may ask a chain.
///
/// Every method is fallible and every paginated method takes an explicit
/// `(offset, limit)` window, so a source can never surprise the traversal
/// with an unbounded response.
pub trait ChainSource {
    /// Short machine name recorded in the output's `chainSource.kind`,
    /// e.g. `"explorer"`, `"fixture"`.
    fn kind(&self) -> &str;

    /// Base URL, when the source has one. Recorded verbatim.
    fn url(&self) -> Option<&str> {
        None
    }

    /// Chain height the map is of. A map is of *now*, and this is the *now*.
    fn height(&self) -> Result<u32, SourceError>;

    /// One box by id.
    fn box_by_id(&self, box_id: &str) -> Result<ChainBox, SourceError>;

    /// Unspent boxes holding a token, paginated.
    fn boxes_by_token_id(
        &self,
        token_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError>;

    /// Emission amount and id for a token — the evidence that decides
    /// whether a 32-byte constant is a token id at all, and whether it is a
    /// singleton.
    fn token_info(&self, token_id: &str) -> Result<TokenInfo, SourceError>;

    /// Unspent boxes at an address, paginated.
    fn boxes_by_address(
        &self,
        address: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError>;

    /// Unspent boxes whose `blake2b256(propositionBytes)` equals `hash`.
    ///
    /// Optional: the default says [`SourceError::Unsupported`], which the map
    /// reports as an unresolved edge rather than pretending the collaborator
    /// does not exist. The public explorer has no such index.
    fn boxes_by_script_hash(
        &self,
        _hash: &str,
        _offset: usize,
        _limit: usize,
    ) -> Result<Page, SourceError> {
        Err(SourceError::Unsupported("boxes by script hash"))
    }

    /// A transaction's inputs and outputs, for a transaction seed.
    fn transaction(&self, tx_id: &str) -> Result<TxBoxes, SourceError>;
}

/// Capture the original source document before the legacy ChainBox defaulting
/// boundary. Its digest identifies this JSON record, not an Ergo box or UTXO.
/// No map traversal, selection, or defaulting behavior changes.
pub fn record_box(
    document: serde_json::Value,
    locator: String,
    revision: Option<String>,
) -> Result<crate::evidence::RecordedBox, String> {
    let sha256 = crate::evidence::case::json_digest(&document);
    crate::evidence::RecordedBox::new(
        document,
        crate::evidence::Origin::SourceRecorded,
        Some(crate::evidence::SourceRecord {
            locator,
            revision,
            sha256,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn absent_registers_are_empty_and_decoded_only_registers_report_a_limit() {
        let mut b = json!({"boxId": "00", "ergoTree": "10010101d17300", "value": 1});
        assert!(serde_json::from_value::<ChainBox>(b.clone())
            .unwrap()
            .registers
            .is_empty());
        for empty in [json!({}), serde_json::Value::Null] {
            b["additionalRegisters"] = empty;
            assert!(serde_json::from_value::<ChainBox>(b.clone())
                .unwrap()
                .registers
                .is_empty());
        }
        b["additionalRegisters"] = json!({"R4": {"sigmaType": "SInt", "renderedValue": "7"}});
        let error = serde_json::from_value::<ChainBox>(b)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("register R4 cannot be represented as raw hex: missing serializedValue")
        );
    }
}
