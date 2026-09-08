//! The drain hunt, phase 1 — "can a transaction that holds no key extract
//! value from these protected boxes, over the shapes an attacker can build?"
//!
//! Design record:
//! `docs/superpowers/specs/2026-09-08-drain-hunt-phase1-design.md`. The spend
//! hunt ([`crate::hunt`]) asks, of one box, whether anyone can spend it. The
//! drain hunt asks, of a labelled contract set plus one fixed transaction
//! shape, whether any arrangement the attacker controls — input order, decoy
//! box contents, free-payee recipients, drained successors — extracts value.
//! The consensus reducer ([`crate::txcheck::check`]) is the only oracle: a
//! hit is a transaction anyone can build and get mined.
//!
//! Phase 1 enumerates a bounded space and does not search:
//!
//! - **input permutation** over the declared `protected`/`companion`/
//!   `attacker` inputs (`external` entries keep their absolute positions, so
//!   the hunt only evaluates shapes the attacker can actually build);
//! - **decoy substitution** from a finite, script-independent family — it
//!   never reads the target script, which is what makes the incident-replay
//!   acceptance test an anti-cheat rather than a tautology;
//! - **payout construction**, verbatim or *drain mode*: successors of
//!   protected boxes drop to a minimal keep-value and one token of each id,
//!   and the first free-payee output receives everything else, with exact
//!   conservation so the probe is a mineable transaction.
//!
//! Two principles from the protocol-map design (`2026-09-08-protocol-map-
//! design.md`) are load-bearing here:
//!
//! - **No binding is unique box identity by default.** An NFT check pins a
//!   token id, not a box — and pins *the box* only when the token's emission
//!   amount is 1, which is the caller's claim when it declares
//!   `protocolNfts`. `propositionBytes ==` pins script bytes only — never
//!   value, tokens or registers.
//! - **Measure leaks by identity and by amount.** A script-matched successor
//!   contributes its *actual* holdings to `protectedOut`, per asset — never
//!   "the right script, so the value stayed". The USE drain's successor had
//!   the right script and the right token ids and 1/1/1 amounts; identity-
//!   only accounting is exactly the hole it walked through.
//!
//! A miss says "not under these probes" — never "safe".

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::scenario::{Scenario, ScenarioBox, TokenAmount};
use crate::txcheck::{check as tx_check, Tx, TxInput, TxRequest};
use crate::SandboxError;

/// What a drained successor keeps. The drained USE pool continuation held
/// exactly this (0.002 ERG); it is a knob, recorded in the report, not a
/// consensus rule.
pub const DRAIN_KEEP_VALUE: i64 = 2_000_000;

// ── Request ──────────────────────────────────────────────────────────────────

/// The role an input plays in the hunted shape. Caller-supplied — typically
/// proposed by the protocol map (`2026-09-08-protocol-map-design.md`) and
/// overridden by hand. `unknown` is refused: the hunt never guesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DrainRole {
    /// Value must not leave. Identity validated against `protocolNfts`.
    Protected,
    /// Keyless box whose script is the authorization (order boxes, trackers).
    Companion,
    /// The attacker's own funds — decoy material.
    Attacker,
    /// Pinned on-chain facts (oracles). Keep position and contents.
    External,
    /// The map could not settle this input; the hunt refuses it.
    Unknown,
}

impl DrainRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            DrainRole::Protected => "protected",
            DrainRole::Companion => "companion",
            DrainRole::Attacker => "attacker",
            DrainRole::External => "external",
            DrainRole::Unknown => "unknown",
        }
    }
}

/// One declared input: a role and its box (flattened, so a protocol map's
/// node record plus a role is a valid entry).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainInput {
    pub role: DrainRole,
    #[serde(flatten)]
    pub box_: ScenarioBox,
}

/// Whether the attacker may re-address (and, in drain mode, re-size) an
/// output. `fixed` outputs are the protocol's own — successors, the preserved
/// order box, the fee. `free` is the payout slot the taker takes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Payee {
    #[default]
    Fixed,
    Free,
}

/// One declared output.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainOutput {
    #[serde(default)]
    pub payee: Payee,
    #[serde(flatten)]
    pub box_: ScenarioBox,
}

/// The drain-hunt request: a labelled contract set and one fixed transaction
/// shape. Roles are caller-supplied and overridable; the protocol map's node
/// roles feed them directly (`unknown` is refused here, as in the map).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainRequest {
    pub inputs: Vec<DrainInput>,
    #[serde(default)]
    pub data_inputs: Vec<ScenarioBox>,
    pub outputs: Vec<DrainOutput>,
    /// Singleton token ids that identify protected boxes. The caller's claim
    /// that these are emission-1 tokens is what turns an id check into a box
    /// binding; the hunt validates presence at `tokens(0)` and nothing more.
    pub protocol_nfts: Vec<String>,
    pub height: u32,
    #[serde(default)]
    pub network: Option<String>,
    /// The attacker's output/input script, hex. Default: `sigmaProp(true)`
    /// compiled by the oracle-pinned compiler.
    #[serde(default)]
    pub attacker_tree: Option<String>,
    #[serde(default)]
    pub max_permutations: Option<usize>,
    #[serde(default)]
    pub max_probes: Option<usize>,
}

// ── Report ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrainVerdict {
    /// Some probe extracted protected value. The report carries the best.
    Drainable,
    /// Nothing extracted under the declared probe space. Not "safe".
    NotUnderProbes,
    /// The request cannot be hunted: shape errors are in `notes`.
    InvalidShape,
}

/// The scenario each protocol input runs in, for `ergo-es eval` replay.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolScenario {
    pub index: usize,
    pub role: DrainRole,
    pub scenario: Scenario,
}

/// A hit's witness: the probe in every representation a validator consumes.
/// `tx_request` is derived mechanically from the probe's boxes (pure
/// conversion), so `ergo-es eval` (via `protocol_scenarios`) and
/// `ergo-es validate-tx` (via `tx_request`) cannot drift apart.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WitnessBundle {
    pub roles: Vec<DrainRole>,
    pub tx_request: TxRequest,
    pub protocol_scenarios: Vec<ProtocolScenario>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainHit {
    /// Per-asset leak, decimal strings: `nanoErg` and token ids.
    pub extracted: BTreeMap<String, String>,
    /// Input positions in realized order, as declared slot indices.
    pub permutation: Vec<usize>,
    /// Decoy labels for attacker slots (`declared` when untouched).
    pub decoys: Vec<String>,
    /// `verbatim` or `drain` payout construction.
    pub payout: &'static str,
    pub witness: WitnessBundle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainReport {
    pub verdict: DrainVerdict,
    /// Probes evaluated (post-deduplication).
    pub probes_run: usize,
    /// Probes generated. `probes_run < probes_total` only under the cap.
    pub probes_total: usize,
    pub capped: bool,
    pub hits: usize,
    /// The hit with the largest total leak.
    pub best: Option<DrainHit>,
    pub notes: Vec<String>,
}

// ── Entry point ──────────────────────────────────────────────────────────────

/// Run the phase-1 drain hunt. Marshalling errors are `Err`; every other
/// outcome — including `invalidShape` — is a report.
pub fn drain_hunt(req: &DrainRequest) -> Result<DrainReport, SandboxError> {
    let mut notes: Vec<String> = Vec::new();

    // ── attacker tree ──
    let net = match req.network.as_deref() {
        Some("testnet") => ergo_ser::address::NetworkPrefix::Testnet,
        _ => ergo_ser::address::NetworkPrefix::Mainnet,
    };
    let attacker_tree = match req.attacker_tree.as_deref().map(str::trim) {
        Some(h) if !h.is_empty() => h.to_string(),
        _ => hex::encode(
            crate::compile::compile_source("sigmaProp(true)", 3, net)
                .map_err(|e| SandboxError::Scenario(format!("attacker tree: {e}")))?
                .tree_bytes,
        ),
    };

    // ── shape validation ──
    let mut shape_errors: Vec<String> = Vec::new();
    if req.inputs.is_empty() {
        shape_errors.push("no inputs declared".into());
    }
    if req.outputs.is_empty() {
        shape_errors.push("no outputs declared".into());
    }
    if req.protocol_nfts.is_empty() {
        shape_errors.push("protocolNfts is empty".into());
    }
    let roles: Vec<DrainRole> = req.inputs.iter().map(|i| i.role).collect();
    let mut declared: Vec<ScenarioBox> = req.inputs.iter().map(|i| i.box_.clone()).collect();
    let nfts: HashSet<String> = req
        .protocol_nfts
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();
    let protected: Vec<usize> = (0..roles.len())
        .filter(|&i| roles[i] == DrainRole::Protected)
        .collect();
    if protected.is_empty() && !req.inputs.is_empty() {
        shape_errors.push("no protected inputs".into());
    }
    for &i in &protected {
        let holds = declared[i]
            .tokens
            .first()
            .map(|t| nfts.contains(&t.id.to_lowercase()))
            .unwrap_or(false);
        if !holds {
            shape_errors.push(format!(
                "protected input {i} does not carry a protocol NFT at tokens(0)"
            ));
        }
        if declared[i]
            .ergo_tree
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            shape_errors.push(format!("input {i} has no ergoTree"));
        }
    }
    for (i, b) in declared.iter_mut().enumerate() {
        if roles[i] == DrainRole::Unknown {
            shape_errors.push(format!("input {i} is `unknown`: the hunt never guesses"));
        }
        if b.ergo_tree
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            // An attacker box with no script is the attacker's own
            // `sigmaProp(true)` box; any other role needs a real tree.
            if roles[i] == DrainRole::Attacker {
                b.ergo_tree = Some(attacker_tree.clone());
            } else {
                shape_errors.push(format!("input {i} has no ergoTree"));
            }
        }
    }
    for (i, o) in req.outputs.iter().enumerate() {
        if o.box_
            .ergo_tree
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            shape_errors.push(format!("output {i} has no ergoTree"));
        }
    }
    if !shape_errors.is_empty() {
        return Ok(DrainReport {
            verdict: DrainVerdict::InvalidShape,
            probes_run: 0,
            probes_total: 0,
            capped: false,
            hits: 0,
            best: None,
            notes: shape_errors,
        });
    }

    // ── probe space ──
    let max_permutations = req.max_permutations.unwrap_or(120).max(1);
    let max_probes = req.max_probes.unwrap_or(20_000).max(1);
    let permutable: Vec<usize> = (0..roles.len())
        .filter(|&i| roles[i] != DrainRole::External)
        .collect();
    let (perms, perms_truncated) = permutations(permutable.len(), max_permutations);
    if perms_truncated {
        notes.push(format!(
            "input permutations capped at {max_permutations} ({} permutable inputs), sampled lexicographically",
            permutable.len()
        ));
    }

    let attacker_slots: Vec<usize> = (0..roles.len())
        .filter(|&i| roles[i] == DrainRole::Attacker)
        .collect();
    let decoys: Vec<Vec<(String, ScenarioBox)>> = (0..roles.len())
        .map(|i| {
            if roles[i] == DrainRole::Attacker {
                decoy_variants(&declared[i], &attacker_tree)
            } else {
                vec![("declared".to_string(), declared[i].clone())]
            }
        })
        .collect();
    let combo_lengths: Vec<usize> = decoys.iter().map(|v| v.len()).collect();

    let free_output = req.outputs.iter().position(|o| o.payee == Payee::Free);
    let protected_trees: HashSet<String> = protected
        .iter()
        .filter_map(|&i| {
            declared[i]
                .ergo_tree
                .as_deref()
                .map(|s| s.trim().to_lowercase())
        })
        .collect();
    let payout_modes: Vec<&'static str> = if free_output.is_some() {
        vec!["verbatim", "drain"]
    } else {
        notes.push("no free-payee output: the drain-mode payout is inexpressible".into());
        vec!["verbatim"]
    };

    // Sanctioned outflow: only what the caller declares through FREE payees.
    // Fixed outputs are the protocol's own and are accounted as protected
    // continuations, never as sanctioned leakage.
    let declared_free: Vec<BTreeMap<String, u128>> = req
        .outputs
        .iter()
        .filter(|o| o.payee == Payee::Free)
        .map(|o| holdings(&o.box_.value, &o.box_.tokens))
        .collect();

    let mut probes_total = 0usize;
    let mut probes_run = 0usize;
    let mut hits = 0usize;
    let mut best: Option<(u128, DrainHit)> = None;
    let mut seen: HashSet<String> = HashSet::new();
    let mut capped = false;

    'perms: for arrangement in &perms {
        for combo_idx in ComboCounter::new(&combo_lengths) {
            let combo: Vec<(String, ScenarioBox)> = combo_idx
                .iter()
                .enumerate()
                .map(|(slot, &vi)| decoys[slot][vi].clone())
                .collect();
            for &payout in &payout_modes {
                probes_total += 1;
                if probes_run >= max_probes {
                    capped = true;
                    break 'perms;
                }
                // Realized input order: external slots keep their positions;
                // permutable positions take the arrangement's slots in order.
                let mut slot_at_position = vec![usize::MAX; roles.len()];
                for (k, &pos) in permutable.iter().enumerate() {
                    slot_at_position[pos] = arrangement[k];
                }
                for &pos in permutable.iter() {
                    if slot_at_position[pos] == usize::MAX {
                        slot_at_position[pos] = pos; // unreachable for full arrangements
                    }
                }
                for i in (0..roles.len()).filter(|&i| roles[i] == DrainRole::External) {
                    slot_at_position[i] = i;
                }
                let position_roles: Vec<DrainRole> =
                    slot_at_position.iter().map(|&slot| roles[slot]).collect();

                let realized_inputs: Vec<ScenarioBox> = (0..roles.len())
                    .map(|pos| {
                        let slot = slot_at_position[pos];
                        combo[slot].1.clone()
                    })
                    .collect();
                let realized_outputs =
                    match realize_outputs(req, &realized_inputs, payout, &attacker_tree) {
                        Some(o) => o,
                        None => {
                            notes.push(
                            "drain-mode payout not constructible (negative remainder); skipping it"
                                .into(),
                        );
                            continue;
                        }
                    };

                // Deduplicate: identical (inputs, outputs) shapes are one probe.
                let key = match serde_json::to_string(&(&realized_inputs, &realized_outputs)) {
                    Ok(k) => k,
                    Err(_) => continue,
                };
                if !seen.insert(key) {
                    continue;
                }
                probes_run += 1;

                // ── oracle: full transaction validation ──
                let probe_seq = probes_run;
                let tx_request =
                    match build_tx_request(req, &realized_inputs, &realized_outputs, probe_seq) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                let check = match tx_check(&tx_request) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if !check.valid {
                    continue;
                }
                let mut disqualified = false;
                for ic in &check.inputs {
                    let pos = ic.index;
                    let accepted = match position_roles[pos] {
                        DrainRole::Protected | DrainRole::Companion | DrainRole::External => {
                            ic.verdict == "pass"
                        }
                        // The attacker signs their own boxes.
                        DrainRole::Attacker => ic.verdict == "pass" || ic.verdict == "needsProof",
                        DrainRole::Unknown => false,
                    };
                    if !accepted {
                        if ic.verdict == "needsProof" {
                            notes.push(format!(
                                "input {pos} ({}) needs a key the attacker does not hold",
                                position_roles[pos].as_str()
                            ));
                        }
                        disqualified = true;
                        break;
                    }
                }
                if disqualified {
                    continue;
                }

                // ── objective: leak by identity and amount ──
                let extracted = leak(
                    &position_roles,
                    &realized_inputs,
                    &realized_outputs,
                    &protected_trees,
                    &declared_free,
                );
                if extracted.is_empty() {
                    continue;
                }
                hits += 1;
                let total: u128 = extracted.values().sum();
                if best.as_ref().map(|(t, _)| total > *t).unwrap_or(true) {
                    let decoy_labels: Vec<String> = attacker_slots
                        .iter()
                        .map(|&s| format!("input {s}: {}", combo[s].0))
                        .collect();
                    best = Some((
                        total,
                        DrainHit {
                            extracted: extracted
                                .into_iter()
                                .map(|(k, v)| (k, v.to_string()))
                                .collect(),
                            permutation: permutable
                                .iter()
                                .map(|&pos| slot_at_position[pos])
                                .collect(),
                            decoys: decoy_labels,
                            payout,
                            witness: witness_bundle(
                                req,
                                &position_roles,
                                &realized_inputs,
                                &realized_outputs,
                                &tx_request,
                            ),
                        },
                    ));
                }
            }
        }
    }

    if capped {
        notes.push(format!(
            "probe cap {max_probes} reached; the space was truncated"
        ));
    }
    notes.dedup();
    let verdict = if hits > 0 {
        DrainVerdict::Drainable
    } else {
        DrainVerdict::NotUnderProbes
    };
    Ok(DrainReport {
        verdict,
        probes_run,
        probes_total,
        capped,
        hits,
        best: best.map(|(_, h)| h),
        notes,
    })
}

// ── Objective ────────────────────────────────────────────────────────────────

/// Per-asset holdings of a box: `nanoErg` plus one entry per token id.
fn holdings(value: &i64, tokens: &[TokenAmount]) -> BTreeMap<String, u128> {
    let mut m = BTreeMap::new();
    if *value > 0 {
        m.insert("nanoErg".to_string(), *value as u128);
    }
    for t in tokens {
        *m.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
    }
    m
}

/// The leak: protected inputs' holdings minus what script-matched successor
/// outputs actually hold, minus what the template sanctions through free
/// payees (their declared amounts). Assets with no leak are omitted.
///
/// Successors are outputs whose script bytes match a protected input's —
/// `propositionBytes ==` pins the script only, so their *actual* holdings are
/// counted, per asset. The drained USE successor (right script, right token
/// ids, 1/1/1 amounts) therefore shields 0.002 ERG and one of each token, and
/// nothing more.
fn leak(
    roles: &[DrainRole],
    realized_inputs: &[ScenarioBox],
    realized_outputs: &[ScenarioBox],
    protected_trees: &HashSet<String>,
    free_declared: &[BTreeMap<String, u128>],
) -> BTreeMap<String, u128> {
    let mut protected_in: BTreeMap<String, u128> = BTreeMap::new();
    for (i, b) in realized_inputs.iter().enumerate() {
        if roles[i] != DrainRole::Protected {
            continue;
        }
        for (k, v) in holdings(&b.value, &b.tokens) {
            *protected_in.entry(k).or_default() += v;
        }
    }
    let mut protected_out: BTreeMap<String, u128> = BTreeMap::new();
    for b in realized_outputs {
        let pinned = b
            .ergo_tree
            .as_deref()
            .map(|s| protected_trees.contains(&s.trim().to_lowercase()))
            .unwrap_or(false);
        if !pinned {
            continue;
        }
        for (k, v) in holdings(&b.value, &b.tokens) {
            *protected_out.entry(k).or_default() += v;
        }
    }
    let mut free: BTreeMap<String, u128> = BTreeMap::new();
    for m in free_declared {
        for (k, v) in m {
            *free.entry(k.clone()).or_default() += *v;
        }
    }
    let mut out = BTreeMap::new();
    for (asset, in_amt) in &protected_in {
        let held = protected_out.get(asset).copied().unwrap_or(0);
        let sanctioned = free.get(asset).copied().unwrap_or(0);
        let leak = in_amt.saturating_sub(held).saturating_sub(sanctioned);
        if leak > 0 {
            out.insert(asset.clone(), leak);
        }
    }
    out
}

// ── Payout construction ──────────────────────────────────────────────────────

/// Realize the template's outputs. `verbatim` keeps every declared box.
/// `drain` drops each script-matched successor to [`DRAIN_KEEP_VALUE`] and
/// one token per declared id, and gives the first free-payee output the
/// attacker's tree and everything else — exact conservation by construction.
/// Returns `None` when the drain shape cannot balance (the caller's declared
/// outputs already exceed the inputs).
fn realize_outputs(
    req: &DrainRequest,
    realized_inputs: &[ScenarioBox],
    payout: &str,
    attacker_tree: &str,
) -> Option<Vec<ScenarioBox>> {
    if payout == "verbatim" {
        return Some(req.outputs.iter().map(|o| o.box_.clone()).collect());
    }
    let free = req.outputs.iter().position(|o| o.payee == Payee::Free)?;
    let protected_trees: HashSet<String> = req
        .inputs
        .iter()
        .filter(|i| i.role == DrainRole::Protected)
        .filter_map(|i| i.box_.ergo_tree.as_deref().map(|s| s.trim().to_lowercase()))
        .collect();

    let mut out: Vec<ScenarioBox> = Vec::with_capacity(req.outputs.len());
    for (i, o) in req.outputs.iter().enumerate() {
        let mut b = o.box_.clone();
        let is_successor = b
            .ergo_tree
            .as_deref()
            .map(|s| protected_trees.contains(&s.trim().to_lowercase()))
            .unwrap_or(false);
        if is_successor {
            b.value = b.value.min(DRAIN_KEEP_VALUE);
            for t in &mut b.tokens {
                t.amount = 1;
            }
        }
        if i == free {
            b.ergo_tree = Some(attacker_tree.to_string());
        }
        out.push(b);
    }

    // The free output takes the remainder: everything the inputs carry that
    // the other outputs do not.
    let erg_out: i128 = out
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != free)
        .map(|(_, b)| b.value as i128)
        .sum();
    let erg_in: i128 = realized_inputs.iter().map(|b| b.value as i128).sum();
    let free_value = erg_in - erg_out;
    if free_value < 0 || free_value > i64::MAX as i128 {
        return None;
    }
    out[free].value = free_value as i64;

    let mut tok_out: BTreeMap<String, u128> = BTreeMap::new();
    for (i, b) in out.iter().enumerate() {
        if i == free {
            continue;
        }
        for t in &b.tokens {
            *tok_out.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    let mut tok_in: BTreeMap<String, u128> = BTreeMap::new();
    for b in realized_inputs {
        for t in &b.tokens {
            *tok_in.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    let mut free_tokens: Vec<TokenAmount> = Vec::new();
    for (id, in_amt) in &tok_in {
        let remainder = in_amt.saturating_sub(tok_out.get(id).copied().unwrap_or(0));
        if remainder > 0 {
            free_tokens.push(TokenAmount {
                id: id.clone(),
                amount: u64::try_from(remainder).ok()?,
            });
        }
    }
    if free_tokens.is_empty() && !out[free].tokens.is_empty() {
        // The template's free output declared a mint (an id the inputs do not
        // carry); drain mode keeps it only if conservation still allows, which
        // `txcheck` judges. Keep declared tokens when there is no remainder.
        free_tokens = out[free].tokens.clone();
    }
    out[free].tokens = free_tokens;
    Some(out)
}

// ── Decoys ───────────────────────────────────────────────────────────────────

/// Dummy token id `i`: 31 zero bytes plus `i`. Distinct ids per index, so a
/// single box never carries a duplicate token id.
fn dummy_token_id(i: u8) -> String {
    let mut bytes = [0u8; 32];
    bytes[31] = i;
    hex::encode(bytes)
}

/// The generic decoy family for one attacker slot: finite, deterministic, and
/// script-independent — it never reads the tree under hunt. First variant is
/// the declared box itself.
fn decoy_variants(base: &ScenarioBox, attacker_tree: &str) -> Vec<(String, ScenarioBox)> {
    let mut out: Vec<(String, ScenarioBox)> = Vec::new();
    // Every variant — the declared box included — carries a script: a
    // treeless attacker box is the attacker's own `sigmaProp(true)` box.
    let push = |label: String, b: ScenarioBox, out: &mut Vec<(String, ScenarioBox)>| {
        let mut b = b;
        let empty = b
            .ergo_tree
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none();
        if empty {
            b.ergo_tree = Some(attacker_tree.to_string());
        }
        out.push((label, b));
    };
    push("declared".to_string(), base.clone(), &mut out);
    // Value variants.
    for k in [0i64, 10i64] {
        let mut b = base.clone();
        b.value = base.value.saturating_mul(k);
        push(format!("value×{k}"), b, &mut out);
    }
    // Token variants: filler tokens at 0..=i, with distinct dummy ids.
    for i in 0..=3u8 {
        let mut b = base.clone();
        b.tokens = (0..=i)
            .map(|j| TokenAmount {
                id: dummy_token_id(j),
                amount: 1,
            })
            .collect();
        push(format!("filler tokens 0..={i}"), b, &mut out);
    }
    // Register variant.
    let mut b = base.clone();
    b.registers = Default::default();
    push("registers cleared".to_string(), b, &mut out);
    out
}

/// Mixed-radix counter over per-slot variant list lengths; the first
/// combination is every slot's first variant (all declared), the last slot's
/// variant increments fastest. Deterministic.
struct ComboCounter {
    lengths: Vec<usize>,
    indices: Vec<usize>,
    done: bool,
}

impl ComboCounter {
    fn new(lengths: &[usize]) -> Self {
        ComboCounter {
            lengths: lengths.to_vec(),
            indices: vec![0; lengths.len()],
            done: lengths.contains(&0),
        }
    }
}

impl Iterator for ComboCounter {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let current = self.indices.clone();
        for i in (0..self.indices.len()).rev() {
            self.indices[i] += 1;
            if self.indices[i] < self.lengths[i] {
                return Some(current);
            }
            self.indices[i] = 0;
        }
        self.done = true;
        Some(current)
    }
}

// ── Permutations ─────────────────────────────────────────────────────────────

/// Lexicographic arrangements of `n` slots, capped at `max`. The first is the
/// identity (declared order). Deterministic; `truncated` reports a cap hit.
fn permutations(n: usize, max: usize) -> (Vec<Vec<usize>>, bool) {
    if n == 0 {
        return (vec![vec![]], false);
    }
    let total: u128 = (1..=n as u128).product();
    let mut out = Vec::new();
    let mut cur: Vec<usize> = (0..n).collect();
    loop {
        out.push(cur.clone());
        if out.len() >= max || out.len() as u128 >= total {
            break;
        }
        // Next lexicographic permutation of `cur`.
        let mut i = n - 1;
        while i > 0 && cur[i - 1] >= cur[i] {
            i -= 1;
        }
        if i == 0 {
            break;
        }
        let mut j = n - 1;
        while cur[j] <= cur[i - 1] {
            j -= 1;
        }
        cur.swap(i - 1, j);
        cur[i..].reverse();
    }
    let truncated = out.len() < total as usize;
    (out, truncated)
}

// ── Marshalling ──────────────────────────────────────────────────────────────

/// A realized `ScenarioBox` as node/explorer box JSON, with a deterministic
/// box id when the scenario does not name one.
fn box_json(sb: &ScenarioBox, id_seed: &str) -> Result<serde_json::Value, SandboxError> {
    let mut registers = serde_json::Map::new();
    for (k, tv) in &sb.registers {
        if tv.r#type != "raw" {
            return Err(SandboxError::Scenario(format!(
                "register {k} is not `raw`; declare boxes for the drain hunt with raw register hex"
            )));
        }
        let hex_val = tv.value.as_str().ok_or_else(|| {
            SandboxError::Scenario(format!("register {k} raw value is not a string"))
        })?;
        registers.insert(k.clone(), serde_json::Value::String(hex_val.to_string()));
    }
    let id: String = match &sb.box_id {
        Some(s) => s.clone(),
        None => {
            let digest = ergo_primitives::digest::blake2b256(id_seed.as_bytes());
            hex::encode(digest.as_bytes())
        }
    };
    Ok(json!({
        "boxId": id,
        "value": sb.value,
        "ergoTree": sb.ergo_tree.clone().unwrap_or_default(),
        "assets": sb
            .tokens
            .iter()
            .map(|t| json!({"tokenId": t.id, "amount": t.amount}))
            .collect::<Vec<_>>(),
        "additionalRegisters": registers,
        "creationHeight": sb.creation_height,
    }))
}

fn build_tx_request(
    req: &DrainRequest,
    realized_inputs: &[ScenarioBox],
    realized_outputs: &[ScenarioBox],
    probe_seq: usize,
) -> Result<TxRequest, SandboxError> {
    let mut boxes = Vec::with_capacity(realized_inputs.len() + req.data_inputs.len());
    let mut tx_inputs = Vec::with_capacity(realized_inputs.len());
    for (i, b) in realized_inputs.iter().enumerate() {
        let bj = box_json(b, &format!("drain|{probe_seq}|in|{i}"))?;
        let id = bj["boxId"].as_str().unwrap_or_default().to_string();
        tx_inputs.push(TxInput {
            box_id: id,
            extension: Default::default(),
        });
        boxes.push(bj);
    }
    let mut data_inputs = Vec::with_capacity(req.data_inputs.len());
    for (i, b) in req.data_inputs.iter().enumerate() {
        let bj = box_json(b, &format!("drain|{probe_seq}|data|{i}"))?;
        let id = bj["boxId"].as_str().unwrap_or_default().to_string();
        data_inputs.push(TxInput {
            box_id: id,
            extension: Default::default(),
        });
        boxes.push(bj);
    }
    let mut outputs = Vec::with_capacity(realized_outputs.len());
    for b in realized_outputs {
        outputs.push(box_json(b, "unused")?);
    }
    Ok(TxRequest {
        tx: Tx {
            inputs: tx_inputs,
            data_inputs,
            outputs,
        },
        boxes,
        height: Some(req.height),
        network: req.network.clone(),
    })
}

fn witness_bundle(
    req: &DrainRequest,
    roles: &[DrainRole],
    realized_inputs: &[ScenarioBox],
    realized_outputs: &[ScenarioBox],
    tx_request: &TxRequest,
) -> WitnessBundle {
    let mut protocol_scenarios = Vec::new();
    for (i, b) in realized_inputs.iter().enumerate() {
        if roles[i] != DrainRole::Protected && roles[i] != DrainRole::Companion {
            continue;
        }
        let scenario = Scenario {
            params: Default::default(),
            headers: Vec::new(),
            secrets: Vec::new(),
            parties: Vec::new(),
            avl: Default::default(),
            tree: b.ergo_tree.clone(),
            source: None,
            tree_version: 0,
            network: req.network.clone(),
            height: req.height,
            self_box: None,
            self_index: Some(i),
            inputs: realized_inputs.to_vec(),
            outputs: realized_outputs.to_vec(),
            data_inputs: req.data_inputs.clone(),
            context_vars: Default::default(),
            miner_pubkey: None,
            pre_header: None,
            cost_limit: None,
            activated_script_version: None,
            proof: None,
            message: None,
        };
        protocol_scenarios.push(ProtocolScenario {
            index: i,
            role: roles[i],
            scenario,
        });
    }
    WitnessBundle {
        roles: roles.to_vec(),
        tx_request: tx_request.clone(),
        protocol_scenarios,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permutations_are_lexicographic_and_capped() {
        let (perms, truncated) = permutations(3, 120);
        assert_eq!(perms.len(), 6);
        assert!(!truncated);
        assert_eq!(
            perms[0],
            vec![0usize, 1, 2],
            "the declared order comes first"
        );
        assert_eq!(perms.last().unwrap(), &vec![2usize, 1, 0]);

        let (perms, truncated) = permutations(3, 2);
        assert_eq!(perms.len(), 2);
        assert!(truncated);

        let (perms, truncated) = permutations(0, 10);
        assert_eq!(perms, vec![Vec::<usize>::new()]);
        assert!(!truncated);
    }

    #[test]
    fn the_decoy_family_is_finite_script_independent_and_includes_the_incident_shape() {
        let base = ScenarioBox {
            value: 2_000_000,
            ergo_tree: None, // filled with the attacker tree
            tokens: vec![],
            ..Default::default()
        };
        let variants = decoy_variants(&base, "deadbeef");
        let labels: Vec<&str> = variants.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels.len(), 8, "finite: {labels:?}");
        assert_eq!(labels[0], "declared");
        assert!(
            labels.contains(&"filler tokens 0..=2"),
            "the incident decoy: {labels:?}"
        );
        // The family never reads the target script: every variant derives
        // from the declared box and fixed constants alone. The treeless
        // declared box carries the attacker tree, nothing else does.
        for (_, b) in &variants {
            assert_eq!(b.ergo_tree.as_deref(), Some("deadbeef"));
        }
    }

    #[test]
    fn the_combo_counter_walks_every_combination_exactly_once() {
        let counter = ComboCounter::new(&[2, 1, 3]);
        let seen: Vec<Vec<usize>> = counter.collect();
        assert_eq!(seen.len(), 6);
        assert_eq!(seen[0], vec![0, 0, 0]);
        assert_eq!(seen.last().unwrap(), &vec![1usize, 0, 2]);
        let unique: HashSet<_> = seen.iter().collect();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn empty_combinations_terminate() {
        let mut counter = ComboCounter::new(&[0]);
        assert!(counter.next().is_none());
    }
}
