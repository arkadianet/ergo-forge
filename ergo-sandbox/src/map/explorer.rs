//! The live public-explorer chain source, and the **recorder** that turns one
//! live run into a committed fixture.
//!
//! Behind the `explorer` feature so the crate can be built network-free. The
//! map's own tests never reach this module: they run against
//! [`super::Fixture`], which is what the recorder writes.
//!
//! Everything the explorer returns is data. Fields are read defensively and
//! re-parsed by the engine — the same discipline `ergo-web`'s `lookup` route
//! applies.

use std::cell::RefCell;
use std::time::Duration;

use serde_json::Value;

use super::fixture::{Fixture, Recorded};
use super::source::{ChainBox, ChainSource, ChainToken, Page, SourceError, TokenInfo, TxBoxes};

/// The default public explorer.
pub const DEFAULT_EXPLORER_URL: &str = "https://api.ergoplatform.com";

/// A read-only client for the Ergo explorer's v1 API.
pub struct ExplorerSource {
    base: String,
    agent: ureq::Agent,
}

impl ExplorerSource {
    /// A client against `base` (no trailing slash), with a short timeout.
    #[must_use]
    pub fn new(base: &str) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build();
        ExplorerSource {
            base: base.trim_end_matches('/').to_string(),
            agent: config.into(),
        }
    }

    /// GET `path`, retrying a transport failure a couple of times. A public
    /// explorer drops the occasional connection; a map that gave up on the
    /// first one would be reported as truncated for no chain reason.
    fn get(&self, path: &str) -> Result<Value, SourceError> {
        let mut last = SourceError::Backend("no attempt".into());
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(400 * attempt));
            }
            match self.get_once(path) {
                Err(SourceError::Backend(m)) => last = SourceError::Backend(m),
                other => return other,
            }
        }
        Err(last)
    }

    fn get_once(&self, path: &str) -> Result<Value, SourceError> {
        let url = format!("{}{path}", self.base);
        match self.agent.get(&url).call() {
            Ok(mut r) => {
                let text = r.body_mut().read_to_string().map_err(|e| {
                    SourceError::Backend(format!("explorer response unreadable: {e}"))
                })?;
                serde_json::from_str(&text)
                    .map_err(|e| SourceError::Backend(format!("explorer answered non-JSON: {e}")))
            }
            Err(ureq::Error::StatusCode(404)) => Err(SourceError::NotFound(path.to_string())),
            Err(e) => Err(SourceError::Backend(format!(
                "explorer request failed: {e}"
            ))),
        }
    }
}

/// Explorer box JSON → [`ChainBox`], reading only the fields the map needs.
///
/// Strict about the fields the map *reasons* with — `boxId`, `ergoTree`,
/// `value`, and every token's id and amount. A default there would be a
/// fabricated fact: it would flow into `json::canonical`, into a role
/// decision, and into a recorded fixture, indistinguishable from something
/// the chain actually said. `creationHeight`/`settlementHeight` are the
/// exception and default to 0, which only degrades the selection order to box
/// id alone — documented on [`ChainBox`].
fn to_box(v: &Value) -> Result<ChainBox, SourceError> {
    let registers = super::source::parse_registers(&v["additionalRegisters"])
        .map_err(|e| SourceError::Backend(format!("box {}: {e}", v["boxId"])))?;
    let parse = || {
        Some(ChainBox {
            box_id: v.get("boxId")?.as_str()?.to_ascii_lowercase(),
            ergo_tree: v.get("ergoTree")?.as_str()?.to_ascii_lowercase(),
            value: v.get("value")?.as_u64()?,
            registers,
            tokens: match v.get("assets") {
                None | Some(Value::Null) => Vec::new(),
                Some(a) => a
                    .as_array()?
                    .iter()
                    .map(|t| {
                        Some(ChainToken {
                            id: t.get("tokenId")?.as_str()?.to_ascii_lowercase(),
                            amount: t.get("amount")?.as_u64()?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            },
            creation_height: v.get("creationHeight").and_then(Value::as_u64).unwrap_or(0) as u32,
            // `settlementHeight` is the height the box entered the chain at — the
            // stable half of the map's canonical selection order.
            inclusion_height: v
                .get("settlementHeight")
                .or_else(|| v.get("inclusionHeight"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
        })
    };
    parse().ok_or_else(|| SourceError::Backend(format!("box {}: unusable JSON", v["boxId"])))
}

/// A page of boxes. One unusable item fails the whole page: silently dropping
/// it would shrink a result set the map then reports as complete.
fn to_page(v: &Value) -> Result<Page, SourceError> {
    let items = match v.get("items") {
        None | Some(Value::Null) => Vec::new(),
        Some(a) => a
            .as_array()
            .ok_or_else(|| SourceError::Backend("explorer page `items` is not an array".into()))?
            .iter()
            .map(to_box)
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(Page {
        items,
        total: v.get("total").and_then(Value::as_u64).map(|t| t as usize),
    })
}

impl ChainSource for ExplorerSource {
    fn kind(&self) -> &str {
        "explorer"
    }

    fn url(&self) -> Option<&str> {
        Some(&self.base)
    }

    fn height(&self) -> Result<u32, SourceError> {
        self.get("/api/v1/networkState")?
            .get("height")
            .and_then(Value::as_u64)
            .map(|h| h as u32)
            .ok_or_else(|| SourceError::Backend("networkState has no height".into()))
    }

    fn box_by_id(&self, box_id: &str) -> Result<ChainBox, SourceError> {
        let v = self.get(&format!("/api/v1/boxes/{box_id}"))?;
        to_box(&v)
    }

    fn boxes_by_token_id(
        &self,
        token_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let v = self.get(&format!(
            "/api/v1/boxes/unspent/byTokenId/{token_id}?offset={offset}&limit={limit}"
        ))?;
        to_page(&v)
    }

    fn token_info(&self, token_id: &str) -> Result<TokenInfo, SourceError> {
        let v = self.get(&format!("/api/v1/tokens/{token_id}"))?;
        Ok(TokenInfo {
            id: token_id.to_ascii_lowercase(),
            emission_amount: v.get("emissionAmount").and_then(Value::as_u64).unwrap_or(0),
        })
    }

    fn boxes_by_address(
        &self,
        address: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let v = self.get(&format!(
            "/api/v1/boxes/unspent/byAddress/{address}?offset={offset}&limit={limit}"
        ))?;
        to_page(&v)
    }

    // `boxes_by_script_hash` is deliberately left at the trait default. The
    // explorer indexes an ErgoTree *template* hash, which is not
    // `blake2b256(propositionBytes)`: answering with it would silently return
    // the wrong boxes. `Unsupported` is the honest answer, and the map turns
    // it into an unresolved edge rather than an absence.

    fn transaction(&self, tx_id: &str) -> Result<TxBoxes, SourceError> {
        let v = self.get(&format!("/api/v1/transactions/{tx_id}"))?;
        transaction_boxes(&v, |id| self.get(&format!("/api/v1/boxes/{id}")))
    }
}

/// Transaction parsing preserves original hex spelling and requires creation
/// height. The map's legacy box normalisation/defaults remain unchanged.
/// Explorer input/data-input references are hydrated through the same read-only
/// box endpoint; the closure also makes this entire adapter testable offline.
fn transaction_boxes(
    v: &Value,
    mut fetch_box: impl FnMut(&str) -> Result<Value, SourceError>,
) -> Result<TxBoxes, SourceError> {
    let mut side = |key: &str| -> Result<Vec<ChainBox>, SourceError> {
        let entries = match v.get(key) {
            None | Some(Value::Null) if key == "dataInputs" => return Ok(Vec::new()),
            Some(Value::Array(a)) => a,
            _ => {
                return Err(SourceError::Backend(format!(
                    "transaction `{key}` is not an array"
                )))
            }
        };
        entries
            .iter()
            .map(|entry| {
                let id = entry
                    .get("boxId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| SourceError::Backend("transaction box has no boxId".into()))?;
                let recorded;
                let b = if [
                    "ergoTree",
                    "value",
                    "creationHeight",
                    "assets",
                    "additionalRegisters",
                ]
                .iter()
                .all(|key| entry.get(key).is_some_and(|v| !v.is_null()))
                {
                    entry
                } else {
                    recorded = fetch_box(id)?;
                    if recorded
                        .get("boxId")
                        .and_then(Value::as_str)
                        .map(str::to_ascii_lowercase)
                        != Some(id.to_ascii_lowercase())
                    {
                        return Err(SourceError::Backend(
                            "hydrated boxId differs from transaction reference".into(),
                        ));
                    }
                    &recorded
                };
                if ["assets", "additionalRegisters"]
                    .iter()
                    .any(|key| b.get(key).is_none_or(Value::is_null))
                {
                    return Err(SourceError::Backend(format!(
                        "box {id}: missing tokens/registers after hydration"
                    )));
                }
                let mut parsed = to_box(b)?;
                parsed.creation_height = b
                    .get("creationHeight")
                    .and_then(Value::as_u64)
                    .and_then(|h| u32::try_from(h).ok())
                    .ok_or_else(|| {
                        SourceError::Backend(format!("box {id}: missing/invalid creationHeight"))
                    })?;
                parsed.box_id = b["boxId"].as_str().unwrap().to_string();
                parsed.ergo_tree = b["ergoTree"].as_str().unwrap().to_string();
                for (token, raw) in parsed
                    .tokens
                    .iter_mut()
                    .zip(b["assets"].as_array().into_iter().flatten())
                {
                    token.id = raw["tokenId"].as_str().unwrap().to_string();
                }
                Ok(parsed)
            })
            .collect()
    };
    let inclusion_height = match v.get("inclusionHeight") {
        None | Some(Value::Null) => None,
        Some(h) => Some(
            h.as_u64()
                .and_then(|h| u32::try_from(h).ok())
                .ok_or_else(|| {
                    SourceError::Backend("invalid transaction inclusionHeight".into())
                })?,
        ),
    };
    Ok(TxBoxes {
        inputs: side("inputs")?,
        outputs: side("outputs")?,
        data_inputs: side("dataInputs")?,
        inclusion_height,
    })
}

/// Merge one page into a recorded result set, keeping the source's total.
fn record(slot: &mut Recorded, p: &Page) {
    for b in &p.items {
        if !slot.items.iter().any(|x| x.box_id == b.box_id) {
            slot.items.push(b.clone());
        }
    }
    if p.total.is_some() {
        slot.total = p.total;
    }
}

/// Wraps a source and archives every answer, so one live run becomes a
/// fixture the acceptance criteria can replay offline forever.
///
/// Negatives are archived too: a token the chain does not know is recorded as
/// `null`, because "asked, and the chain said no" is the *evidence* a
/// classification rests on.
pub struct RecordingSource<S: ChainSource> {
    inner: S,
    archive: RefCell<Fixture>,
}

impl<S: ChainSource> RecordingSource<S> {
    /// Start recording `inner`.
    pub fn new(inner: S) -> Result<Self, SourceError> {
        let height = inner.height()?;
        let archive = Fixture::new(inner.kind(), inner.url(), height);
        Ok(RecordingSource {
            inner,
            archive: RefCell::new(archive),
        })
    }

    /// The archive so far.
    #[must_use]
    pub fn finish(self) -> Fixture {
        self.archive.into_inner()
    }
}

impl<S: ChainSource> ChainSource for RecordingSource<S> {
    fn kind(&self) -> &str {
        self.inner.kind()
    }

    fn url(&self) -> Option<&str> {
        self.inner.url()
    }

    fn height(&self) -> Result<u32, SourceError> {
        Ok(self.archive.borrow().height)
    }

    fn box_by_id(&self, box_id: &str) -> Result<ChainBox, SourceError> {
        let b = self.inner.box_by_id(box_id)?;
        self.archive
            .borrow_mut()
            .boxes
            .insert(box_id.to_string(), b.clone());
        Ok(b)
    }

    fn boxes_by_token_id(
        &self,
        token_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let p = self.inner.boxes_by_token_id(token_id, offset, limit)?;
        record(
            self.archive
                .borrow_mut()
                .boxes_by_token
                .entry(token_id.to_string())
                .or_default(),
            &p,
        );
        Ok(p)
    }

    fn token_info(&self, token_id: &str) -> Result<TokenInfo, SourceError> {
        match self.inner.token_info(token_id) {
            Ok(info) => {
                self.archive
                    .borrow_mut()
                    .tokens
                    .insert(token_id.to_string(), Some(info.clone()));
                Ok(info)
            }
            Err(SourceError::NotFound(m)) => {
                self.archive
                    .borrow_mut()
                    .tokens
                    .insert(token_id.to_string(), None);
                Err(SourceError::NotFound(m))
            }
            Err(e) => Err(e),
        }
    }

    fn boxes_by_address(
        &self,
        address: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let p = self.inner.boxes_by_address(address, offset, limit)?;
        record(
            self.archive
                .borrow_mut()
                .boxes_by_address
                .entry(address.to_string())
                .or_default(),
            &p,
        );
        Ok(p)
    }

    fn boxes_by_script_hash(
        &self,
        hash: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let p = self.inner.boxes_by_script_hash(hash, offset, limit)?;
        let mut a = self.archive.borrow_mut();
        let index = a.boxes_by_script_hash.get_or_insert_with(Default::default);
        record(index.entry(hash.to_string()).or_default(), &p);
        Ok(p)
    }

    fn transaction(&self, tx_id: &str) -> Result<TxBoxes, SourceError> {
        let t = self.inner.transaction(tx_id)?;
        self.archive
            .borrow_mut()
            .transactions
            .insert(tx_id.to_string(), t.clone());
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn incident_transaction_adapter_preserves_hex_and_hydrates_references() {
        let b = json!({"boxId": "AB".repeat(32), "ergoTree": "10010101D17300",
            "value": 9007199254740993u64, "creationHeight": 123,
            "assets": [{"tokenId": "Cd".repeat(32), "amount": 7}],
            "additionalRegisters": {"R4": {"serializedValue": "0e02ABcd"}}});
        let mut fetched = Vec::new();
        let tx = json!({"inputs": [{"boxId": b["boxId"]}], "outputs": [b],
            "dataInputs": [{"boxId": b["boxId"]}], "inclusionHeight": 456});
        let parsed = transaction_boxes(&tx, |id| {
            fetched.push(id.to_string());
            Ok(b.clone())
        })
        .unwrap();
        assert_eq!(fetched, vec![b["boxId"].as_str().unwrap(); 2]);
        assert_eq!(parsed.inputs, parsed.outputs);
        assert_eq!(parsed.data_inputs, parsed.outputs);
        assert_eq!(parsed.inclusion_height, Some(456));
        assert_eq!(parsed.inputs[0].value, 9007199254740993);
        assert_eq!(parsed.inputs[0].ergo_tree, "10010101D17300");
        assert_eq!(parsed.inputs[0].tokens[0].id, "Cd".repeat(32));
        assert_eq!(parsed.inputs[0].registers["R4"], "0e02ABcd");
        let mut missing = tx.clone();
        missing.as_object_mut().unwrap().remove("inclusionHeight");
        assert_eq!(
            transaction_boxes(&missing, |_| Ok(b.clone()))
                .unwrap()
                .inclusion_height,
            None
        );
        let mut bad = b.clone();
        bad.as_object_mut().unwrap().remove("creationHeight");
        assert!(transaction_boxes(&tx, |_| Ok(bad.clone()))
            .unwrap_err()
            .to_string()
            .contains("creationHeight"));
        bad = b.clone();
        bad["boxId"] = json!("00".repeat(32));
        assert!(transaction_boxes(&tx, |_| Ok(bad.clone())).is_err());
    }

    #[test]
    fn explorer_registers_survive_box_and_page_parsing() {
        let mut b = json!({
            "boxId": "00", "ergoTree": "10010101d17300", "value": 1,
            "additionalRegisters": {
                "R4": {"serializedValue": "040e", "sigmaType": "SInt", "renderedValue": "7"},
                "R5": "050e"
            }
        });
        let parsed = to_box(&b).unwrap();
        assert_eq!(parsed.registers["R4"], "040e");
        assert_eq!(parsed.registers["R5"], "050e");
        assert_eq!(
            to_page(&json!({"items": [b.clone()]})).unwrap().items,
            vec![parsed]
        );

        b["additionalRegisters"]["R4"] = json!({"renderedValue": "7"});
        let error = to_page(&json!({"items": [b]})).unwrap_err().to_string();
        assert!(
            error.contains("register R4 cannot be represented as raw hex: missing serializedValue")
        );
        let empty = json!({"boxId": "00", "ergoTree": "10010101d17300", "value": 1});
        assert!(to_box(&empty).unwrap().registers.is_empty());
    }
}
