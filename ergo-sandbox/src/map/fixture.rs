//! A **recorded** chain source: every response the map needed, archived.
//!
//! This is what lets the acceptance criteria run in CI with no network. A
//! fixture is fetched once, by hand, and committed; from then on the map is
//! reproducible byte for byte against a frozen chain state.
//!
//! Two honesty properties matter:
//!
//! - A query the archive does not answer is a **loud error**
//!   ([`SourceError::Backend`]), never an empty result. A fixture gap must not
//!   read as "the chain has nothing there".
//! - A recorded negative is explicit: `"tokens": { "<id>": null }` means the
//!   chain was asked and said no such token — that is the *evidence* the
//!   classification rests on.
//!
//! Response order is deliberately not meaningful: the map re-sorts every list
//! into its own canonical order, and the determinism test shuffles a fixture's
//! lists to prove it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::source::{ChainBox, ChainSource, Page, SourceError, TokenInfo, TxBoxes};

/// A recorded result set: what the source returned, plus the total it said
/// there were. Keeping the total is what lets a replay report the *same*
/// truncation the live run did, rather than a smaller one invented by the
/// size of the recording.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Recorded {
    /// The boxes, as recorded. Order is not meaningful.
    #[serde(default)]
    pub items: Vec<ChainBox>,
    /// The source's reported total, when it gave one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

/// A recorded chain archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fixture {
    /// Archive schema version.
    pub format_version: u32,
    /// What was recorded from, reported as `chainSource.kind`.
    pub kind: String,
    /// The source's base URL at record time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The chain height the archive is of.
    pub height: u32,
    /// Boxes by id.
    #[serde(default)]
    pub boxes: BTreeMap<String, ChainBox>,
    /// Token evidence by id. `null` records "asked; no such token".
    #[serde(default)]
    pub tokens: BTreeMap<String, Option<TokenInfo>>,
    /// Unspent holders by token id, as recorded (order not meaningful).
    #[serde(default)]
    pub boxes_by_token: BTreeMap<String, Recorded>,
    /// Unspent boxes by address.
    #[serde(default)]
    pub boxes_by_address: BTreeMap<String, Recorded>,
    /// Unspent boxes by `blake2b256(propositionBytes)`. Absent entirely when
    /// the recorded source had no such index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boxes_by_script_hash: Option<BTreeMap<String, Recorded>>,
    /// Transactions by id.
    #[serde(default)]
    pub transactions: BTreeMap<String, TxBoxes>,
}

impl Fixture {
    /// An empty archive at `height`, ready to record into.
    #[must_use]
    pub fn new(kind: &str, url: Option<&str>, height: u32) -> Self {
        Fixture {
            format_version: 1,
            kind: kind.to_string(),
            url: url.map(str::to_string),
            height,
            boxes: BTreeMap::new(),
            tokens: BTreeMap::new(),
            boxes_by_token: BTreeMap::new(),
            boxes_by_address: BTreeMap::new(),
            boxes_by_script_hash: None,
            transactions: BTreeMap::new(),
        }
    }

    /// Parse an archive from JSON.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("map fixture: {e}"))
    }

    /// Render the archive as pretty JSON, ready to commit.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
}

/// Apply the caller's `(offset, limit)` window to a recorded list.
///
/// The archive keeps whole result sets, so a window is a slice: pagination is
/// a property of the live source, not of the recording.
fn page(rec: &Recorded, offset: usize, limit: usize) -> Page {
    Page {
        items: rec.items.iter().skip(offset).take(limit).cloned().collect(),
        total: Some(rec.total.unwrap_or(rec.items.len()).max(rec.items.len())),
    }
}

fn missing(what: &str, key: &str) -> SourceError {
    SourceError::Backend(format!(
        "fixture has no recorded answer for {what} `{key}` — re-record it rather than \
         reading the gap as an absence"
    ))
}

impl ChainSource for Fixture {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn height(&self) -> Result<u32, SourceError> {
        Ok(self.height)
    }

    fn box_by_id(&self, box_id: &str) -> Result<ChainBox, SourceError> {
        self.boxes
            .get(box_id)
            .cloned()
            .ok_or_else(|| missing("box", box_id))
    }

    fn boxes_by_token_id(
        &self,
        token_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        self.boxes_by_token
            .get(token_id)
            .map(|v| page(v, offset, limit))
            .ok_or_else(|| missing("boxes by token id", token_id))
    }

    fn token_info(&self, token_id: &str) -> Result<TokenInfo, SourceError> {
        match self.tokens.get(token_id) {
            Some(Some(info)) => Ok(info.clone()),
            Some(None) => Err(SourceError::NotFound(format!("token {token_id}"))),
            None => Err(missing("token", token_id)),
        }
    }

    fn boxes_by_address(
        &self,
        address: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        self.boxes_by_address
            .get(address)
            .map(|v| page(v, offset, limit))
            .ok_or_else(|| missing("boxes by address", address))
    }

    fn boxes_by_script_hash(
        &self,
        hash: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let Some(index) = self.boxes_by_script_hash.as_ref() else {
            return Err(SourceError::Unsupported("boxes by script hash"));
        };
        // Within a source that *has* the index, an unrecorded hash is a
        // genuine "no such script on chain": the recorder asks for every hash
        // the map raises.
        Ok(index.get(hash).map_or_else(
            || Page {
                items: Vec::new(),
                total: Some(0),
            },
            |v| page(v, offset, limit),
        ))
    }

    fn transaction(&self, tx_id: &str) -> Result<TxBoxes, SourceError> {
        self.transactions
            .get(tx_id)
            .cloned()
            .ok_or_else(|| missing("transaction", tx_id))
    }
}
