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
fn to_box(v: &Value) -> Option<ChainBox> {
    Some(ChainBox {
        box_id: v.get("boxId")?.as_str()?.to_ascii_lowercase(),
        ergo_tree: v.get("ergoTree")?.as_str()?.to_ascii_lowercase(),
        value: v.get("value").and_then(Value::as_u64).unwrap_or(0),
        tokens: v
            .get("assets")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|t| {
                        Some(ChainToken {
                            id: t.get("tokenId")?.as_str()?.to_ascii_lowercase(),
                            amount: t.get("amount").and_then(Value::as_u64).unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        creation_height: v.get("creationHeight").and_then(Value::as_u64).unwrap_or(0) as u32,
        // `settlementHeight` is the height the box entered the chain at — the
        // stable half of the map's canonical selection order.
        inclusion_height: v
            .get("settlementHeight")
            .or_else(|| v.get("inclusionHeight"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
    })
}

fn to_page(v: &Value) -> Page {
    Page {
        items: v
            .get("items")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(to_box).collect())
            .unwrap_or_default(),
        total: v.get("total").and_then(Value::as_u64).map(|t| t as usize),
    }
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
        to_box(&v).ok_or_else(|| SourceError::Backend(format!("box {box_id}: unusable JSON")))
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
        Ok(to_page(&v))
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
        Ok(to_page(&v))
    }

    // `boxes_by_script_hash` is deliberately left at the trait default. The
    // explorer indexes an ErgoTree *template* hash, which is not
    // `blake2b256(propositionBytes)`: answering with it would silently return
    // the wrong boxes. `Unsupported` is the honest answer, and the map turns
    // it into an unresolved edge rather than an absence.

    fn transaction(&self, tx_id: &str) -> Result<TxBoxes, SourceError> {
        let v = self.get(&format!("/api/v1/transactions/{tx_id}"))?;
        let side = |k: &str| {
            v.get(k)
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(to_box).collect::<Vec<_>>())
                .unwrap_or_default()
        };
        Ok(TxBoxes {
            inputs: side("inputs"),
            outputs: side("outputs"),
        })
    }
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
