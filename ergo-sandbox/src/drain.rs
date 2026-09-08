//! The drain hunt — "can a transaction that holds no key extract value from
//! these protected boxes, over the shapes an attacker can build?"
//!
//! Design records: phase 1
//! (`docs/superpowers/specs/2026-09-08-drain-hunt-phase1-design.md`, shipped
//! in #64), phase 2 — output synthesis
//! (`docs/superpowers/specs/2026-09-08-drain-hunt-phase2-design.md`, merged in
//! #65). The spend hunt ([`crate::hunt`]) asks, of one box, whether anyone can
//! spend it. The drain hunt asks, of a labelled contract set plus one fixed
//! transaction shape, whether any arrangement the attacker controls — input
//! order, decoy box contents, free-payee recipients, drained successors, and
//! (phase 2) invented outputs — extracts value. The consensus reducer
//! ([`crate::txcheck::check`]) is the only oracle: a hit is a transaction
//! anyone can build and get mined.
//!
//! The hunt enumerates a bounded space and does not search:
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
//!   conservation so the probe is a mineable transaction;
//! - **output synthesis** (phase 2, opt-in via the request's `synthesis`
//!   block): companion re-creations with padded token layouts (fillers
//!   *sourced* from attacker inputs — never conjured), new attacker sinks,
//!   per-successor `{verbatim, minimized}` states, one deterministic value
//!   split, minted placeholders sized from the declared request alone, and
//!   output permutation. With the block absent or all-off the hunt is
//!   byte-identical to phase 1.
//!
//! The probe axes have a **pinned order**, outermost first: synthesized-output
//! shapes → output permutation → per-successor states → value splits → mint
//! variants → input permutation + decoy combinations. Truncation is the
//! normal case; the pinned order is what decides which probes exist at all,
//! so it is recorded in the report next to the caps.
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
//!   only accounting is exactly the hole it walked through. With synthesis
//!   on, script bytes alone shield nothing: a script-matched output counts as
//!   protected only when the protected input's protocol NFT rides along
//!   (same id, same token index); every script-matched NFT-less output is
//!   reported as its own `nftDetached` signal.
//!
//! A miss says "not under these probes" — never "safe". With synthesis on,
//! every synthesized probe's rejection is classified (`conservation` /
//! `missingKey` / `script`) and tallied, so a miss can never silently mean
//! "conservation blocked us".

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

/// Default cap on synthesized outputs when the `synthesis` block is active
/// but `maxNewOutputs` is omitted (Decision 1 of the phase-2 spec).
pub const DEFAULT_MAX_NEW_OUTPUTS: usize = 2;

/// Default cap on the per-successor `{verbatim, minimized}` combinations
/// (`2^s` capped).
pub const DEFAULT_MAX_SUCCESSOR_STATES: usize = 8;

/// Default cap on output permutations.
pub const DEFAULT_OUTPUT_PERMUTATIONS: usize = 24;

/// Placeholder token id for a mint probe's synthesized token. Resolved to
/// the first input's box id before the oracle runs (`txcheck` allows exactly
/// one minted id — the first input's box id). Deliberately not hex: it can
/// never collide with a real or dummy token id, and a bug that leaks it into
/// a probe fails conservation loudly instead of passing silently.
pub const MINT_SENTINEL: &str = "*mint-first-input-box-id*";

/// The pinned probe-axis order, outermost first (phase-2 spec, Decision 1).
/// Recorded in every report.
pub const AXIS_ORDER: [&str; 7] = [
    "synthesized-output shapes",
    "output permutation",
    "per-successor states",
    "value splits",
    "mint variants",
    "input permutation + decoy combinations",
    "filler counts (re-creation shapes, innermost)",
];

// ── Phase-2 synthesis request ────────────────────────────────────────────────

/// The output-synthesis degrees of freedom (phase-2 spec, Decision 1). Every
/// field defaults off: an omitted `synthesis` block and an all-off block are
/// byte-identical to phase 1.
///
/// `maxNewOutputs` caps the total number of synthesized outputs. When the
/// block is active (any degree on) but the cap is omitted, the spec's default
/// of 2 applies; when the block is absent or all-off, the resolved cap is 0
/// and no shape beyond `none` is generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Synthesis {
    /// Cap on synthesized outputs beyond the template. `None` = unspecified:
    /// [`DEFAULT_MAX_NEW_OUTPUTS`] when any degree is on, else 0.
    #[serde(default)]
    pub max_new_outputs: Option<usize>,
    /// Companion re-creations with padded token layouts (fillers sourced
    /// from attacker inputs — never conjured).
    #[serde(default)]
    pub companion_recreations: bool,
    /// Each script-matched successor independently `{verbatim, minimized}`
    /// (`2^s` combinations, capped at `max_successor_states`).
    #[serde(default)]
    pub successor_states: bool,
    /// Cap on per-successor state combinations.
    #[serde(default = "default_max_successor_states")]
    pub max_successor_states: usize,
    /// The first deterministic value split (`⌈half⌉`/`⌊half⌋` across two
    /// sinks); with two sinks this is the only value distribution.
    #[serde(default)]
    pub splits: bool,
    /// Mint probes: one synthesized output may mint the first input's box
    /// id, in amounts drawn from the declared request alone.
    #[serde(default)]
    pub mints: bool,
    /// Cap on output permutations.
    #[serde(default = "default_output_permutations")]
    pub max_output_permutations: usize,
    /// Outputs join the permutation domain (synthesized boxes must be able
    /// to land at pinned indices like `OUTPUTS(0)`).
    #[serde(default)]
    pub permute_outputs: bool,
}

fn default_max_successor_states() -> usize {
    DEFAULT_MAX_SUCCESSOR_STATES
}

fn default_output_permutations() -> usize {
    DEFAULT_OUTPUT_PERMUTATIONS
}

impl Default for Synthesis {
    fn default() -> Self {
        Synthesis {
            max_new_outputs: None,
            companion_recreations: false,
            successor_states: false,
            max_successor_states: DEFAULT_MAX_SUCCESSOR_STATES,
            splits: false,
            mints: false,
            max_output_permutations: DEFAULT_OUTPUT_PERMUTATIONS,
            permute_outputs: false,
        }
    }
}

impl Synthesis {
    /// Whether any synthesis degree is on. An inactive block is byte-identical
    /// to phase 1.
    pub fn active(&self) -> bool {
        self.companion_recreations
            || self.successor_states
            || self.splits
            || self.mints
            || self.permute_outputs
    }

    /// The resolved synthesized-output cap: explicit, or the spec default
    /// when the block is active, else 0 (nothing is synthesized). An
    /// inactive block resolves to 0 even with an explicit cap — a cap is
    /// not a degree, and only degrees turn shapes on.
    pub fn resolved_max_new_outputs(&self) -> usize {
        if !self.active() {
            return 0;
        }
        self.max_new_outputs.unwrap_or(DEFAULT_MAX_NEW_OUTPUTS)
    }
}

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
    /// Output-synthesis degrees. Absent = phase 1, byte-identical.
    #[serde(default)]
    pub synthesis: Synthesis,
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
    /// The synthesized-output shape that produced the hit (`none` for
    /// template shapes).
    pub shape: String,
    pub witness: WitnessBundle,
}

/// One shape family's traversal tally (phase-2 report).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeTally {
    pub shape: String,
    /// Probes the generator materialized (post filler-sourcing, pre dedup).
    pub generated: usize,
    /// Probes executed against the oracle (post dedup).
    pub run: usize,
    /// This shape's slice of the total-probe cap (the allocator's ceiling
    /// minus the previous shape's). With synthesis off, the single shape's
    /// budget is the whole cap.
    pub budget: usize,
}

/// Why synthesized probes were rejected (phase-2 report). A `notUnderProbes`
/// on a synthesized hunt must never silently mean "conservation blocked us".
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejections {
    /// The oracle refused the transaction on ERG/token conservation.
    pub conservation: usize,
    /// A `protected`/`companion` input needed a key the attacker does not
    /// hold (`needsProof`).
    pub missing_key: usize,
    /// A protocol script evaluated to false or threw.
    pub script: usize,
}

/// A script-matched output whose protected input's protocol NFT did not ride
/// along (phase-2 report, Decision 2 rule 2). Named whether or not the probe
/// drained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NftDetached {
    pub output_index: usize,
    pub detail: String,
}

/// The synthesis record (phase-2 report): which degrees were enabled, the
/// caps, the pinned axis order, and the per-shape traversal tallies.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisRecord {
    pub enabled: bool,
    pub degrees: SynthesisDegrees,
    pub caps: SynthesisCaps,
    pub axis_order: Vec<&'static str>,
    /// How the total-probe cap was shared across the shapes in `shapes`:
    /// as load-bearing as the axis order itself under truncation, and
    /// recorded next to it.
    pub allocation: String,
    /// Companion inputs the re-creation axis considered (the degree's
    /// candidate set; 0 when the degree is off).
    pub companions_considered: usize,
    /// Of those, how many qualified: carrying at least one token at
    /// amount 1 within token index 0..=3 (the syntactic shape of a
    /// box-identifying singleton). An axis that will generate nothing is
    /// visible here instead of only in zero tallies.
    pub companions_qualified: usize,
    pub shapes: Vec<ShapeTally>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisDegrees {
    pub companion_recreations: bool,
    pub successor_states: bool,
    pub splits: bool,
    pub mints: bool,
    pub permute_outputs: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisCaps {
    pub max_new_outputs: usize,
    pub max_successor_states: usize,
    pub max_output_permutations: usize,
    pub max_probes: usize,
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
    /// The synthesis record: degrees, caps, pinned axis order, shape tallies.
    pub synthesis: SynthesisRecord,
    /// Rejection classification of synthesized probes.
    pub rejections: Rejections,
    /// Script-matched outputs whose protocol NFT did not ride along.
    pub nft_detached: Vec<NftDetached>,
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
            synthesis: synthesis_record(&req.synthesis, max_probe_cap(req), 1, 0, 0, Vec::new()),
            rejections: Rejections::default(),
            nft_detached: Vec::new(),
        });
    }

    // ── probe space ──
    let syn = &req.synthesis;
    let syn_enabled = syn.active();
    let max_permutations = req.max_permutations.unwrap_or(120).max(1);
    let max_probes = max_probe_cap(req);
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

    // NFT riders (phase-2 spec, Decision 2 rule 2): with synthesis on, a
    // script-matched output shields holdings only when the protected input's
    // protocol NFT rides along — same id, same token index. Off = the
    // phase-1 rule, byte-identical.
    let riders: BTreeMap<String, (String, usize)> = protected
        .iter()
        .filter_map(|&i| {
            let tree = declared[i].ergo_tree.as_deref()?.trim().to_lowercase();
            let (idx, t) = declared[i]
                .tokens
                .iter()
                .enumerate()
                .find(|(_, t)| nfts.contains(&t.id.to_lowercase()))?;
            Some((tree, (t.id.to_lowercase(), idx)))
        })
        .collect();

    // The five synthesis axes, in the pinned order; the sixth (input
    // permutation + decoy combinations) is the inner loop below.
    let axes = AxisIter::build(
        syn,
        req,
        &roles,
        &declared,
        req.outputs.len(),
        req.outputs
            .iter()
            .filter(|o| {
                o.box_
                    .ergo_tree
                    .as_deref()
                    .map(|s| protected_trees.contains(&s.trim().to_lowercase()))
                    .unwrap_or(false)
            })
            .count(),
    );
    // Companion visibility for the re-creation axis: who was considered,
    // who qualified. An axis that will generate nothing (e.g. a request
    // whose only companion carries its singleton at amount 3) is readable
    // here instead of discoverable only by instrumenting the run.
    let companions_considered = usize::from(syn.companion_recreations)
        * roles.iter().filter(|&&r| r == DrainRole::Companion).count();
    let companions_qualified = usize::from(syn.companion_recreations)
        * (0..roles.len())
            .filter(|&i| {
                roles[i] == DrainRole::Companion
                    && declared[i].tokens.iter().take(4).any(|t| t.amount == 1)
            })
            .count();
    if syn.companion_recreations && companions_considered > 0 && companions_qualified == 0 {
        notes.push(format!(
            "companion re-creations: {companions_considered} companion(s) considered, 0 qualified              (no companion carries an amount-1 token at index 0..=3); the axis will generate nothing"
        ));
    }
    if syn_enabled {
        for (s, truncated) in axes.out_perms_truncated.iter().enumerate() {
            if *truncated {
                notes.push(format!(
                    "output permutations capped at {} ({} outputs, shape '{}'), sampled lexicographically",
                    syn.max_output_permutations.max(1),
                    req.outputs.len() + axes.shapes[s].new_count(),
                    axes.shapes[s].static_label
                ));
            }
        }
    }

    let mut probes_total = 0usize;
    let mut probes_run = 0usize;
    let mut hits = 0usize;
    let mut best: Option<(u128, DrainHit)> = None;
    let mut seen: HashSet<String> = HashSet::new();
    let mut capped = false;
    let mut rejections = Rejections::default();
    // ── the cap is ALLOCATED, not spent depth-first ──
    // Truncation is the normal case, and shape `none` (the phase-1 point)
    // can alone produce more points than the whole budget on a real set —
    // depth-first spending would zero every synthesis shape exactly when
    // they matter. The pinned order's promise is that truncation *preserves
    // the new degrees*; the allocator is what delivers it: the budget is
    // split evenly across synthesis shapes (cumulative ceilings), each
    // shape runs inside its slice, and a shape that finishes early leaves
    // its remainder to later shapes. With one shape this is exactly the
    // phase-1 cap, byte-identical.
    let shape_count = axes.shapes.len();
    let base_slice = max_probes / shape_count;
    let extra = max_probes % shape_count;
    let mut shape_ceilings: Vec<usize> = Vec::with_capacity(shape_count);
    let mut acc = 0usize;
    for i in 0..shape_count {
        acc += base_slice + usize::from(i < extra);
        shape_ceilings.push(acc);
    }

    // One tally bucket per (shape, filler count) — the filler dimension is
    // part of the reported label, which is what makes padded reachability
    // observable in the report.
    let mut shape_tallies: Vec<ShapeTally> = Vec::new();
    let mut tally_of: Vec<Vec<usize>> = Vec::with_capacity(axes.shapes.len());
    for (si, s) in axes.shapes.iter().enumerate() {
        let budget = shape_ceilings[si] - if si == 0 { 0 } else { shape_ceilings[si - 1] };
        let mut per_f = Vec::with_capacity(axes.filler_domains[si].len());
        for &f in &axes.filler_domains[si] {
            per_f.push(shape_tallies.len());
            shape_tallies.push(ShapeTally {
                shape: s.tally_label(f),
                generated: 0,
                run: 0,
                budget,
            });
        }
        tally_of.push(per_f);
    }
    let mut nft_detached: Vec<NftDetached> = Vec::new();
    let mut detached_seen: HashSet<String> = HashSet::new();
    const DETACHED_CAP: usize = 16;

    let mut axes = axes;
    while let Some(point) = axes.next() {
        // This shape's slice is spent: skip its remaining points (never
        // run them), let the odometer advance to the next shape.
        if probes_run >= shape_ceilings[point.shape_index] {
            capped = true;
            continue;
        }
        let shape_ceiling = shape_ceilings[point.shape_index];
        'shape: {
            for arrangement in &perms {
                for combo_idx in ComboCounter::new(&combo_lengths) {
                    let combo: Vec<(String, ScenarioBox)> = combo_idx
                        .iter()
                        .enumerate()
                        .map(|(slot, &vi)| decoys[slot][vi].clone())
                        .collect();
                    // The combo's attacker-slot token ids, in slot order — the
                    // only source for re-creation padding (phase-2 spec: the
                    // padding is sourced, never conjured).
                    let attacker_slot_tokens: Vec<Vec<String>> = attacker_slots
                        .iter()
                        .map(|&s| {
                            combo[s]
                                .1
                                .tokens
                                .iter()
                                .map(|t| t.id.to_lowercase())
                                .collect()
                        })
                        .collect();
                    for &payout in &payout_modes {
                        // The re-creation family's innermost axis: filler counts
                        // (a single 0 for shapes without a re-creation).
                        for (f_idx, &fillers) in point.filler_domain.iter().enumerate() {
                            probes_total += 1;
                            if probes_run >= shape_ceiling {
                                capped = true;
                                break 'shape;
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
                            let tally = tally_of[point.shape_index][f_idx];
                            let (realized_outputs, _synthesized) = if point.is_phase1() {
                                match realize_outputs(req, &realized_inputs, payout, &attacker_tree)
                                {
                                    Some(o) => {
                                        let flags = vec![false; o.len()];
                                        if syn_enabled {
                                            shape_tallies[tally].generated += 1;
                                        }
                                        (o, flags)
                                    }
                                    None => {
                                        notes.push(
                                "drain-mode payout not constructible (negative remainder); skipping it"
                                    .into(),
                            );
                                        continue;
                                    }
                                }
                            } else {
                                match materialize_synthesized(
                                    req,
                                    payout,
                                    &point,
                                    fillers,
                                    &realized_inputs,
                                    &combo,
                                    &attacker_slot_tokens,
                                    &protected_trees,
                                    free_output,
                                    &attacker_tree,
                                ) {
                                    Some((o, flags)) => {
                                        shape_tallies[tally].generated += 1;
                                        (o, flags)
                                    }
                                    // Unsourced padding or an unfundable shape: not
                                    // generated at all (never a conservation tally).
                                    None => continue,
                                }
                            };

                            // Deduplicate: identical (inputs, outputs) shapes are one probe.
                            let key =
                                match serde_json::to_string(&(&realized_inputs, &realized_outputs))
                                {
                                    Ok(k) => k,
                                    Err(_) => continue,
                                };
                            if !seen.insert(key) {
                                continue;
                            }
                            probes_run += 1;
                            if syn_enabled {
                                shape_tallies[tally].run += 1;
                            }

                            // ── oracle: full transaction validation ──
                            let probe_seq = probes_run;
                            let (tx_request, oracle_outputs) = match build_tx_request(
                                req,
                                &realized_inputs,
                                &realized_outputs,
                                probe_seq,
                            ) {
                                Ok(r) => r,
                                Err(_) => continue,
                            };
                            let check = match tx_check(&tx_request) {
                                Ok(c) => c,
                                Err(_) => continue,
                            };
                            if !check.valid {
                                if syn_enabled {
                                    match classify_rejection(&check) {
                                        Rejection::Conservation => rejections.conservation += 1,
                                        Rejection::MissingKey => rejections.missing_key += 1,
                                        Rejection::Script => rejections.script += 1,
                                    }
                                }
                                continue;
                            }
                            let mut disqualified = false;
                            let mut refused_key = false;
                            for ic in &check.inputs {
                                let pos = ic.index;
                                let accepted = match position_roles[pos] {
                                    DrainRole::Protected
                                    | DrainRole::Companion
                                    | DrainRole::External => ic.verdict == "pass",
                                    // The attacker signs their own boxes.
                                    DrainRole::Attacker => {
                                        ic.verdict == "pass" || ic.verdict == "needsProof"
                                    }
                                    DrainRole::Unknown => false,
                                };
                                if !accepted {
                                    if ic.verdict == "needsProof" {
                                        refused_key = true;
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
                                if syn_enabled && refused_key {
                                    rejections.missing_key += 1;
                                }
                                continue;
                            }

                            // ── objective: leak by identity and amount ──
                            let (extracted, detached) = leak(
                                &position_roles,
                                &realized_inputs,
                                &oracle_outputs,
                                &protected_trees,
                                &declared_free,
                                if syn_enabled { Some(&riders) } else { None },
                            );
                            if syn_enabled {
                                for d in detached {
                                    if detached_seen.insert(d.detail.clone())
                                        && nft_detached.len() < DETACHED_CAP
                                    {
                                        nft_detached.push(d);
                                    }
                                }
                            }
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
                                        shape: shape_tallies[tally].shape.clone(),
                                        witness: witness_bundle(
                                            req,
                                            &position_roles,
                                            &realized_inputs,
                                            &oracle_outputs,
                                            &tx_request,
                                        ),
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        } // 'shape
    }

    if capped {
        if shape_count == 1 {
            notes.push(format!(
                "probe cap {max_probes} reached; the space was truncated"
            ));
        } else {
            notes.push(format!(
                "probe cap {max_probes} allocated across {shape_count} synthesis shapes \
                 (~{base_slice} probes each, unused budget flows to later shapes); \
                 the space was truncated"
            ));
        }
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
        synthesis: synthesis_record(
            syn,
            max_probes,
            shape_count,
            companions_considered,
            companions_qualified,
            shape_tallies,
        ),
        rejections,
        nft_detached,
    })
}

// ── Phase-2 probe axes ───────────────────────────────────────────────────────

/// The resolved total-probe cap (phase-2 spec: 50,000 default).
fn max_probe_cap(req: &DrainRequest) -> usize {
    req.max_probes.unwrap_or(50_000).max(1)
}

fn synthesis_record(
    syn: &Synthesis,
    max_probes: usize,
    shape_count: usize,
    companions_considered: usize,
    companions_qualified: usize,
    shapes: Vec<ShapeTally>,
) -> SynthesisRecord {
    SynthesisRecord {
        enabled: syn.active(),
        degrees: SynthesisDegrees {
            companion_recreations: syn.companion_recreations,
            successor_states: syn.successor_states,
            splits: syn.splits,
            mints: syn.mints,
            permute_outputs: syn.permute_outputs,
        },
        caps: SynthesisCaps {
            max_new_outputs: syn.resolved_max_new_outputs(),
            max_successor_states: syn.max_successor_states.max(1),
            max_output_permutations: syn.max_output_permutations.max(1),
            max_probes,
        },
        axis_order: AXIS_ORDER.to_vec(),
        allocation: format!(
            "the probe cap is allocated across {shape_count} synthesis shapes in the pinned order              (equal slices, unused budget flows to later shapes); a shape that exhausts its slice              is skipped, never steals from later shapes"
        ),
        companions_considered,
        companions_qualified,
        shapes,
    }
}

/// One companion re-creation: the companion box rebuilt verbatim (same
/// script, value, tokens) with sourced filler tokens inserted before its
/// singleton at `nft_index`, moving that token to `nft_index + fillers`.
/// The filler count is the family's own innermost axis (see
/// [`AXIS_ORDER`]), so padded shapes share every re-creation shape's budget
/// instead of starving behind the unpadded one.
#[derive(Debug, Clone)]
struct Recreate {
    slot: usize,
    nft_index: usize,
}

/// A synthesized-output shape: at most one companion re-creation plus
/// attacker sinks (`new_count() ≤ maxNewOutputs`).
#[derive(Debug, Clone)]
struct ShapeDesc {
    /// The re-creation, if any: the companion slot and the token index to
    /// move. A companion qualifies through any token it carries at amount 1
    /// — the syntactic shape of a box-identifying singleton. (The map's
    /// `TokenClass::Singleton` is the semantic source of the same fact when
    /// a map is available; the request carries only the boxes, so the
    /// amount-1 shape is the honest request-local proxy. This is
    /// deliberately NOT `protocolNfts`, which the phase-1 validation pins
    /// to the protected boxes' own `tokens(0)` — a companion naturally
    /// carries its own singleton, never a protected box's NFT.)
    recreate: Option<Recreate>,
    sinks: usize,
    /// The label without the filler count (`none`, `sinks(1)`,
    /// `recreate(companion=1,nft=0)`).
    static_label: String,
}

impl ShapeDesc {
    fn new_count(&self) -> usize {
        usize::from(self.recreate.is_some()) + self.sinks
    }

    /// The reported shape label for one filler count. Re-creation shapes
    /// embed it — the per-filler-count tally bucket is what makes padded
    /// reachability observable; other shapes have no filler dimension.
    fn tally_label(&self, fillers: usize) -> String {
        match &self.recreate {
            Some(rc) => {
                let base = format!(
                    "recreate(companion={},nft={},fillers={})",
                    rc.slot, rc.nft_index, fillers
                );
                if self.sinks > 0 {
                    format!("{base}+sink")
                } else {
                    base
                }
            }
            None => self.static_label.clone(),
        }
    }
}

/// Per-successor state: the phase-1 collapse (payout-dependent: all-verbatim
/// under `verbatim`, all-minimized under `drain`) or an explicit
/// `{verbatim, minimized}` bitmask over the declared successors in order.
#[derive(Debug, Clone)]
enum SuccState {
    Phase1,
    Bits(Vec<bool>),
}

/// One point in the five synthesis axes. The sixth axis — input permutation
/// and decoy combinations, with the payout mode — is enumerated inside these.
struct ProbePoint<'a> {
    shape_index: usize,
    shape: &'a ShapeDesc,
    out_perm: &'a [usize],
    succ: &'a SuccState,
    split: bool,
    mint: Option<u64>,
    /// The filler counts this shape materializes over (the family's
    /// innermost axis; `[0]` without a re-creation). Owned, so the caller
    /// can iterate it while holding the point.
    filler_domain: Vec<usize>,
}

impl ProbePoint<'_> {
    /// The exact phase-1 point: no synthesized outputs, identity output
    /// order, the phase-1 successor collapse, no split, no mint. Such points
    /// are materialized by the untouched phase-1 code path, which is what
    /// makes the all-off block byte-identical by construction.
    fn is_phase1(&self) -> bool {
        matches!(self.succ, SuccState::Phase1)
            && self.shape.new_count() == 0
            && self.out_perm.iter().enumerate().all(|(i, &p)| i == p)
            && !self.split
            && self.mint.is_none()
    }
}

/// A lazy odometer over the five synthesis axes in the pinned order
/// (outermost first): shapes → output permutation → per-successor states →
/// value splits → mint variants. The total-probe cap binds inside the
/// phase-1 axes, so truncation preserves the new degrees and spends its
/// budget varying the phase-1 space inside each synthesis shape.
///
/// A lending iterator (`next(&mut self) -> Option<ProbePoint<'_>>`): each
/// point borrows the axis tables, which the caller must drop before
/// advancing — the hunt's loop does exactly that.
struct AxisIter {
    shapes: Vec<ShapeDesc>,
    out_perms: Vec<Vec<Vec<usize>>>,
    out_perms_truncated: Vec<bool>,
    succs: Vec<SuccState>,
    splits: Vec<Vec<bool>>,
    mints: Vec<Vec<Option<u64>>>,
    /// Per shape: the filler counts its re-creation materializes over
    /// (innermost axis — `[0]` for shapes without a re-creation).
    filler_domains: Vec<Vec<usize>>,
    i: [usize; 5],
    started: bool,
    exhausted: bool,
}

impl AxisIter {
    #[allow(clippy::too_many_arguments)]
    fn build(
        syn: &Synthesis,
        req: &DrainRequest,
        roles: &[DrainRole],
        declared: &[ScenarioBox],
        template_outputs: usize,
        successor_count: usize,
    ) -> Self {
        let max_new = syn.resolved_max_new_outputs();

        // Axis 1: synthesized-output shapes, in the pinned order — none →
        // companion re-creations → re-creation + sink → sinks. A companion
        // qualifies through any token it carries at amount 1 (see
        // `ShapeDesc.recreate` — deliberately not `protocolNfts`).
        let mut shapes: Vec<ShapeDesc> = vec![ShapeDesc {
            recreate: None,
            sinks: 0,
            static_label: "none".to_string(),
        }];
        let mut recreations: Vec<Recreate> = Vec::new();
        if syn.companion_recreations {
            for (slot, role) in roles.iter().enumerate() {
                if *role != DrainRole::Companion {
                    continue;
                }
                // Insertion only moves a token rightward: reachable targets
                // are `j..=3`. The filler count is not part of the shape —
                // it is the family's innermost axis, so the unpadded
                // re-creation never starves the padded ones (both review
                // passes' sourcing work stays reachable by construction).
                for (j, t) in declared[slot].tokens.iter().enumerate() {
                    if t.amount == 1 && j <= 3 {
                        recreations.push(Recreate { slot, nft_index: j });
                    }
                }
            }
        }
        for rc in &recreations {
            shapes.push(ShapeDesc {
                recreate: Some(rc.clone()),
                sinks: 0,
                static_label: format!("recreate(companion={},nft={})", rc.slot, rc.nft_index),
            });
        }
        if syn.companion_recreations && max_new >= 2 {
            for rc in &recreations {
                shapes.push(ShapeDesc {
                    recreate: Some(rc.clone()),
                    sinks: 1,
                    static_label: format!(
                        "recreate(companion={},nft={})+sink",
                        rc.slot, rc.nft_index
                    ),
                });
            }
        }
        if max_new >= 1 {
            shapes.push(ShapeDesc {
                recreate: None,
                sinks: 1,
                static_label: "sinks(1)".to_string(),
            });
        }
        if syn.splits && max_new >= 2 {
            shapes.push(ShapeDesc {
                recreate: None,
                sinks: 2,
                static_label: "sinks(2)".to_string(),
            });
        }

        // Axis 2: output permutation (identity when off).
        let (out_perms, out_perms_truncated): (Vec<Vec<Vec<usize>>>, Vec<bool>) = shapes
            .iter()
            .map(|s| {
                if syn.permute_outputs {
                    let (perms, truncated) = permutations(
                        template_outputs + s.new_count(),
                        syn.max_output_permutations.max(1),
                    );
                    (perms, truncated)
                } else {
                    (vec![(0..template_outputs + s.new_count()).collect()], false)
                }
            })
            .fold((Vec::new(), Vec::new()), |(mut ps, mut ts), (p, t)| {
                ps.push(p);
                ts.push(t);
                (ps, ts)
            });

        // Axis 3: per-successor states (`2^s` capped).
        let succs: Vec<SuccState> = if syn.successor_states && successor_count >= 1 {
            let total = 1usize << successor_count.min(usize::BITS as usize - 1);
            (0..total.min(syn.max_successor_states.max(1)))
                .map(|m| SuccState::Bits((0..successor_count).map(|k| (m >> k) & 1 == 1).collect()))
                .collect()
        } else {
            vec![SuccState::Phase1]
        };

        // Axis 4: value splits (only shapes with two sinks split).
        let splits: Vec<Vec<bool>> = shapes
            .iter()
            .map(|s| vec![syn.splits && s.sinks >= 2])
            .collect();

        // Axis 5: mint variants — amounts from the declared request alone.
        let amounts = mint_amounts(req);
        let mints: Vec<Vec<Option<u64>>> = shapes
            .iter()
            .map(|s| {
                let mut v: Vec<Option<u64>> = vec![None];
                if syn.mints && s.new_count() >= 1 {
                    v.extend(amounts.iter().map(|&a| Some(a)));
                }
                v
            })
            .collect();

        // The re-creation family's own innermost axis: filler counts. It is
        // deliberately NOT a shape dimension — sequenced shapes would let the
        // unpadded re-creation eat the whole budget (the one reachability
        // bug both review passes would have shipped). Innermost, every
        // (outperm, successor, split, mint, perm, combo, payout) point tries
        // every filler count, so the sourcing machinery executes on real
        // requests whenever any budget reaches a re-creation shape.
        let filler_domains: Vec<Vec<usize>> = shapes
            .iter()
            .map(|s| match &s.recreate {
                Some(rc) => (0..=(3 - rc.nft_index)).collect(),
                None => vec![0],
            })
            .collect();

        AxisIter {
            shapes,
            out_perms,
            out_perms_truncated,
            succs,
            splits,
            mints,
            filler_domains,
            i: [0; 5],
            started: false,
            exhausted: false,
        }
    }

    fn radix(&self, d: usize) -> usize {
        match d {
            0 => self.shapes.len(),
            1 => self.out_perms[self.i[0]].len(),
            2 => self.succs.len(),
            3 => self.splits[self.i[0]].len(),
            _ => self.mints[self.i[0]].len(),
        }
    }

    fn next(&mut self) -> Option<ProbePoint<'_>> {
        if self.exhausted {
            return None;
        }
        if self.started {
            let mut d = 4;
            loop {
                self.i[d] += 1;
                if self.i[d] < self.radix(d) {
                    break;
                }
                self.i[d] = 0;
                if d == 0 {
                    self.exhausted = true;
                    return None;
                }
                d -= 1;
            }
        } else {
            self.started = true;
            for d in 0..5 {
                if self.radix(d) == 0 {
                    self.exhausted = true;
                    return None;
                }
            }
        }
        Some(ProbePoint {
            shape_index: self.i[0],
            shape: &self.shapes[self.i[0]],
            out_perm: &self.out_perms[self.i[0]][self.i[1]],
            succ: &self.succs[self.i[2]],
            split: self.splits[self.i[0]][self.i[3]],
            mint: self.mints[self.i[0]][self.i[4]],
            filler_domain: self.filler_domains[self.i[0]].clone(),
        })
    }
}

// ── Phase-2 materialization ──────────────────────────────────────────────────

/// Shrink a successor the phase-1 way: minimal keep-value, one token per id.
fn shrink_successor(b: &mut ScenarioBox) {
    b.value = b.value.min(DRAIN_KEEP_VALUE);
    for t in &mut b.tokens {
        t.amount = 1;
    }
}

/// Why the oracle rejected a synthesized probe (phase-2 spec, Decision 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rejection {
    Conservation,
    MissingKey,
    Script,
}

/// Classify a rejected transaction from `txcheck`'s problems and per-input
/// verdicts. Conservation first: it is the vacuity risk the classification
/// exists to expose.
fn classify_rejection(check: &crate::txcheck::TxCheck) -> Rejection {
    let conservation = check
        .problems
        .iter()
        .any(|p| p.contains("not conserved") || p.contains("outputs carry"));
    if conservation {
        return Rejection::Conservation;
    }
    if check.inputs.iter().any(|ic| ic.verdict == "needsProof") {
        return Rejection::MissingKey;
    }
    Rejection::Script
}

/// Materialize one synthesized probe. Returns `None` when the shape cannot
/// be funded or its padding is not sourced — such points are skipped, never
/// generated to fail conservation.
#[allow(clippy::too_many_arguments)]
fn materialize_synthesized(
    req: &DrainRequest,
    payout: &str,
    point: &ProbePoint,
    fillers_count: usize,
    realized_inputs: &[ScenarioBox],
    combo: &[(String, ScenarioBox)],
    attacker_slot_tokens: &[Vec<String>],
    protected_trees: &HashSet<String>,
    free_output: Option<usize>,
    attacker_tree: &str,
) -> Option<(Vec<ScenarioBox>, Vec<bool>)> {
    let shape = point.shape;

    // 1. Template outputs under the per-successor state.
    let mut outs: Vec<ScenarioBox> = req.outputs.iter().map(|o| o.box_.clone()).collect();
    let flags = vec![false; outs.len()];
    let succ_idxs: Vec<usize> = req
        .outputs
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.box_
                .ergo_tree
                .as_deref()
                .map(|s| protected_trees.contains(&s.trim().to_lowercase()))
                .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();
    match point.succ {
        SuccState::Phase1 => {
            if payout == "drain" {
                for &i in &succ_idxs {
                    shrink_successor(&mut outs[i]);
                }
            }
        }
        SuccState::Bits(bits) => {
            for (k, &i) in succ_idxs.iter().enumerate() {
                if bits.get(k).copied().unwrap_or(false) {
                    shrink_successor(&mut outs[i]);
                }
            }
        }
    }
    if payout == "drain" {
        if let Some(f) = free_output {
            outs[f].ergo_tree = Some(attacker_tree.to_string());
        }
    }

    // 2. Companion re-creations. The padding is sourced, never conjured:
    //    filler ids come from the combo's attacker inputs (slot order, token
    //    order — the deterministic pairing), distinct from each other and
    //    from every id the re-creation carries. A box cannot hold one id
    //    twice and conservation rejects any output id the inputs do not
    //    carry, so unsourced padding is not generated at all.
    let mut new_boxes: Vec<ScenarioBox> = Vec::new();
    if let Some(rc) = &shape.recreate {
        let companion = &combo[rc.slot].1;
        let mut exclude: HashSet<String> = companion
            .tokens
            .iter()
            .map(|t| t.id.to_lowercase())
            .collect();
        let mut fillers: Vec<String> = Vec::with_capacity(fillers_count);
        'source: for slot_ids in attacker_slot_tokens {
            for id in slot_ids {
                if !exclude.contains(id) {
                    exclude.insert(id.clone());
                    fillers.push(id.clone());
                    if fillers.len() == fillers_count {
                        break 'source;
                    }
                }
            }
        }
        if fillers.len() < fillers_count {
            return None;
        }
        let mut tokens: Vec<TokenAmount> =
            Vec::with_capacity(companion.tokens.len() + fillers_count);
        for (idx, t) in companion.tokens.iter().enumerate() {
            if idx == rc.nft_index {
                tokens.extend(fillers.iter().map(|id| TokenAmount {
                    id: id.clone(),
                    amount: 1,
                }));
            }
            tokens.push(t.clone());
        }
        new_boxes.push(ScenarioBox {
            value: companion.value,
            ergo_tree: companion.ergo_tree.clone(),
            tokens,
            creation_height: companion.creation_height,
            registers: companion.registers.clone(),
            box_id: None,
            extension: Default::default(),
        });
    }

    // 3. The value ledger. The pool is what the sinks may take (or the free
    //    output absorbs): inputs minus template outputs at their
    //    state-adjusted values, minus re-creations. The free output sits at
    //    its declared value until step 4.
    let erg_in: i128 = realized_inputs.iter().map(|b| b.value as i128).sum();
    let erg_out: i128 = outs.iter().map(|b| b.value as i128).sum::<i128>()
        + new_boxes.iter().map(|b| b.value as i128).sum::<i128>();
    let pool = erg_in - erg_out;
    if pool < 0 {
        return None;
    }

    // 4. Sinks, or the free output absorbing the pool.
    let k = shape.sinks;
    let mut sink_boxes: Vec<ScenarioBox> = Vec::new();
    if k >= 1 {
        // The free output keeps its declared amount; the sinks take the pool.
        let sink_floor = DRAIN_KEEP_VALUE as i128;
        if pool < sink_floor {
            return None;
        }
        let (v1, v2) = if k >= 2 {
            // The first deterministic split: ⌈half⌉/⌊half⌋, each floored.
            let half_up = (pool + 1) / 2;
            let half_down = pool - half_up;
            if half_down < sink_floor {
                return None;
            }
            (half_up, Some(half_down))
        } else {
            (pool, None)
        };
        // The token pool: what the inputs carry that no other output does.
        let mut committed: BTreeMap<String, u128> = BTreeMap::new();
        for (i, b) in outs.iter().enumerate() {
            if Some(i) == free_output {
                continue;
            }
            for t in &b.tokens {
                *committed.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
            }
        }
        for b in &new_boxes {
            for t in &b.tokens {
                *committed.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
            }
        }
        let token_pool: BTreeMap<String, u128> = tok_in_map(realized_inputs)
            .into_iter()
            .filter_map(|(id, in_amt)| {
                let left = in_amt.saturating_sub(*committed.get(&id).unwrap_or(&0));
                if left > 0 {
                    Some((id, left))
                } else {
                    None
                }
            })
            .collect();
        let mut sink1_tokens: Vec<TokenAmount> = Vec::new();
        if let Some(amt) = point.mint {
            // A minted placeholder leads the sink's tokens: a script checking
            // tokens(0) sees it.
            sink1_tokens.push(TokenAmount {
                id: MINT_SENTINEL.to_string(),
                amount: amt,
            });
        }
        for (id, amt) in &token_pool {
            let share = if k >= 2 { amt.div_ceil(2) } else { *amt };
            sink1_tokens.push(TokenAmount {
                id: id.clone(),
                amount: u64::try_from(share).ok()?,
            });
        }
        sink_boxes.push(ScenarioBox {
            value: i64::try_from(v1).ok()?,
            ergo_tree: Some(attacker_tree.to_string()),
            tokens: sink1_tokens,
            creation_height: req.height,
            ..Default::default()
        });
        if let Some(v2) = v2 {
            let mut sink2_tokens: Vec<TokenAmount> = Vec::new();
            for (id, amt) in &token_pool {
                let share = amt - amt.div_ceil(2);
                if share > 0 {
                    sink2_tokens.push(TokenAmount {
                        id: id.clone(),
                        amount: u64::try_from(share).ok()?,
                    });
                }
            }
            sink_boxes.push(ScenarioBox {
                value: i64::try_from(v2).ok()?,
                ergo_tree: Some(attacker_tree.to_string()),
                tokens: sink2_tokens,
                creation_height: req.height,
                ..Default::default()
            });
        }
    } else if let Some(f) = free_output {
        // No sinks: the free output absorbs the pool.
        outs[f].value = (outs[f].value as i128 + pool).min(i64::MAX as i128) as i64;
        // Tokens: what the inputs carry that no other output does.
        let mut committed: BTreeMap<String, u128> = BTreeMap::new();
        for (i, b) in outs.iter().enumerate() {
            if i == f {
                continue;
            }
            for t in &b.tokens {
                *committed.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
            }
        }
        for b in &new_boxes {
            for t in &b.tokens {
                *committed.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
            }
        }
        let pool_tok: BTreeMap<String, u128> = tok_in_map(realized_inputs)
            .into_iter()
            .filter_map(|(id, in_amt)| {
                let left = in_amt.saturating_sub(*committed.get(&id).unwrap_or(&0));
                if left > 0 {
                    Some((id, left))
                } else {
                    None
                }
            })
            .collect();
        if payout == "drain" {
            // The phase-1 formula: the free output's tokens are REPLACED by
            // the remainder; the declared-mint quirk is preserved.
            let declared = outs[f].tokens.clone();
            let mut free_tokens: Vec<TokenAmount> = pool_tok
                .into_iter()
                .map(|(id, amt)| TokenAmount {
                    id,
                    amount: u64::try_from(amt).unwrap_or(u64::MAX),
                })
                .collect();
            if free_tokens.is_empty() && !declared.is_empty() {
                free_tokens = declared;
            }
            outs[f].tokens = free_tokens;
        } else {
            // Verbatim: the declared tokens plus whatever a minimized
            // successor freed (merged by id).
            let mut free_tokens = outs[f].tokens.clone();
            for (id, amt) in pool_tok {
                if amt == 0 {
                    continue;
                }
                let amt = u64::try_from(amt).unwrap_or(u64::MAX);
                match free_tokens.iter_mut().find(|t| t.id.to_lowercase() == id) {
                    Some(t) => t.amount = t.amount.saturating_add(amt),
                    None => free_tokens.push(TokenAmount { id, amount: amt }),
                }
            }
            outs[f].tokens = free_tokens;
        }
    }
    // Mint on a re-creation-only shape: appended (never prepended — the
    // re-creation's NFT index is the shape's whole point).
    if k == 0 {
        if let (Some(amt), Some(rc)) = (point.mint, new_boxes.first_mut()) {
            rc.tokens.push(TokenAmount {
                id: MINT_SENTINEL.to_string(),
                amount: amt,
            });
        }
    }

    // 5. Assemble and apply the output permutation (axis 2).
    let mut all: Vec<ScenarioBox> = outs;
    let mut all_flags = flags;
    for b in new_boxes {
        all.push(b);
        all_flags.push(true);
    }
    for b in sink_boxes {
        all.push(b);
        all_flags.push(true);
    }
    if all.len() != point.out_perm.len() {
        return None;
    }
    let mut permuted: Vec<ScenarioBox> = Vec::with_capacity(all.len());
    let mut permuted_flags: Vec<bool> = Vec::with_capacity(all.len());
    for &src in point.out_perm {
        permuted.push(all[src].clone());
        permuted_flags.push(all_flags[src]);
    }
    Some((permuted, permuted_flags))
}

/// Lowercased token holdings of the realized inputs.
fn tok_in_map(realized_inputs: &[ScenarioBox]) -> BTreeMap<String, u128> {
    let mut tok_in: BTreeMap<String, u128> = BTreeMap::new();
    for b in realized_inputs {
        for t in &b.tokens {
            *tok_in.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    tok_in
}

/// Mint amounts from the declared request alone: `{1, the largest amount of
/// any token held by a declared input, the largest amount declared on any
/// template output}`, deduplicated, ascending. Never derived from the target
/// script — that is the one property the anti-cheat rests on.
fn mint_amounts(req: &DrainRequest) -> Vec<u64> {
    let mut amounts: Vec<u64> = vec![1];
    let max_in = req
        .inputs
        .iter()
        .flat_map(|i| i.box_.tokens.iter().map(|t| t.amount))
        .max();
    let max_out = req
        .outputs
        .iter()
        .flat_map(|o| o.box_.tokens.iter().map(|t| t.amount))
        .max();
    for m in [max_in, max_out].into_iter().flatten() {
        if !amounts.contains(&m) {
            amounts.push(m);
        }
    }
    amounts.sort_unstable();
    amounts
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
    riders: Option<&BTreeMap<String, (String, usize)>>,
) -> (BTreeMap<String, u128>, Vec<NftDetached>) {
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
    let mut detached: Vec<NftDetached> = Vec::new();
    for (pos, b) in realized_outputs.iter().enumerate() {
        let pinned = b
            .ergo_tree
            .as_deref()
            .map(|s| protected_trees.contains(&s.trim().to_lowercase()))
            .unwrap_or(false);
        if !pinned {
            continue;
        }
        // Phase-2 rule (Decision 2, rule 2): with synthesis on, a
        // script-matched output shields only when the protected input's
        // protocol NFT rides along — same id, same token index. Script
        // equality alone would let a synthesized script-clone park the
        // reserves, score zero leak, and walk away — a false negative, so
        // every such output is also named in the report.
        if let Some(riders) = riders {
            let tree = b
                .ergo_tree
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_lowercase();
            // No rider requirement recorded (defensive): script bytes
            // count, as in phase 1.
            if let Some((nft, idx)) = riders.get(&tree) {
                let rides = b
                    .tokens
                    .get(*idx)
                    .map(|t| t.id.to_lowercase() == *nft)
                    .unwrap_or(false);
                if !rides {
                    detached.push(NftDetached {
                        output_index: pos,
                        detail: format!(
                            "output {pos} matches a protected script but tokens({idx}) is not the protocol NFT {nft}"
                        ),
                    });
                    continue;
                }
            }
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
    (out, detached)
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

/// Build the oracle request from a realized probe, and return the oracle's
/// view of the outputs: mint sentinels resolved to the first input's box id
/// (`txcheck` allows exactly one minted id — the first input's box id). The
/// returned boxes are what the witness and the protocol scenarios replay,
/// so the oracle and every validator see the same transaction.
fn build_tx_request(
    req: &DrainRequest,
    realized_inputs: &[ScenarioBox],
    realized_outputs: &[ScenarioBox],
    probe_seq: usize,
) -> Result<(TxRequest, Vec<ScenarioBox>), SandboxError> {
    let mut boxes = Vec::with_capacity(realized_inputs.len() + req.data_inputs.len());
    let mut tx_inputs = Vec::with_capacity(realized_inputs.len());
    let mut first_input_id: Option<String> = None;
    for (i, b) in realized_inputs.iter().enumerate() {
        let bj = box_json(b, &format!("drain|{probe_seq}|in|{i}"))?;
        let id = bj["boxId"].as_str().unwrap_or_default().to_string();
        if i == 0 {
            first_input_id = Some(id.clone());
        }
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
    let mut oracle_outputs = Vec::with_capacity(realized_outputs.len());
    let mut outputs = Vec::with_capacity(realized_outputs.len());
    for b in realized_outputs {
        let mut b = b.clone();
        if b.tokens.iter().any(|t| t.id == MINT_SENTINEL) {
            let first = first_input_id.as_deref().ok_or_else(|| {
                SandboxError::Scenario("mint probe built with no inputs".to_string())
            })?;
            for t in &mut b.tokens {
                if t.id == MINT_SENTINEL {
                    t.id = first.to_string();
                }
            }
        }
        outputs.push(box_json(&b, "unused")?);
        oracle_outputs.push(b);
    }
    Ok((
        TxRequest {
            tx: Tx {
                inputs: tx_inputs,
                data_inputs,
                outputs,
            },
            boxes,
            height: Some(req.height),
            network: req.network.clone(),
        },
        oracle_outputs,
    ))
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

// ── The protocol map feeds the hunt ──────────────────────────────────────────

/// The map's proposed role as the drain hunt's role. The map never proposes
/// `attacker` — the attacker box is the hunt's own construct.
impl From<&crate::map::Role> for DrainRole {
    fn from(r: &crate::map::Role) -> Self {
        match r {
            crate::map::Role::Protected => DrainRole::Protected,
            crate::map::Role::Companion => DrainRole::Companion,
            crate::map::Role::External => DrainRole::External,
            crate::map::Role::Unknown => DrainRole::Unknown,
        }
    }
}

/// A mapped chain box as a scenario box. The archive shape carries no
/// additional registers, which is a fidelity limit only for scripts that
/// read R4–R9 of *other* inputs.
#[must_use]
pub fn scenario_box_from_chain(b: &crate::map::source::ChainBox) -> ScenarioBox {
    ScenarioBox {
        value: b.value.min(i64::MAX as u64) as i64,
        ergo_tree: Some(b.ergo_tree.clone()),
        tokens: b
            .tokens
            .iter()
            .map(|t| TokenAmount {
                id: t.id.clone(),
                amount: t.amount,
            })
            .collect(),
        creation_height: b.creation_height,
        registers: Default::default(),
        box_id: Some(b.box_id.clone()),
        extension: Default::default(),
    }
}

/// Build a phase-1 request from a protocol map — the map spec's promise that
/// roles are "what stops that being hand work". Every `protected` and
/// `companion` node becomes an input (the map's proposal, overridable by the
/// caller afterwards); `external` nodes become data inputs; `unknown` nodes
/// are skipped and **returned**, never silently defaulted. The template is
/// the generic honest one: each protected box rebuilt verbatim as its
/// successor, one free payee as the drain sink carrying the attacker's tree.
/// Verbatim probes of this template are generally unbalanced (the attacker's
/// own input has no sanctioned output); the drain-mode construction balances
/// by taking the remainder, which is the workhorse here.
///
/// Nodes the map left `unknown` are returned so the caller resolves them and
/// re-runs — the hunt would refuse them anyway.
#[must_use]
pub fn request_from_map(
    m: &crate::map::ProtocolMap,
    attacker: ScenarioBox,
) -> (DrainRequest, Vec<String>) {
    let mut inputs = Vec::new();
    let mut successors = Vec::new();
    let mut data_inputs = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for node in m.nodes.values() {
        match node.role {
            crate::map::Role::Protected | crate::map::Role::Companion => {
                let b = scenario_box_from_chain(&node.chain_box);
                if node.role == crate::map::Role::Protected {
                    successors.push(b.clone());
                }
                inputs.push(DrainInput {
                    role: (&node.role).into(),
                    box_: b,
                });
            }
            crate::map::Role::External => {
                data_inputs.push(scenario_box_from_chain(&node.chain_box));
            }
            crate::map::Role::Unknown => skipped.push(node.chain_box.box_id.clone()),
        }
    }

    inputs.push(DrainInput {
        role: DrainRole::Attacker,
        box_: attacker.clone(),
    });

    let payout_tree = attacker
        .ergo_tree
        .clone()
        .unwrap_or_else(|| "10010101d17300".to_string());
    let mut outputs: Vec<DrainOutput> = successors
        .into_iter()
        .map(|b| DrainOutput {
            payee: Payee::Fixed,
            box_: b,
        })
        .collect();
    outputs.push(DrainOutput {
        payee: Payee::Free,
        box_: ScenarioBox {
            value: 0,
            ergo_tree: Some(payout_tree),
            ..Default::default()
        },
    });

    (
        DrainRequest {
            inputs,
            data_inputs,
            outputs,
            protocol_nfts: m.protocol_nfts.iter().cloned().collect(),
            height: m.height,
            network: None,
            attacker_tree: None,
            max_permutations: None,
            max_probes: None,
            // Synthesis stays off by default: the caller opts in by setting
            // `request.synthesis` (the flagship runs declare it explicitly).
            synthesis: Synthesis::default(),
        },
        skipped,
    )
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

    #[test]
    fn synthesis_serde_defaults_make_the_omitted_block_all_off() {
        let syn: Synthesis = serde_json::from_value(json!({})).expect("parses");
        assert_eq!(syn, Synthesis::default());
        assert!(!syn.active());
        assert_eq!(syn.resolved_max_new_outputs(), 0);

        let syn: Synthesis = serde_json::from_value(json!({
            "maxNewOutputs": 0, "companionRecreations": false, "successorStates": false,
            "splits": false, "mints": false, "permuteOutputs": false
        }))
        .expect("parses");
        assert!(!syn.active());
        assert_eq!(syn.resolved_max_new_outputs(), 0);

        // An explicit cap alone is not a degree: an inactive block never
        // synthesizes, whatever cap it names.
        let syn: Synthesis = serde_json::from_value(json!({ "maxNewOutputs": 2 })).expect("parses");
        assert!(!syn.active());
        assert_eq!(syn.resolved_max_new_outputs(), 0);

        // An active block without an explicit cap gets the spec default.
        let syn: Synthesis =
            serde_json::from_value(json!({ "companionRecreations": true })).expect("parses");
        assert!(syn.active());
        assert_eq!(syn.resolved_max_new_outputs(), DEFAULT_MAX_NEW_OUTPUTS);
    }

    #[test]
    fn the_axis_odometer_walks_the_pinned_order() {
        let syn = Synthesis {
            max_new_outputs: Some(1),
            companion_recreations: true,
            splits: false,
            mints: true,
            permute_outputs: true,
            ..Synthesis::default()
        };
        // 1 protected, 1 companion carrying its own singleton, 1 attacker.
        let roles = [
            DrainRole::Protected,
            DrainRole::Companion,
            DrainRole::Attacker,
        ];
        let nft = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let declared = vec![
            ScenarioBox {
                tokens: vec![TokenAmount {
                    id: nft.into(),
                    amount: 1,
                }],
                ..Default::default()
            },
            ScenarioBox {
                tokens: vec![TokenAmount {
                    id: nft.into(),
                    amount: 1,
                }],
                ..Default::default()
            },
            ScenarioBox::default(),
        ];
        // The companion qualifies through its amount-1 token — not through
        // `protocolNfts` (the dummy request names the protected NFT only).
        let axes = AxisIter::build(&syn, &dummy_request(), &roles, &declared, 2, 1);

        // Shapes: none, then the companion's re-creation (singleton at index
        // 0; the filler count is NOT a shape — it is the family's innermost
        // axis), its +sink variant, then sinks(1). The odometer must walk
        // shapes outermost, then output permutations, then successor states,
        // then splits, then mints — the phase-1 axes innermost.
        let mut seen: Vec<(String, Vec<usize>, bool, Option<u64>)> = Vec::new();
        let mut iter = axes;
        while let Some(p) = iter.next() {
            seen.push((
                p.shape.static_label.clone(),
                p.out_perm.to_vec(),
                p.split,
                p.mint,
            ));
        }
        // Each shape's points are contiguous (the shape axis is outermost);
        // first-occurrence order is the pinned shape order.
        let mut shape_order: Vec<&str> = Vec::new();
        for (l, ..) in &seen {
            if shape_order.last() != Some(&l.as_str()) {
                shape_order.push(l.as_str());
            }
        }
        // maxNewOutputs is Some(1): the +sink variant (which needs 2) and
        // sinks(2) are absent.
        let expected_shapes = ["none", "recreate(companion=1,nft=0)", "sinks(1)"];
        assert_eq!(shape_order, expected_shapes);
        // The filler counts ride inside the re-creation shape's points.
        assert_eq!(
            axes_dummy_fillers(&syn, &roles, &declared),
            vec![0, 1, 2, 3],
            "the re-creation family's filler domain is 0..=3 for a singleton at index 0"
        );
        // Within the `none` shape: 2 output permutations, no mint variants
        // (a mint attaches only to a synthesized output).
        let none_points: Vec<_> = seen.iter().filter(|(l, ..)| l == "none").collect();
        assert_eq!(none_points.len(), 2, "2 outperms, no mints on `none`");
        assert_eq!(none_points[0].1, vec![0, 1], "identity outperm first");
        // Within the first re-creation shape: 3 outputs → 6 outperms × 2
        // mint variants; mints vary inside a fixed outperm, outperms after.
        let rc0: Vec<_> = seen
            .iter()
            .filter(|(l, ..)| l == "recreate(companion=1,nft=0)")
            .collect();
        assert_eq!(rc0.len(), 6 * 2);
        assert_eq!(rc0[0].3, None, "no mint first");
        assert!(rc0[1].3.is_some(), "mints vary inside a fixed outperm");
        assert_eq!(rc0[2].1, vec![0, 2, 1], "outperm varies after mints");
    }

    /// The filler domain the axis builder derives for the unit fixture.
    fn axes_dummy_fillers(
        syn: &Synthesis,
        roles: &[DrainRole],
        declared: &[ScenarioBox],
    ) -> Vec<usize> {
        let axes = AxisIter::build(syn, &dummy_request(), roles, declared, 2, 1);
        axes.filler_domains[1].clone()
    }

    /// A minimal valid request for the axis builder's mint-amount probe.
    fn dummy_request() -> DrainRequest {
        serde_json::from_value(json!({
            "inputs": [{ "role": "protected", "value": 1, "ergoTree": "10010101d17300" }],
            "outputs": [{ "payee": "free", "value": 1, "ergoTree": "10010101d17300" }],
            "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "height": 1
        }))
        .expect("parses")
    }
}
