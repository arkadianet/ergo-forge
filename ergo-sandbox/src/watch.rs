//! Bounded observations of a source response. Callers retain baselines; no
//! background polling or server state. Script comparison belongs to lockfile.
use crate::{
    identity::LIMITATION,
    lockfile::{self, LiveComparison, LiveStatus, Lockfile},
    map::source::{ChainBox, ChainSource, Page, SourceError, TokenInfo, TxBoxes},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap};

pub const NO_EXPLORER: &str = "no explorer configured (explorer-dependency)";
pub const OBSERVATION: &str = "Observation of the source's response; chain membership and currentness are not independently verified. Nothing is signed or broadcast.";
pub const MAX_REPORTS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchInput {
    pub lockfile: Lockfile,
    pub nfts: Vec<String>,
    #[serde(default)]
    pub registers: Vec<String>,
    /// Caller-supplied register references, keyed by NFT. These are supplied
    /// data, not attestations that the baseline belongs to a chain or NFT.
    #[serde(default)]
    pub baseline_boxes: BTreeMap<String, ChainBox>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceInfo {
    pub kind: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegisterStatus {
    Unchanged,
    Changed,
    Unverified,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterObservation {
    pub name: String,
    /// Raw serialized constant hex, exactly as returned (no interpretation).
    pub current: Option<String>,
    pub expected: Option<String>,
    pub status: RegisterStatus,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchReport {
    pub nft: String,
    pub lockfile_fingerprint: String,
    pub chain_source: SourceInfo,
    /// Source height read immediately before this holder lookup. Queries are
    /// not an atomic snapshot. This is not the box's creation/inclusion height.
    pub height: Option<u32>,
    pub live: LiveComparison,
    pub registers: Vec<RegisterObservation>,
    /// Re-submit this box to compare later observations to the same baseline.
    pub baseline_box: Option<ChainBox>,
    pub baseline_origin: &'static str,
    pub limitation: &'static str,
    pub observation: &'static str,
}

/// SHA-256 of the compact serde JSON serialization of schema-1 Lockfile.
/// Struct field order and ordered parameter maps make formatting irrelevant.
pub fn fingerprint(lock: &Lockfile) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(lock).expect("lock serializes"),
    ))
}

fn valid_hex(value: &str) -> bool {
    !value.is_empty() && hex::decode(value).is_ok()
}

/// Input errors fail before any source call. Caps reject rather than truncate.
pub fn validate(inputs: &[WatchInput]) -> Result<(), String> {
    let count: usize = inputs.iter().map(|i| i.nfts.len()).sum();
    if inputs.is_empty() || count == 0 || count > MAX_REPORTS {
        return Err(format!("watch requires 1–{MAX_REPORTS} lockfile/NFT pairs"));
    }
    for input in inputs {
        if input.lockfile.schema_version != 1 || !valid_hex(&input.lockfile.tree_hex) {
            return Err("watch requires a schema-1 lockfile with nonempty treeHex bytes".into());
        }
        if input.nfts.is_empty() {
            return Err("each lockfile requires at least one NFT".into());
        }
        let mut nfts = std::collections::BTreeSet::new();
        for nft in &input.nfts {
            if nft.len() != 64 || !valid_hex(nft) || !nfts.insert(nft.to_ascii_lowercase()) {
                return Err("NFTs must be unique 32-byte hex IDs per lockfile".into());
            }
        }
        let mut registers = std::collections::BTreeSet::new();
        for register in &input.registers {
            if !matches!(register.as_str(), "R4" | "R5" | "R6" | "R7" | "R8" | "R9")
                || !registers.insert(register)
            {
                return Err("watched registers must be unique names R4–R9".into());
            }
        }
        let mut baselines = std::collections::BTreeSet::new();
        for nft in input.baseline_boxes.keys() {
            let nft = nft.to_ascii_lowercase();
            if !nfts.contains(&nft) || !baselines.insert(nft) {
                return Err("baselineBoxes keys must identify distinct requested NFTs".into());
            }
        }
    }
    Ok(())
}

fn report(
    input: &WatchInput,
    nft: &str,
    source: SourceInfo,
    height: Option<u32>,
    live: LiveComparison,
    holder: Option<ChainBox>,
) -> WatchReport {
    let supplied = input
        .baseline_boxes
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(nft))
        .map(|(_, b)| b);
    let baseline = supplied.or(holder.as_ref());
    let registers: Vec<_> = input.registers.iter().map(|name| {
        let current = holder.as_ref().and_then(|b| b.registers.get(name)).cloned();
        let expected = baseline.and_then(|b| b.registers.get(name)).cloned();
        let (status, reason) = match (current.as_deref(), expected.as_deref()) {
            (Some(a), Some(b)) if valid_hex(a) && valid_hex(b) => {
                let changed = !a.eq_ignore_ascii_case(b);
                (if changed { RegisterStatus::Changed } else { RegisterStatus::Unchanged },
                 if supplied.is_none() { "First observation establishes the baseline; no earlier value was supplied.".into() }
                 else { "Compared serialized register bytes against the supplied baseline.".into() })
            }
            _ => (RegisterStatus::Unverified, if holder.is_none() { live.reason.clone() }
                  else { "Watched register missing or invalid hex in observation or baseline; absence cannot be distinguished from omitted source data.".into() }),
        };
        RegisterObservation { name: name.clone(), current, expected, status, reason }
    }).collect();
    let baseline_box = supplied.cloned().or_else(|| {
        // Do not establish a partial first baseline that could silently reset
        // one missing register on a later observation.
        (!registers.is_empty()
            && registers
                .iter()
                .all(|r| r.status != RegisterStatus::Unverified))
        .then(|| holder.clone())
        .flatten()
    });
    WatchReport {
        nft: nft.to_ascii_lowercase(),
        lockfile_fingerprint: fingerprint(&input.lockfile),
        chain_source: source,
        height,
        live,
        registers,
        baseline_box,
        baseline_origin: if supplied.is_some() {
            "supplied"
        } else if holder.is_some() {
            "first_observation"
        } else {
            "unavailable"
        },
        limitation: LIMITATION,
        observation: OBSERVATION,
    }
}

/// A configured adapter may be unavailable (for example in a featureless CLI).
/// Preserve its identity and the reason, with every observation unverified.
pub fn unavailable(
    inputs: &[WatchInput],
    source: SourceInfo,
    reason: &str,
) -> Result<Vec<WatchReport>, String> {
    validate(inputs)?;
    Ok(inputs
        .iter()
        .flat_map(|input| {
            input.nfts.iter().map(|nft| {
                report(
                    input,
                    nft,
                    source.clone(),
                    None,
                    LiveComparison::unverified(reason),
                    None,
                )
            })
        })
        .collect())
}

/// Exactly one height read and at most one token-info read and one holder page
/// (offset 0, limit 2) per report. `compare_live` owns all holder validation.
pub fn observe(
    inputs: &[WatchInput],
    source: Option<&dyn ChainSource>,
) -> Result<Vec<WatchReport>, String> {
    validate(inputs)?;
    let Some(source) = source else {
        return unavailable(
            inputs,
            SourceInfo {
                kind: "none".into(),
                url: None,
            },
            NO_EXPLORER,
        );
    };
    let mut reports = Vec::new();
    for input in inputs {
        for nft in &input.nfts {
            let info = SourceInfo {
                kind: source.kind().into(),
                url: source.url().map(str::to_string),
            };
            let height = match source.height() {
                Ok(height) => height,
                Err(e) => {
                    reports.push(report(
                        input,
                        nft,
                        info,
                        None,
                        LiveComparison::unverified(format!("Source height unavailable: {e}")),
                        None,
                    ));
                    continue;
                }
            };
            let capture = HolderResponse {
                source,
                holder: RefCell::new(None),
            };
            let live = lockfile::compare_live(&input.lockfile, Some(nft), Some(&capture));
            // Use the exact response that compare_live checked; never refetch a
            // possibly different box or fork singleton/byte matching logic.
            let holder = if live.status == LiveStatus::Unverified {
                None
            } else {
                capture.holder.into_inner()
            };
            reports.push(report(input, nft, info, Some(height), live, holder));
        }
    }
    Ok(reports)
}

/// Incomplete observations take precedence over change (4 before 3). All rows
/// remain in the output so a mixed batch cannot conceal an observed change.
pub fn exit_code(reports: &[WatchReport]) -> u8 {
    if reports.is_empty()
        || reports.iter().any(|r| {
            r.live.status == LiveStatus::Unverified
                || r.registers
                    .iter()
                    .any(|v| v.status == RegisterStatus::Unverified)
        })
    {
        4
    } else if reports.iter().any(|r| {
        r.live.status == LiveStatus::BytesDiffer
            || r.registers
                .iter()
                .any(|v| v.status == RegisterStatus::Changed)
    }) {
        3
    } else {
        0
    }
}

struct HolderResponse<'a> {
    source: &'a dyn ChainSource,
    holder: RefCell<Option<ChainBox>>,
}
impl ChainSource for HolderResponse<'_> {
    fn kind(&self) -> &str {
        self.source.kind()
    }
    fn height(&self) -> Result<u32, SourceError> {
        self.source.height()
    }
    fn token_info(&self, id: &str) -> Result<TokenInfo, SourceError> {
        self.source.token_info(id)
    }
    fn boxes_by_token_id(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        let page = self.source.boxes_by_token_id(id, offset, limit)?;
        if page.items.len() == 1 {
            *self.holder.borrow_mut() = page.items.first().cloned();
        }
        Ok(page)
    }
    fn box_by_id(&self, _: &str) -> Result<ChainBox, SourceError> {
        Err(SourceError::Unsupported("watch box by id"))
    }
    fn boxes_by_address(&self, _: &str, _: usize, _: usize) -> Result<Page, SourceError> {
        Err(SourceError::Unsupported("watch address lookup"))
    }
    fn transaction(&self, _: &str) -> Result<TxBoxes, SourceError> {
        Err(SourceError::Unsupported("watch transaction lookup"))
    }
}
