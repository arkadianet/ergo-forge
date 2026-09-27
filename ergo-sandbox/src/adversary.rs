//! Adversary **search**: bounded, seeded, replayable *sequences* of
//! [`AttackOp`]s over a Play draft.
//!
//! [`crate::attack`] takes a hand-written sequence of adversary operations and
//! reports which scripts changed verdict. This module proposes the sequences
//! instead of asking for them: it walks a draft forward with operations drawn
//! from a seeded PRNG, carries each step's outputs into the next step's
//! unspent set, and — when a step changes a verdict — delta-debugs the
//! sequence back to one that still changes it.
//!
//! Four things make the result checkable rather than a story:
//!
//! - **Determinism.** The PRNG is a counter-mode `blake2b256` over the seed
//!   (`Prng`), the same digest the sandbox already uses for synthetic ids: no
//!   dependency, no platform entropy. The same draft, seed and caps run the
//!   same probes in the same order on any machine, and the report's
//!   `fingerprint` pins the whole trace — draft, resolved caps, every step, the
//!   witness. Two searches that agree on it ran the same experiment; searches
//!   that disagree ran something else.
//! - **Caps, and a cap that is visible.** Depth defaults to 2 and cannot
//!   exceed 4; every call to the reducer spends one probe out of `maxProbes`;
//!   a step carries at most `maxOpsPerStep` operations and `maxUnspent` boxes;
//!   each family's own caps (arrangements, tamper edits, token shift) are
//!   recorded next to them. Whatever a cap dropped appears in `rejections` or
//!   `notes`, never as silence.
//! - **A miss is not safe.** [`SearchVerdict::NoFlipUnderProbes`] says only
//!   that these operations, at these caps, on this draft, changed nothing. Like
//!   every result in this crate the answer is synthetic and carries
//!   `nodeValidated: false` (see [`SearchReport::claim`]): canonical
//!   transaction validation has not run, and a changed verdict is a scoped
//!   observation about a reconstructed transaction — never a claim that a live
//!   contract is exploitable.
//! - **A witness is a well-formed transaction.** A mutation that breaks the
//!   balances changes verdicts for reasons that have nothing to do with the
//!   contract. Such a step is recorded, tallied and kept out of the witness, and
//!   it is not carried forward either: a chain state an invalid transaction
//!   cannot produce is not somewhere the next step can stand.
//!
//! The search stops at the first verdict change it finds and shrinks that
//! sequence. It does not keep hunting for a better one, so the reported witness
//! is the first in the pinned order, not the shortest of all witnesses.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::attack::{apply_op, diff_inputs, AttackOp, AttackResult, InputDiff, MAX_BOX_TOKENS};
use crate::drain::permutations;
use crate::play::{self, PlayRequest, PlayResult};
use crate::scenario::ScenarioBox;
use crate::SandboxError;

/// Default depth: two moves — the drafted transaction, then one transaction
/// built on what the first one produced.
pub const DEFAULT_MAX_DEPTH: usize = 2;
/// Depth ceiling. A deeper sequence is a protocol reconstruction, not an
/// experiment; the search refuses to pretend otherwise.
pub const MAX_DEPTH_LIMIT: usize = 4;

/// Default reducer calls, the unmutated draft included. One call is a real
/// reduction, so this is the search's entire cost.
pub const DEFAULT_MAX_PROBES: usize = 32;
/// Probe ceiling.
pub const MAX_PROBES_LIMIT: usize = 512;

/// Default operations in one sampled sequence.
pub const DEFAULT_MAX_OPS_PER_STEP: usize = 3;
/// Operations-per-step ceiling.
pub const MAX_OPS_LIMIT: usize = 8;

/// Default boxes one step may carry into the next.
pub const DEFAULT_MAX_UNSPENT: usize = 24;
/// Unspent-box ceiling.
pub const MAX_UNSPENT_LIMIT: usize = 128;

/// Drafts carried forward per depth level. A shape bound, not a probe bound:
/// the probe cap decides how much work actually happens.
pub const MAX_FRONTIER: usize = 4;

/// Arrangements offered for one reorder axis, the draft's own declared order
/// excluded. Drawn from the drain hunt's bounded lexicographic enumeration.
pub const PERMUTATION_CAP: usize = 8;

/// Tamper edits kept, in the priority order the generator builds them. A
/// register-rich draft offers more; the rest are reported as dropped.
pub const TAMPER_CAP: usize = 12;

/// The most token slots one generated shift may prepend. Two is the smallest
/// shape a token-index game needs (the real one), against a wire cap of 255
/// tokens per box (recorded in the report as `families.maxBoxTokens`).
const SHIFT_BY_MAX: usize = 2;

/// How many times a colliding sequence draw is retried before it is taken as it
/// came. Small on purpose: the probe cap, not the collision rate, is the bound.
const DRAW_ATTEMPTS: usize = 4;

/// The seed an omitted or blank `seed` resolves to, so a report always names
/// the search it came from.
const DEFAULT_SEED: &str = "ergo-forge/adversary-search";

/// Version tag in the trace fingerprint: a change to what the fingerprint
/// covers changes this number.
const FINGERPRINT_VERSION: u32 = 1;

/// The operation kinds the generator can propose, in the order it considers
/// them. Recorded in every report so a truncated run states what it could have
/// tried and in what order.
pub const OP_KINDS: [&str; 6] = [
    "reorderInputs",
    "insertDecoy",
    "swapDataInput",
    "reorderDataInputs",
    "shiftTokenIndices",
    "tamperField",
];

/// How a step's outputs become the next step's draft, in words. The carry *is*
/// what makes this search multi-step, so it is recorded next to the caps rather
/// than left for a reader to infer from the numbers.
pub const CARRY_POLICY: &str = "each step spends the outputs it created first, then the boxes it left unspent in declared order, capped at maxUnspent, in one transaction at height+1; every carried box is recreated as an output with the same script, value, tokens and registers but a fresh Play id and height, so value and tokens are conserved without a synthetic pass-through sink; carried inputs carry no secrets or context variables, and a data input the cap dropped is dropped from the transaction too";

/// Caller-controlled knobs. Every field is optional and every value is clamped
/// into `1..=limit` by the resolver, so no request can make the search deeper,
/// wider or longer than the ceilings this module states.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SearchOptions {
    /// The PRNG seed. Blank resolves to this module's default seed; the
    /// resolved seed is echoed in the report and folded into the fingerprint.
    pub seed: String,
    /// Moves to carry forward (default [`DEFAULT_MAX_DEPTH`], ceiling
    /// [`MAX_DEPTH_LIMIT`]).
    pub max_depth: Option<usize>,
    /// Reducer calls the search may spend, the unmutated draft included
    /// (default [`DEFAULT_MAX_PROBES`], ceiling [`MAX_PROBES_LIMIT`]).
    pub max_probes: Option<usize>,
    /// Operations in one sampled sequence (default
    /// [`DEFAULT_MAX_OPS_PER_STEP`], ceiling [`MAX_OPS_LIMIT`]).
    pub max_ops_per_step: Option<usize>,
    /// Boxes one step may carry into the next (default
    /// [`DEFAULT_MAX_UNSPENT`], ceiling [`MAX_UNSPENT_LIMIT`]).
    pub max_unspent: Option<usize>,
}

/// A Play draft plus the search's knobs. The draft is the honest transaction:
/// the reference every verdict change is measured against.
///
/// The knobs are read from the `options` object; the rest of the request is the
/// draft itself. Unknown top-level fields are refused instead of being silently
/// swallowed by the flattened Play shape.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub draft: PlayRequest,
    pub options: SearchOptions,
}

impl<'de> Deserialize<'de> for SearchRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("adversary request must be an object"))?;
        const PLAY_FIELDS: [&str; 4] = ["height", "network", "boxes", "tx"];
        for key in object.keys() {
            if !PLAY_FIELDS.contains(&key.as_str()) && key != "options" {
                return Err(serde::de::Error::custom(format!(
                    "unknown adversary request field `{key}`; put search knobs under `options`"
                )));
            }
        }
        let draft = serde_json::from_value(value.clone()).map_err(serde::de::Error::custom)?;
        let options = match object.get("options") {
            Some(options) => {
                serde_json::from_value(options.clone()).map_err(serde::de::Error::custom)?
            }
            None => SearchOptions::default(),
        };
        Ok(Self { draft, options })
    }
}

/// The aggregate answer. Neither variant is a security claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchVerdict {
    /// Some step under these operations changed at least one input's verdict
    /// against the draft that step mutated, in either direction, in a
    /// transaction that was well formed (its balances and construction held).
    /// A step whose own transaction was malformed is recorded and never
    /// counted here.
    Flipped,
    /// Nothing changed under the operations and caps that ran. Explicitly not
    /// "safe": the probe space is bounded, and the bounds are recorded.
    NoFlipUnderProbes,
}

/// One input's verdict before and after.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flip {
    pub box_id: String,
    pub before: String,
    pub after: String,
}

/// The unmutated draft's per-input verdicts: the reference every flip is
/// measured against, so a reader sees the comparison without re-running it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceInput {
    pub box_id: String,
    pub verdict: &'static str,
}

/// The caps a run actually used, next to the ceilings they were clamped to.
/// Load-bearing under truncation: which number bound the search is the
/// difference between "there was nothing there" and "the cap stopped us".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCaps {
    /// The resolved seed.
    pub seed: String,
    pub max_depth: usize,
    pub max_probes: usize,
    pub max_ops_per_step: usize,
    pub max_unspent: usize,
    pub max_frontier: usize,
    /// Per-family caps, and the wire limits they sit against.
    pub families: FamilyCaps,
    /// The operation kinds considered, in the order they were considered.
    pub op_kinds: Vec<&'static str>,
    /// The carry policy, in words.
    pub carry: &'static str,
}

/// The per-family bounds a run used, so a report says what it could not do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamilyCaps {
    pub max_depth_limit: usize,
    pub max_probes_limit: usize,
    pub max_ops_limit: usize,
    pub max_unspent_limit: usize,
    /// Arrangements offered per reorder axis.
    pub permutation_cap: usize,
    /// Tamper edits kept, in priority order.
    pub tamper_cap: usize,
    /// The most token slots one generated shift prepends.
    pub shift_by_max: usize,
    /// The wire's own cap on tokens per box, far above `shift_by_max`.
    pub max_box_tokens: usize,
}

/// One evaluated (or refused) step, in the order the search ran it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStep {
    /// 1-based position in the trace.
    pub index: usize,
    /// How many times outputs were carried forward to reach this draft; 1 is
    /// the drafted transaction itself.
    pub depth: usize,
    /// The operations this step proposed, in order.
    pub ops: Vec<AttackOp>,
    /// Canonical digest of the draft the operations were applied to: together
    /// with `ops`, this step's exact replay identity.
    pub draft_digest: String,
    /// `evaluated`, `refused` (the draft cannot accept one of the operations)
    /// or `invalid` (the reducer could not run the transaction at all). A
    /// refused or invalid step is a record, never a silent skip.
    pub status: &'static str,
    /// Whether every input script accepted. A well-formed step can still be
    /// `false` when a script refuses it; that refusal can still be a useful
    /// witness and is distinguished by `problems`.
    pub ok: bool,
    /// Inputs whose verdict differs from the parent draft this step mutated.
    /// Each carried draft is evaluated before mutation, so box ids align.
    pub flipped: Vec<Flip>,
    /// Conservation and construction problems Play reported.
    pub problems: Vec<String>,
    /// Why the step never ran, when it did not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// The first sequence found that changed a verdict, and the shortest one this
/// run could establish that still changes one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// The trace position of the step that flipped.
    pub step: usize,
    /// That step's depth.
    pub depth: usize,
    /// The sequence as found.
    pub ops: Vec<AttackOp>,
    /// The delta-debugged sequence. A trial is kept when *some* input's verdict
    /// still changes, not necessarily the one that changed first, so the
    /// reduced sequence's own flip is recorded in `flipped`. Equal to `ops`
    /// when nothing could be dropped.
    pub minimal_ops: Vec<AttackOp>,
    /// True when the probe budget ran out before a sequence no further split
    /// of which still flips. `minimal_ops` is then a shorter sequence, not a
    /// minimal one.
    pub shrink_truncated: bool,
    /// The input(s) the reduced sequence flips.
    pub flipped: Vec<Flip>,
    /// The reduced sequence's own evaluation against its step's draft. Feeding
    /// that draft and `minimalOps` back to [`crate::attack::apply_attack`]
    /// reproduces it.
    pub witness: AttackResult,
}

/// What a step could not be, counted by cause. A miss that cannot say why it
/// missed is a miss nobody can act on.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRejections {
    /// Steps whose operations the draft refused (an index past the input list,
    /// an order that is not a permutation, a token slot no box could carry).
    pub operations: usize,
    /// Steps the reducer could not build or run at all.
    pub invalid: usize,
    /// Steps refused for a script verdict (`fail` / `error`).
    pub script: usize,
    /// Steps refused because a script needed a key the draft supplies nowhere
    /// (`needsProof`).
    pub missing_key: usize,
    /// Steps refused because the mutation broke conservation. Such a step
    /// proves nothing about the contract: the mutation, not the script, stopped
    /// the spend.
    pub conservation: usize,
    /// Drafts the next step could not be built from — nothing spendable left.
    pub no_next_step: usize,
    /// Drafts the frontier width turned away.
    pub frontier_full: usize,
    /// Boxes the unspent cap dropped across the search.
    pub boxes_dropped: usize,
}

/// The search's answer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchReport {
    /// Synthetic by construction: caller-supplied boxes and context, and a
    /// seeded search over reconstructed transactions. `nodeValidated` is
    /// `false` here and in the witness — nothing in this module ran canonical
    /// transaction validation.
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    pub verdict: SearchVerdict,
    /// A one-line reading of the verdict, in the same register as the rest of
    /// the crate's observations.
    pub observation: &'static str,
    pub caps: SearchCaps,
    /// Canonical digest of the request under search, in full.
    pub draft_digest: String,
    /// The unmutated draft's per-input verdicts.
    pub reference: Vec<ReferenceInput>,
    /// Every call to the reducer: the baseline, each step, each shrink trial.
    pub probes: usize,
    /// Of those, the shrink trials.
    pub shrink_probes: usize,
    /// True when a cap stopped the search with work left. A truncated miss is
    /// weaker than a complete one, and says so.
    pub truncated: bool,
    /// The deepest step evaluated.
    pub depth_reached: usize,
    /// Every step, in order.
    pub steps: Vec<SearchStep>,
    /// The first witness found, delta-debugged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best: Option<SearchHit>,
    pub rejections: SearchRejections,
    /// Canonical digest of the seed, the resolved caps, the draft and the whole
    /// trace. Two searches that agree here ran the same probes in the same
    /// order on the same draft. A canonical JSON digest — not a node
    /// transaction id, and not a signature.
    pub fingerprint: String,
    /// Everything the caps and the search declined to do, in words.
    pub notes: Vec<String>,
}

/// Resolve the caller's knobs into the caps a run uses. Every value is clamped
/// into `1..=limit`.
fn resolve(o: &SearchOptions) -> SearchCaps {
    let clamp =
        |v: Option<usize>, default: usize, limit: usize| v.unwrap_or(default).clamp(1, limit);
    SearchCaps {
        seed: match o.seed.trim() {
            "" => DEFAULT_SEED.to_string(),
            s => s.to_string(),
        },
        max_depth: clamp(o.max_depth, DEFAULT_MAX_DEPTH, MAX_DEPTH_LIMIT),
        max_probes: clamp(o.max_probes, DEFAULT_MAX_PROBES, MAX_PROBES_LIMIT),
        max_ops_per_step: clamp(o.max_ops_per_step, DEFAULT_MAX_OPS_PER_STEP, MAX_OPS_LIMIT),
        max_unspent: clamp(o.max_unspent, DEFAULT_MAX_UNSPENT, MAX_UNSPENT_LIMIT),
        max_frontier: MAX_FRONTIER,
        families: FamilyCaps {
            max_depth_limit: MAX_DEPTH_LIMIT,
            max_probes_limit: MAX_PROBES_LIMIT,
            max_ops_limit: MAX_OPS_LIMIT,
            max_unspent_limit: MAX_UNSPENT_LIMIT,
            permutation_cap: PERMUTATION_CAP,
            tamper_cap: TAMPER_CAP,
            shift_by_max: SHIFT_BY_MAX,
            max_box_tokens: MAX_BOX_TOKENS,
        },
        op_kinds: OP_KINDS.to_vec(),
        carry: CARRY_POLICY,
    }
}

/// A counter-mode deterministic PRNG: `blake2b256(key ‖ counter)` over
/// `key = blake2b256(seed)`. The same digest the sandbox already uses for
/// synthetic ids, so the search adds no dependency and no platform entropy:
/// the seed is the only input, and the same seed draws the same operations
/// anywhere.
struct Prng {
    key: [u8; 32],
    counter: u64,
}

impl Prng {
    /// Key the stream on the seed under this module's own domain, the way
    /// [`crate::attack`]'s synthetic ids are: a seed string is caller-supplied,
    /// and two callers who pass the same string to two different generators must
    /// not get the same stream.
    fn new(seed: &str) -> Self {
        let material = format!("ergo-forge/adversary-search/v1/{seed}");
        Self {
            key: *ergo_primitives::digest::blake2b256(material.as_bytes()).as_bytes(),
            counter: 0,
        }
    }

    /// The next 32 bytes of the stream.
    fn block(&mut self) -> [u8; 32] {
        let mut material = [0u8; 40];
        material[..32].copy_from_slice(&self.key);
        material[32..].copy_from_slice(&self.counter.to_le_bytes());
        self.counter += 1;
        *ergo_primitives::digest::blake2b256(&material).as_bytes()
    }

    /// A uniform value in `0..n` by rejection sampling — no modulo bias, which
    /// matters when `n` is a permutation count like 6 or 24.
    fn below(&mut self, n: usize) -> Option<usize> {
        if n == 0 {
            return None;
        }
        let n = n as u64;
        let bound = u64::MAX - (u64::MAX % n);
        loop {
            let b = self.block();
            let v = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
            if v < bound {
                return Some((v % n) as usize);
            }
        }
    }
}

/// `count` distinct indices out of `0..n`, drawn without replacement by a
/// partial Fisher–Yates, so one sequence never applies the same edit twice.
fn sample_distinct(prng: &mut Prng, n: usize, count: usize) -> Vec<usize> {
    let mut pool: Vec<usize> = (0..n).collect();
    let take = count.min(n);
    let mut out = Vec::with_capacity(take);
    for i in 0..take {
        let j = i + prng.below(pool.len() - i).unwrap_or(0);
        pool.swap(i, j);
        out.push(pool[i]);
    }
    out
}

/// The probe budget. Every call to the reducer spends one, the unmutated draft
/// included, so a run costs at most `maxProbes` reductions and can report how
/// much of the budget it left.
struct Budget {
    total: usize,
    spent: usize,
    shrink_spent: usize,
}

impl Budget {
    fn new(total: usize) -> Self {
        Self {
            total,
            spent: 0,
            shrink_spent: 0,
        }
    }

    fn left(&self) -> usize {
        self.total.saturating_sub(self.spent)
    }

    /// One step's reduction.
    fn spend(&mut self) {
        self.spent += 1;
    }

    /// A shrink trial spends from the same budget as a step: a minimized
    /// witness is worth a reduction, but never one the steps did not earn.
    fn spend_shrink(&mut self) -> bool {
        if self.left() == 0 {
            return false;
        }
        self.spent += 1;
        self.shrink_spent += 1;
        true
    }
}

/// `n!`, when it fits the reportable range. A saturated result is reported as a
/// lower bound rather than a wrapped number.
fn arrangements(n: usize) -> Option<u128> {
    (1..=n as u128).try_fold(1u128, |acc, i| acc.checked_mul(i))
}

fn arrangement_note(n: usize, offered: usize) -> String {
    match arrangements(n) {
        Some(total) => format!(
            "the arrangement space of {n} inputs is {total} entries, the first {offered} were offered"
        ),
        None => format!(
            "the arrangement space of {n} inputs exceeds the reportable range (at least 2^128 entries); the first {offered} were offered"
        ),
    }
}

/// The box an input or data input names, matched case-insensitively the way
/// [`play::apply`] resolves one.
fn box_of<'a>(draft: &'a PlayRequest, id: &str) -> Option<&'a ScenarioBox> {
    draft.boxes.iter().find(|b| {
        b.box_id
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case(id))
            .unwrap_or(false)
    })
}

/// The operations a draft can accept, in the pinned order of [`OP_KINDS`].
/// Nothing here is random — the PRNG chooses among these — and everything here
/// is bounded: the arrangement axes come from the drain hunt's capped
/// enumeration, the tamper edits are kept in a fixed priority order, and what a
/// cap dropped is returned as a note rather than a silent omission.
fn candidates(draft: &PlayRequest, caps: &SearchCaps) -> (Vec<AttackOp>, Vec<String>) {
    let mut ops: Vec<AttackOp> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let n_inputs = draft.tx.inputs.len();
    let n_data = draft.tx.data_inputs.len();

    // reorderInputs. The enumeration's first entry is the draft's own order and
    // is dropped here: a step that changes nothing is not an experiment.
    if n_inputs >= 2 {
        let (orders, cut) = permutations(n_inputs, caps.families.permutation_cap + 1);
        ops.extend(
            orders
                .into_iter()
                .skip(1)
                .map(|order| AttackOp::ReorderInputs { order }),
        );
        if cut {
            notes.push(format!(
                "reorderInputs: {}",
                arrangement_note(n_inputs, caps.families.permutation_cap)
            ));
        }
    }

    // insertDecoy: one decoy per slot, shaped by the script that reads that
    // slot, plus one appended after the last input.
    for i in 0..n_inputs {
        ops.push(AttackOp::InsertDecoy {
            at_index: i,
            target_input: i,
        });
    }
    if n_inputs > 0 {
        ops.push(AttackOp::InsertDecoy {
            at_index: n_inputs,
            target_input: 0,
        });
    }

    // swapDataInput: one look-alike per data input.
    for index in 0..n_data {
        ops.push(AttackOp::SwapDataInput { index });
    }

    // reorderDataInputs: the same bounded enumeration, when there is an axis.
    if n_data >= 2 {
        let (orders, cut) = permutations(n_data, caps.families.permutation_cap + 1);
        ops.extend(
            orders
                .into_iter()
                .skip(1)
                .map(|order| AttackOp::ReorderDataInputs { order }),
        );
        if cut {
            notes.push(format!(
                "reorderDataInputs: arrangements capped at {}",
                caps.families.permutation_cap
            ));
        }
    }

    // shiftTokenIndices: the smallest token-index games, by one or two slots.
    for input in 0..n_inputs {
        for by in 1..=caps.families.shift_by_max {
            ops.push(AttackOp::ShiftTokenIndices { input, by });
        }
    }

    let (tampers, tamper_notes) = tamper_candidates(draft);
    for text in tamper_notes {
        note(&mut notes, text);
    }
    ops.extend(tampers);
    (ops, notes)
}

fn different_register_value(tv: &crate::scenario::TypedValue) -> Option<serde_json::Value> {
    Some(match tv.r#type.as_str() {
        "Boolean" => json!(!tv.value.as_bool()?),
        "Byte" | "Short" | "Int" | "Long" => {
            let current = tv
                .value
                .as_i64()
                .or_else(|| tv.value.as_str()?.parse::<i64>().ok())?;
            json!(if current == 0 { 1 } else { 0 })
        }
        "BigInt" => {
            let zero = tv
                .value
                .as_str()?
                .trim()
                .parse::<i128>()
                .is_ok_and(|n| n == 0);
            json!(if zero { "1" } else { "0" })
        }
        t if t.starts_with("Coll[") => {
            if tv.value.as_array().is_some_and(|v| v.is_empty()) || tv.value == json!("") {
                return None;
            }
            json!([])
        }
        _ => return None,
    })
}

/// The bounded edit list for `tamperField`, in priority order: what a spender
/// breaks first — a spent box's token amount, box values, output tokens, then
/// registers. Token amounts go up by one rather than to zero, because the wire
/// requires a strictly positive amount and a zeroed token is a mutation the box
/// builder refuses before any script runs. Registers are only edited where a
/// box already carries them, so a tamper cannot fail the dense-from-R4 register
/// invariant before it reaches the script, and each keeps its own type, because
/// a type mismatch would error the reducer before identity ever mattered.
fn tamper_candidates(draft: &PlayRequest) -> (Vec<AttackOp>, Vec<String>) {
    let mut edits: Vec<AttackOp> = Vec::new();
    let mut push = |target: &str, index: usize, field: &str, value: serde_json::Value| {
        edits.push(AttackOp::TamperField {
            target: target.to_string(),
            index,
            field: field.to_string(),
            value,
        });
    };
    for (i, input) in draft.tx.inputs.iter().enumerate() {
        for (t, token) in box_of(draft, &input.box_id)
            .map(|b| b.tokens.as_slice())
            .unwrap_or_default()
            .iter()
            .take(4)
            .enumerate()
        {
            push(
                "input",
                i,
                &format!("tokenAmount:{t}"),
                json!(token.amount.saturating_add(1)),
            );
        }
    }
    for i in 0..draft.tx.outputs.len() {
        push("output", i, "value", json!(0));
    }
    for i in 0..draft.tx.inputs.len() {
        push("input", i, "value", json!(0));
    }
    for (i, output) in draft.tx.outputs.iter().enumerate() {
        for (t, token) in output.tokens.iter().take(4).enumerate() {
            push(
                "output",
                i,
                &format!("tokenAmount:{t}"),
                json!(token.amount.saturating_add(1)),
            );
        }
    }
    for (i, output) in draft.tx.outputs.iter().enumerate() {
        for (name, tv) in &output.registers {
            let Some(value) = different_register_value(tv) else {
                continue;
            };
            push(
                "output",
                i,
                name,
                json!({ "type": tv.r#type, "value": value }),
            );
        }
    }
    for (i, input) in draft.tx.inputs.iter().enumerate() {
        for (name, tv) in box_of(draft, &input.box_id)
            .map(|b| &b.registers)
            .into_iter()
            .flatten()
        {
            let Some(value) = different_register_value(tv) else {
                continue;
            };
            push(
                "input",
                i,
                name,
                json!({ "type": tv.r#type, "value": value }),
            );
        }
    }

    let mut notes = Vec::new();
    if edits.len() > TAMPER_CAP {
        notes.push(format!(
            "tamperField: {} edits offered, the first {TAMPER_CAP} kept in priority order",
            edits.len()
        ));
        edits.truncate(TAMPER_CAP);
    }
    (edits, notes)
}

/// Sequences to sample at one draft: as many as the draft's candidates offer,
/// but no more than this draft's even share of the probes still unspent across
/// the levels still to come — so raising `maxProbes` widens the search instead
/// of saturating the first draft.
fn sequences_for(
    budget_left: usize,
    drafts_left: usize,
    levels_left: usize,
    candidates: usize,
) -> usize {
    candidates.min(
        budget_left
            .div_ceil(drafts_left.max(1) * levels_left.max(1))
            .max(1),
    )
}

/// `want` sequences of 1..=max_len operations, each a distinct draw from `ops`.
///
/// A sequence already drawn at this draft is not drawn again: the operations are
/// a pure function of the draft, so re-running one would spend a probe to
/// learn the same thing twice. A draw that collides is retried a few times and
/// then taken as it came, rather than reported as a hole in the search.
fn sample_sequences(
    ops: &[AttackOp],
    prng: &mut Prng,
    want: usize,
    max_len: usize,
) -> Vec<Vec<AttackOp>> {
    let mut out: Vec<Vec<AttackOp>> = Vec::with_capacity(want);
    let mut drawn: BTreeSet<Vec<usize>> = BTreeSet::new();
    for _ in 0..want {
        let mut last = Vec::new();
        let mut accepted = false;
        for _ in 0..DRAW_ATTEMPTS {
            let len = 1 + prng.below(max_len).unwrap_or(0);
            last = sample_distinct(prng, ops.len(), len);
            if drawn.insert(last.clone()) {
                accepted = true;
                break;
            }
        }
        if accepted && !last.is_empty() {
            out.push(last.into_iter().map(|i| ops[i].clone()).collect());
        }
    }
    out
}

/// The flipped entries of a per-input diff.
fn flips(diff: &[InputDiff]) -> Vec<Flip> {
    diff.iter()
        .filter(|d| d.changed)
        .map(|d| Flip {
            box_id: d.box_id.clone(),
            before: d.before.clone(),
            after: d.after.clone(),
        })
        .collect()
}

/// Evaluate one trial sequence: its result when it still changes a verdict in a
/// transaction that itself evaluated clean, `None` otherwise — a trial that
/// breaks the balances is not a reduction, exactly as it is not a witness.
/// `None` also covers a draft the trial cannot be applied to. One probe.
fn flips_under(draft: &PlayRequest, ops: &[AttackOp], parent: &PlayResult) -> Option<PlayResult> {
    let mut mutated = draft.clone();
    for op in ops {
        apply_op(&mut mutated, op).ok()?;
    }
    let result = play::apply(&mutated).ok()?;
    (result.problems.is_empty() && !flips(&diff_inputs(parent, &result)).is_empty())
        .then_some(result)
}

/// Delta debugging over a found sequence: drop a contiguous chunk, keep the
/// reduction if the remainder still changes a verdict against the unmutated
/// draft, halve the chunk when nothing could be dropped. Bounded by the probe
/// budget, and a budget that runs out is reported — the caller gets a shorter
/// sequence and is told it is not a minimal one.
fn ddmin(
    draft: &PlayRequest,
    ops: &[AttackOp],
    found: PlayResult,
    parent: &PlayResult,
    budget: &mut Budget,
) -> (Vec<AttackOp>, PlayResult, bool) {
    let mut best = ops.to_vec();
    let mut result = found;
    let mut granularity = 2usize;
    while best.len() > 1 {
        let chunk = best.len().div_ceil(granularity).max(1);
        let mut reduced = false;
        let mut start = 0usize;
        while start < best.len() {
            let end = (start + chunk).min(best.len());
            let mut trial = Vec::with_capacity(best.len() - (end - start));
            trial.extend_from_slice(&best[..start]);
            trial.extend_from_slice(&best[end..]);
            // The empty sequence is the unmutated draft, which by definition
            // does not flip: never spend a probe on it.
            if trial.is_empty() {
                start = end;
                continue;
            }
            if !budget.spend_shrink() {
                return (best, result, true);
            }
            if let Some(trial_result) = flips_under(draft, &trial, parent) {
                best = trial;
                result = trial_result;
                reduced = true;
                break;
            }
            start = end;
        }
        if reduced {
            granularity = granularity.saturating_sub(1).max(2);
        } else if granularity >= best.len() {
            break;
        } else {
            granularity = (granularity * 2).min(best.len());
        }
    }
    (best, result, false)
}

/// Why a step's transaction did not evaluate clean, in the drain hunt's
/// precedence: a step the reducer could not evaluate is never recorded as a
/// script refusal, and a conservation break outranks a script verdict, because
/// a mutation that broke the balances blinded the experiment rather than the
/// contract refusing. `None` means the step evaluated clean.
fn classify(result: &PlayResult) -> Option<SearchRejections> {
    let mut rejection = SearchRejections::default();
    if result
        .problems
        .iter()
        .any(|p| p.contains("not conserved") || p.contains("outputs carry"))
    {
        rejection.conservation += 1;
    } else if result.inputs.iter().any(|i| i.verdict == "needsProof") {
        rejection.missing_key += 1;
    } else if result
        .inputs
        .iter()
        .any(|i| matches!(i.verdict, "fail" | "error"))
    {
        rejection.script += 1;
    } else {
        return None;
    }
    Some(rejection)
}

/// Fold one step's rejection cause into the tally.
fn add(rejections: &mut SearchRejections, cause: SearchRejections) {
    rejections.conservation += cause.conservation;
    rejections.missing_key += cause.missing_key;
    rejections.script += cause.script;
}

/// Append a note unless it is already there: the same cap is hit at every level
/// of a search, and a report should say so once.
fn note(notes: &mut Vec<String>, text: String) {
    if !notes.contains(&text) {
        notes.push(text);
    }
}

/// "1 step" / "2 steps". A tally in a note is read by a person, and a singular
/// count wearing a plural noun is the kind of small wrongness that makes a
/// machine-written report read as machine-written.
fn plural(n: usize, noun: &str) -> String {
    let suffix = if n == 1 {
        ""
    } else if noun == "box" {
        "es"
    } else {
        "s"
    };
    format!("{n} {noun}{suffix}")
}

/// Search the draft for an operation sequence that changes a verdict.
///
/// Errors are marshalling only (a draft the reducer cannot build, a step whose
/// next draft cannot be built): every other outcome — including
/// [`SearchVerdict::NoFlipUnderProbes`] — is a report.
pub fn search(req: &SearchRequest) -> Result<SearchReport, SandboxError> {
    let caps = resolve(&req.options);
    let draft_value = serde_json::to_value(&req.draft)
        .map_err(|e| SandboxError::Scenario(format!("draft is not serialisable: {e}")))?;
    let draft_digest = crate::evidence::case::json_digest(&draft_value);
    let mut prng = Prng::new(&caps.seed);
    let mut budget = Budget::new(caps.max_probes);

    // The unmutated draft is the root reference reported to the caller. Each
    // later step is compared with the parent draft it actually mutated, since
    // carried outputs necessarily have different box identities.
    let baseline = play::apply(&req.draft)?;
    budget.spend();
    let reference: Vec<ReferenceInput> = baseline
        .inputs
        .iter()
        .map(|r| ReferenceInput {
            box_id: r.box_id.clone(),
            verdict: r.verdict,
        })
        .collect();

    let mut steps: Vec<SearchStep> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut rejections = SearchRejections::default();
    let mut best: Option<SearchHit> = None;
    let mut depth_reached = 0usize;
    let mut truncated = false;
    let mut exhausted = false;
    // Steps whose verdict changed but whose own transaction did not evaluate
    // clean: recorded, counted here, and never a witness.
    let mut ineligible = 0usize;
    let mut frontier: Vec<(PlayRequest, PlayResult)> = vec![(req.draft.clone(), baseline.clone())];

    'levels: for depth in 1..=caps.max_depth {
        let levels_left = caps.max_depth - depth + 1;
        let mut next: Vec<(PlayRequest, PlayResult)> = Vec::new();
        for (draft, parent) in &frontier {
            if budget.left() == 0 {
                exhausted = true;
                break 'levels;
            }
            let (ops_pool, pool_notes) = candidates(draft, &caps);
            for text in pool_notes {
                note(&mut notes, text);
            }
            if ops_pool.is_empty() {
                note(
                    &mut notes,
                    format!("depth {depth}: the draft accepts none of the operation kinds"),
                );
                continue;
            }
            let want = sequences_for(budget.left(), frontier.len(), levels_left, ops_pool.len());
            let step_digest = crate::evidence::case::json_digest(
                &serde_json::to_value(draft)
                    .map_err(|e| SandboxError::Scenario(format!("draft: {e}")))?,
            );
            for ops in sample_sequences(&ops_pool, &mut prng, want, caps.max_ops_per_step) {
                if budget.left() == 0 {
                    exhausted = true;
                    break 'levels;
                }
                let index = steps.len() + 1;
                let mut mutated = draft.clone();
                let mut refused = None;
                for op in &ops {
                    if let Err(e) = apply_op(&mut mutated, op) {
                        refused = Some(e.to_string());
                        break;
                    }
                }
                if let Some(why) = refused {
                    rejections.operations += 1;
                    steps.push(SearchStep {
                        index,
                        depth,
                        ops,
                        draft_digest: step_digest.clone(),
                        status: "refused",
                        ok: false,
                        flipped: Vec::new(),
                        problems: Vec::new(),
                        refused: Some(why),
                    });
                    continue;
                }
                budget.spend();
                let result = match play::apply(&mutated) {
                    Ok(result) => result,
                    Err(e) => {
                        rejections.invalid += 1;
                        steps.push(SearchStep {
                            index,
                            depth,
                            ops,
                            draft_digest: step_digest.clone(),
                            status: "invalid",
                            ok: false,
                            flipped: Vec::new(),
                            problems: Vec::new(),
                            refused: Some(e.to_string()),
                        });
                        continue;
                    }
                };
                let diff = diff_inputs(parent, &result);
                let flipped = flips(&diff);
                if let Some(cause) = classify(&result) {
                    add(&mut rejections, cause);
                }
                steps.push(SearchStep {
                    index,
                    depth,
                    ops: ops.clone(),
                    draft_digest: step_digest.clone(),
                    status: "evaluated",
                    ok: result.ok,
                    flipped: flipped.clone(),
                    problems: result.problems.clone(),
                    refused: None,
                });
                depth_reached = depth_reached.max(depth);

                // A step whose own transaction was not well formed is not
                // evidence: whatever changed, the mutation broke the balances
                // or the construction, not the contract. Its flip stays on the
                // step record, but it is neither the witness nor a next
                // position — a chain state an invalid transaction cannot produce
                // is not somewhere the next step can stand.
                let well_formed = result.problems.is_empty();
                if !well_formed && !flipped.is_empty() {
                    ineligible += 1;
                }
                if well_formed && !flipped.is_empty() {
                    // A verdict changed in a well-formed transaction. Delta-debug
                    // the sequence down to one that still does, then stop: the
                    // first witness in the pinned order is the reported one, and
                    // hunting on for a shorter sequence would be a different
                    // experiment than the one asked for.
                    let (minimal_ops, minimal_result, shrink_truncated) =
                        ddmin(draft, &ops, result, parent, &mut budget);
                    let witness_diff = diff_inputs(parent, &minimal_result);
                    let applied = minimal_ops.len();
                    best = Some(SearchHit {
                        step: index,
                        depth,
                        ops,
                        minimal_ops,
                        shrink_truncated,
                        flipped: flips(&witness_diff),
                        witness: AttackResult {
                            result: minimal_result,
                            applied,
                            diff: witness_diff,
                        },
                    });
                    truncated |= shrink_truncated;
                    break 'levels;
                }

                // Carry this step's outputs into the next step: a sequence that
                // only ever re-spends the drafted boxes cannot watch a position
                // move. The carried step's own transaction conserves by
                // construction, which is what lets a later move's verdict be
                // attributed to the move rather than to the arithmetic.
                if well_formed && depth < caps.max_depth {
                    match play::next_step_draft(&mutated, &result, caps.max_unspent)? {
                        Some(step) => {
                            if step.carried < step.available {
                                rejections.boxes_dropped += step.available - step.carried;
                                truncated = true;
                            }
                            if next.len() < caps.max_frontier {
                                if budget.left() == 0 {
                                    exhausted = true;
                                    break 'levels;
                                }
                                // The next level compares against this unmutated
                                // carried draft, judged like a step: inputs that
                                // lost their proofs in the carry still count as a
                                // parent, but broken balances do not.
                                budget.spend();
                                match play::apply(&step.request) {
                                    Ok(parent) if parent.problems.is_empty() => {
                                        next.push((step.request, parent));
                                    }
                                    Ok(parent) => {
                                        if let Some(cause) = classify(&parent) {
                                            add(&mut rejections, cause);
                                        } else {
                                            rejections.invalid += 1;
                                        }
                                    }
                                    Err(_) => rejections.invalid += 1,
                                }
                            } else {
                                rejections.frontier_full += 1;
                                truncated = true;
                            }
                        }
                        None => rejections.no_next_step += 1,
                    }
                }
                continue;
            }
        }
        if next.is_empty() {
            if depth < caps.max_depth {
                note(
                    &mut notes,
                    format!(
                        "the search stopped after depth {depth}: no draft could be carried further"
                    ),
                );
            }
            break;
        }
        frontier = next;
    }

    if exhausted {
        truncated = true;
        note(
            &mut notes,
            format!(
                "the probe cap of {} was spent in full with work still to do",
                caps.max_probes
            ),
        );
    }
    if best.is_some() {
        note(
            &mut notes,
            "stopped at the first verdict change found: the pinned order makes it the reported \
             witness, not the shortest of all witnesses"
                .to_string(),
        );
    }
    if ineligible > 0 {
        note(
            &mut notes,
            format!(
                "{} changed a verdict in a transaction that was not well formed: the flip is on \
                 the step record, the mutation rather than the contract is the reason, and none \
                 of them is the witness",
                plural(ineligible, "step")
            ),
        );
    }
    if rejections.operations > 0 {
        note(
            &mut notes,
            format!(
                "{}: each is a step record, not a skip",
                plural(rejections.operations, "refused operation sequence")
            ),
        );
    }
    for (count, text) in [
        (rejections.invalid, "the reducer could not run at all"),
        (rejections.conservation, "whose mutation broke conservation"),
        (rejections.missing_key, "a script refused for want of a key"),
        (rejections.script, "a script refused"),
        (
            rejections.no_next_step,
            "with nothing spendable left to carry",
        ),
    ] {
        if count > 0 {
            note(&mut notes, format!("{} {text}", plural(count, "step")));
        }
    }
    if rejections.boxes_dropped > 0 {
        note(
            &mut notes,
            format!(
                "the unspent cap of {} dropped {}",
                caps.max_unspent,
                plural(rejections.boxes_dropped, "box")
            ),
        );
    }
    if rejections.frontier_full > 0 {
        note(
            &mut notes,
            format!(
                "the frontier width of {} turned away {}",
                caps.max_frontier,
                plural(rejections.frontier_full, "draft")
            ),
        );
    }

    let fingerprint = crate::evidence::case::json_digest(&json!({
        "version": FINGERPRINT_VERSION,
        "caps": &caps,
        "draft": &draft_value,
        "reference": &reference,
        "probes": budget.spent,
        "shrinkProbes": budget.shrink_spent,
        "steps": &steps,
        "best": &best,
    }));
    Ok(SearchReport {
        claim: crate::claim::ClaimMetadata::legacy(
            "adversary-search",
            "caller-supplied boxes and context; a seeded search over reconstructed transactions, \
             replayed from a recorded trace",
        ),
        verdict: if best.is_some() {
            SearchVerdict::Flipped
        } else {
            SearchVerdict::NoFlipUnderProbes
        },
        observation: if best.is_some() {
            "a sequence of adversary operations changed which scripts accepted in a \
             reconstructed, well-formed transaction; canonical transaction validation has not run"
        } else {
            "no operation sequence under these caps changed a verdict: not under these probes, \
             not safe"
        },
        caps,
        draft_digest,
        reference,
        probes: budget.spent,
        shrink_probes: budget.shrink_spent,
        truncated,
        depth_reached,
        steps,
        best,
        rejections,
        fingerprint,
        notes,
    })
}
