//! A **shadow model**: a bounded, independent, strictly weaker second opinion
//! on a result the pinned node engine already produced.
//!
//! # What this is, and what it refuses to be
//!
//! The honest way to second-guess a result is *not* to write a second
//! reducer. A second independent reading of ErgoScript semantics in this
//! workspace would be a third implementation of consensus, it would be
//! weaker than the pinned one by construction, and a disagreement would say
//! nothing about the chain. [`refusal`] states that refusal as a value so a
//! caller has to acknowledge it rather than infer it.
//!
//! What *is* available without duplicating consensus is a **consistency
//! model**: an independent re-derivation of a handful of *monotone
//! bookkeeping* relations between an aggregate claim and the probe records it
//! summarises. If a recorded result says "a sample spent without a proof" and
//! no probe passed, the record is wrong about itself — no script semantics
//! required. That is the whole capability, and [`ShadowReport::not_evaluated`]
//! names what it is not.
//!
//! # Two-way relations
//!
//! Each aggregate class is a *classification* of the probe records, so every
//! relation is checked in both directions. A class must have the evidence it
//! names (`movableByAnyone` needs a passing preserve sample), and it must not
//! carry evidence that names a different class (`movableByAnyone` may not have
//! a passing attacker sample). One direction alone accepts a record that
//! contradicts itself on the same axis: a `requiresProof` record that also
//! reports a pass is the clearest example, and no probe count or cost figure
//! makes it consistent. The relations are one line each in
//! [`ShadowModel::observe`], and [`AGGREGATE_CLASSES`] documents each one.
//!
//! # The five boundaries
//!
//! 1. **Never node-validated.** [`NodeValidation`] is uninhabited: no public
//!    field, no constructor, no `Default`, and no `Deserialize`, and nothing in
//!    this module returns one. A record that *claims* node validation is
//!    reported as a divergence ([`DivergenceKind::NodeValidationClaimed`]); the
//!    report's own `nodeValidated` field is a `bool` that is `false` by
//!    construction, and [`ShadowReport`] has no constructor that could set it
//!    otherwise.
//! 2. **Never an execution verdict.** [`AcceptedExecution`] and
//!    [`ConfirmedViolation`] are uninhabited for the same reason. The three
//!    possible [`Signal`]s are agreement, disagreement, and "not modelled" —
//!    there is no "safe", no "exploited", and no "safe to sign".
//! 3. **Never prose.** A recorded `observation` string is carried and length
//!    checked, and never parsed: [`ShadowReport::prose_used`] is always
//!    `false`. A result that can only be justified in words is not modelled.
//! 4. **Bounded.** Every count and every text field the model reads is capped
//!    by the constants below and by [`ShadowPolicy::max_text`], and every
//!    finding's `detail` is at most [`MAX_RESIDUAL_TEXT`] characters,
//!    ellipsis included. A record past a cap is a divergence
//!    ([`DivergenceKind::RecordTooLarge`]), not an invitation to spend more
//!    time on it.
//! 5. **One way in, one way out.** Only the two *inputs* — [`RecordedEval`]
//!    and [`ShadowPolicy`] — derive `Deserialize`. Every type this module
//!    *emits* ([`Refusal`], [`Signal`], [`DivergenceClass`],
//!    [`DivergenceKind`], [`Divergence`], [`ShadowReport`]) does not: their
//!    labels, rule ids and refusal wording are chosen here, and a report read
//!    back from JSON is not this module's opinion of any record. In particular
//!    there is no path by which a `nodeValidated: true` report enters this
//!    module's vocabulary at all.
//!
//! # Labels
//!
//! Every report is `static-analysis` / `preflight` / review-priority. This
//! module cannot mint the node's authority, so it does not name it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Most probes one record may carry before the model refuses to model it.
pub const MAX_OBSERVED_PROBES: usize = 64;
/// Most register reads one record may disclose.
pub const MAX_REGISTER_READS: usize = 32;
/// Most erroring register reads *one probe* may disclose. The instrument's
/// probe reads are a subset of the record's own disclosed reads, so a probe
/// claiming more than the record's cap is already past it.
pub const MAX_PROBE_ERRORING_READS: usize = 32;
/// Most residual propositions one record may carry.
pub const MAX_RESIDUALS: usize = 16;
/// Longest free-text field the model reads (prose is length-checked, not read).
pub const MAX_TEXT: usize = 512;
/// Longest one probe's `reduced_to` proposition may be.
pub const MAX_RESIDUAL_TEXT: usize = 256;

/// The only `method` this module's reports ever carry.
pub const METHOD: &str = "static-analysis";
/// The only `authority` this module's reports ever carry.
pub const AUTHORITY: &str = "preflight";
/// The only meaning its severities ever carry.
pub const SEVERITY_MEANING: &str = "review-priority";

/// Node validation, as a fact this workspace could establish. **Uninhabited by
/// construction**: this module has no constructor for it, so it cannot report
/// one, and any future attempt to add one is a reviewable diff. It carries no
/// `Deserialize` either, so a JSON document cannot bring one into existence
/// either — the only way out of the type is this module, and it has no way in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NodeValidation {
    _private: (),
}

/// A spending execution the node accepted. **Uninhabited by construction** —
/// see [`NodeValidation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AcceptedExecution {
    _private: (),
}

/// A declared property the node's acceptance violated. **Uninhabited by
/// construction** — see [`NodeValidation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConfirmedViolation {
    _private: (),
}

/// What a shadow model declines to do, as a value.
///
/// The capability this module refuses is *deciding whether a transaction is
/// valid*. Only the pinned node validator can answer that, and it answers it
/// in [`crate::evidence`] territory: a fresh accepted execution, checked
/// against node wire types, under a declared property. Everything here is
/// strictly weaker, and the refusal exists so a caller cannot mistake the
/// difference for an omission.
///
/// Serialised, never read back: the wording is this module's, and a
/// `Deserialize` here would let a caller mint a refusal that this module never
/// made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Refusal {
    /// The capability declined.
    pub refused: &'static str,
    /// Stable reason code, for a report that cites the refusal.
    pub reason: &'static str,
    /// Why an independent model here would be a defect, not a check.
    pub text: &'static str,
    /// The strongest thing this module can say instead.
    pub instead: &'static str,
}

/// The refusal, as a value. Read it, print it, cite it; there is no version of
/// this module that returns an execution verdict.
pub const fn refusal() -> Refusal {
    Refusal {
        refused: "transaction validity / execution acceptance / property violation",
        reason: "consensus-duplication",
        text: "a second independent reading of ErgoScript semantics in this workspace would \
               be a third implementation of consensus: weaker than the pinned engine by \
               construction, disagreeing with it for reasons that say nothing about the \
               chain, and reachable by anyone who edits it",
        instead: "bookkeeping consistency between a recorded aggregate and its probe records, \
                  reported as agreement, disagreement, or not-modelled",
    }
}

/// One recorded probe, as the model sees it: labels and counts only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRecord {
    /// Probe family label, opaque to the model beyond being non-empty.
    pub kind: String,
    /// Output shape label (`attacker` / `preserve` in the spend hunt).
    pub output: String,
    /// The recorded reduction verdict, as its wire token.
    pub verdict: String,
    /// Register reads that reached a value before the probe raised.
    #[serde(default)]
    pub erroring_reads: Vec<String>,
    /// The residual proposition, when the probe needed a proof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_to: Option<String>,
    /// Block-cost units the recorded run consumed.
    #[serde(default)]
    pub cost: u64,
    /// Whether the recorded run stopped on the cost limit.
    #[serde(default)]
    pub cost_exhausted: bool,
    /// Producer fields the model does not interpret. Retained so a wire
    /// rename is visible to the harness rather than silently dropped here.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A recorded result, reduced to the fields a consistency model may read.
///
/// `observation` is carried so a caller can see the record's own prose, and is
/// never interpreted (see [`ShadowReport::prose_used`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedEval {
    /// The producing instrument's own method label.
    pub method: String,
    /// The aggregate claim, as its wire token.
    pub verdict: String,
    /// Whether the record claims node validation. A `true` here is a
    /// divergence, never an upgrade for this module.
    #[serde(default)]
    pub node_validated: bool,
    /// Whether the record's SELF was generated rather than supplied.
    #[serde(default)]
    pub self_synthetic: bool,
    /// Whether the producing instrument recorded a truncation.
    #[serde(default)]
    pub truncated: bool,
    /// Register reads the producing instrument disclosed.
    #[serde(default)]
    pub register_reads: Vec<String>,
    /// Residual propositions the record carries.
    #[serde(default)]
    pub residuals: Vec<String>,
    /// The probe records the aggregate summarises.
    #[serde(default)]
    pub probes: Vec<ProbeRecord>,
    /// The record's own prose. Length-checked, never parsed.
    #[serde(default)]
    pub observation: String,
    /// Producer fields outside the model's vocabulary. The holdout harness
    /// cross-checks the fields it depends on before constructing this record.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Verdict tokens the model can interpret. A token outside this set is a
/// refusal, not a guess: the model reports [`DivergenceKind::UnknownToken`]
/// and leaves the rules that needed it unchecked.
pub const PROBE_VERDICTS: [&str; 6] = [
    "pass",
    "fail",
    "error",
    "needsProof",
    "proofAccepted",
    "proofRejected",
];

/// Aggregate classes the model can interpret, each with the probe evidence its
/// own documentation claims. A class outside this set is refused.
///
/// Each entry states a *two-way* relation, and the rule
/// `aggregate-supported-by-probes` enforces both halves: the evidence must be
/// present, and the evidence that would name a different class must be absent.
pub const AGGREGATE_CLASSES: [(&str, &str); 4] = [
    (
        "spendableByAnyone",
        "requires a passing attacker-shaped probe",
    ),
    (
        "movableByAnyone",
        "requires a passing preserve-shaped probe and no passing attacker probe",
    ),
    (
        "requiresProof",
        "requires at least one residual proposition and no passing probe of either shape",
    ),
    ("notUnderProbes", "requires that no probe passed"),
];

/// How far a record is from being modelled. Serialised, never read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Signal {
    /// Every applicable rule held, and every input was interpretable.
    Consistent,
    /// At least one rule was violated. The record contradicts itself.
    Divergent,
    /// No rule was violated, but at least one input was uninterpretable, so
    /// agreement is not a statement about the whole record.
    Underdetermined,
}

/// Whether a finding breaks a rule or blocks one. Serialised, never read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DivergenceClass {
    /// A documented relation between the aggregate and the probes is broken.
    Violation,
    /// The model declined to interpret an input, so a rule stayed unchecked.
    Refusal,
}

/// One finding, with a stable wire name. Serialised, never read back: a
/// finding is this module's conclusion about a record, and accepting one from a
/// document would be a conclusion nobody re-derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DivergenceKind {
    /// A capped field is past its bound; the record is not modelled.
    RecordTooLarge,
    /// A verdict or class token is outside the closed vocabulary.
    UnknownToken,
    /// The record has no method label, so its provenance is unestablished.
    UndisclosedProvenance,
    /// The record claims node validation, which this module cannot hold.
    NodeValidationClaimed,
    /// The aggregate names evidence the probe records do not contain.
    AggregateUnsupportedByProbes,
    /// A residual proposition no `needsProof` probe produced.
    ResidualWithoutProbe,
    /// A probe reports a register read the record does not disclose.
    HiddenRegisterRead,
    /// A synthetic SELF with a disclosed register read, and a probe that passed
    /// anyway: the record cannot be both.
    SyntheticSelfPassedWithRegisterRead,
    /// A probe passed on a budget it had already exhausted.
    PassOnExhaustedBudget,
    /// The observed probe count is not the one the policy pinned.
    ProbeCountDiffersFromPolicy,
    /// One output shape is missing, so the shape axis did not run.
    ShapeAxisUnpaired,
    /// A probe carries no family label, so it cannot be reviewed.
    UnlabelledProbe,
}

impl DivergenceKind {
    /// Whether this finding breaks a rule or blocks one.
    pub const fn class(self) -> DivergenceClass {
        match self {
            Self::RecordTooLarge
            | Self::UndisclosedProvenance
            | Self::NodeValidationClaimed
            | Self::AggregateUnsupportedByProbes
            | Self::ResidualWithoutProbe
            | Self::HiddenRegisterRead
            | Self::SyntheticSelfPassedWithRegisterRead
            | Self::PassOnExhaustedBudget
            | Self::ProbeCountDiffersFromPolicy
            | Self::ShapeAxisUnpaired
            | Self::UnlabelledProbe => DivergenceClass::Violation,
            Self::UnknownToken => DivergenceClass::Refusal,
        }
    }

    /// The stable wire name of this finding.
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::RecordTooLarge => "recordTooLarge",
            Self::UnknownToken => "unknownToken",
            Self::UndisclosedProvenance => "undisclosedProvenance",
            Self::NodeValidationClaimed => "nodeValidationClaimed",
            Self::AggregateUnsupportedByProbes => "aggregateUnsupportedByProbes",
            Self::ResidualWithoutProbe => "residualWithoutProbe",
            Self::HiddenRegisterRead => "hiddenRegisterRead",
            Self::SyntheticSelfPassedWithRegisterRead => "syntheticSelfPassedWithRegisterRead",
            Self::PassOnExhaustedBudget => "passOnExhaustedBudget",
            Self::ProbeCountDiffersFromPolicy => "probeCountDiffersFromPolicy",
            Self::ShapeAxisUnpaired => "shapeAxisUnpaired",
            Self::UnlabelledProbe => "unlabelledProbe",
        }
    }
}

/// One finding against one record. Serialised, never read back — see
/// [`DivergenceKind`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Divergence {
    /// What the model found.
    pub kind: DivergenceKind,
    /// Violation or refusal.
    pub class: DivergenceClass,
    /// The rule that produced it, as a stable id.
    pub rule: &'static str,
    /// Bounded detail: counts, tokens and capped field names, at most
    /// [`MAX_RESIDUAL_TEXT`] characters including the truncation marker. Never
    /// prose from the record.
    pub detail: String,
}

/// The namespace-declared requirements a caller adds on top of the built-in
/// relations. Each is opt-in: with [`ShadowPolicy::default`] the model checks
/// only its own relations, and reports every one of them as evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShadowPolicy {
    /// The probe count the namespace was measured under. A record with another
    /// count is a different measurement, and the model says so rather than
    /// comparing them.
    pub expected_probe_count: Option<usize>,
    /// Require both output shapes, so the shape axis is known to have run.
    pub require_paired_output_shapes: bool,
    /// Require every probe to carry a family label, so each is reviewable.
    pub require_probe_labels: bool,
    /// Longest free-text field the model reads. Capped at [`MAX_TEXT`], and
    /// floored at 1 by [`ShadowPolicy::max_text`]: a bound of zero would call
    /// every record "too large", including one that says nothing at all.
    pub max_text: usize,
}

impl ShadowPolicy {
    /// The text bound this policy actually enforces: at least 1, at most
    /// [`MAX_TEXT`]. Read this rather than [`ShadowPolicy::max_text`], so the
    /// model's stated bound and its applied bound cannot differ.
    pub const fn max_text(&self) -> usize {
        // `Ord::clamp` is not const on this toolchain, so the two ends are
        // spelled out. A policy may ask for a tighter bound than the cap, but
        // never for none at all.
        let floored = if self.max_text < 1 { 1 } else { self.max_text };
        if floored > MAX_TEXT {
            MAX_TEXT
        } else {
            floored
        }
    }
}

impl Default for ShadowPolicy {
    fn default() -> Self {
        Self {
            expected_probe_count: None,
            require_paired_output_shapes: false,
            require_probe_labels: false,
            max_text: MAX_TEXT,
        }
    }
}

/// What the model can never check, whatever the record says. Reported on every
/// [`ShadowReport`] so a reader of the report sees the boundary with it.
pub const NOT_EVALUATED: [&str; 6] = [
    "script-semantics",
    "transaction-validity",
    "deployment-identity",
    "cost-soundness",
    "positional-binding",
    "exhaustive-probe-coverage",
];

/// The result of modelling one record. Serialised, never read back: a report
/// is an output, and a document read back as one could claim
/// `nodeValidated: true` without anything here having re-derived it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowReport {
    /// What the report is about, as the caller named it.
    pub subject: String,
    /// [`METHOD`], always.
    pub method: &'static str,
    /// [`AUTHORITY`], always.
    pub authority: &'static str,
    /// Always `false`; see the module's boundary note.
    pub node_validated: bool,
    /// Always [`SEVERITY_MEANING`].
    pub severity_meaning: &'static str,
    /// Agreement, disagreement, or not-modelled.
    pub signal: Signal,
    /// Every finding, in rule order.
    pub divergences: Vec<Divergence>,
    /// Rule ids that ran to a conclusion.
    pub evaluated: Vec<&'static str>,
    /// Rule ids left unchecked because an input was uninterpretable.
    pub unchecked: Vec<&'static str>,
    /// What this model never checks, whatever the record says.
    pub not_evaluated: Vec<&'static str>,
    /// Always `false`: prose is length-checked, never parsed.
    pub prose_used: bool,
    /// The refusal this module carries on every report.
    pub refusal: Refusal,
}

/// The model. Stateless apart from its policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowModel {
    policy: ShadowPolicy,
}

impl ShadowModel {
    /// A model over the built-in relations, plus the caller's policy.
    pub const fn new(policy: ShadowPolicy) -> Self {
        Self { policy }
    }

    /// The policy this model checks.
    pub const fn policy(&self) -> &ShadowPolicy {
        &self.policy
    }

    /// Model one recorded result.
    pub fn observe(&self, subject: &str, record: &RecordedEval) -> ShadowReport {
        let mut out = Findings::default();
        let max_text = self.policy.max_text();

        // ── bounds, before any rule reads a field ──
        out.ran("record-size");
        // Every field a rule below reads or iterates is bounded here, so the
        // work this model does — and the size of the report it produces —
        // never depends on how much text a record carries.
        let long_text = |text: &str| text.chars().count() > max_text;
        let widest_probe_reads = record
            .probes
            .iter()
            .map(|p| p.erroring_reads.len())
            .max()
            .unwrap_or(0);
        let over_count = record.probes.len() > MAX_OBSERVED_PROBES
            || record.register_reads.len() > MAX_REGISTER_READS
            || record.residuals.len() > MAX_RESIDUALS
            || widest_probe_reads > MAX_PROBE_ERRORING_READS;
        let over_text = long_text(&record.observation)
            || long_text(&record.verdict)
            || long_text(&record.method)
            || record
                .register_reads
                .iter()
                .chain(record.residuals.iter())
                .any(|t| long_text(t))
            || record.probes.iter().any(|p| {
                long_text(&p.kind)
                    || long_text(&p.output)
                    || long_text(&p.verdict)
                    || p.reduced_to.as_deref().is_some_and(long_text)
                    || p.erroring_reads.iter().any(|r| long_text(r))
            });
        if over_count || over_text {
            out.add(
                DivergenceKind::RecordTooLarge,
                "record-size",
                trunc(&format!(
                    "probes={} reads={} probeReads={} residuals={} text<={max_text}",
                    record.probes.len(),
                    record.register_reads.len(),
                    widest_probe_reads,
                    record.residuals.len()
                )),
            );
        }
        out.ran("token-vocabulary");
        let class_known = AGGREGATE_CLASSES.iter().any(|(c, _)| *c == record.verdict);
        if !class_known {
            out.add(
                DivergenceKind::UnknownToken,
                "token-vocabulary",
                trunc(&format!(
                    "aggregate class `{}` is not in the vocabulary",
                    record.verdict
                )),
            );
        }
        let tokens_known = record
            .probes
            .iter()
            .all(|p| PROBE_VERDICTS.contains(&p.verdict.as_str()));
        if !tokens_known {
            out.add(
                DivergenceKind::UnknownToken,
                "token-vocabulary",
                trunc(&format!(
                    "{} of {} probe tokens are outside the vocabulary",
                    record
                        .probes
                        .iter()
                        .filter(|p| !PROBE_VERDICTS.contains(&p.verdict.as_str()))
                        .count(),
                    record.probes.len()
                )),
            );
        }

        // ── provenance and the node-validation boundary ──
        out.ran("provenance-disclosed");
        if record.method.trim().is_empty() {
            out.add(
                DivergenceKind::UndisclosedProvenance,
                "provenance-disclosed",
                "the record carries no method label".to_string(),
            );
        }
        out.ran("no-node-validation");
        if record.node_validated {
            out.add(
                DivergenceKind::NodeValidationClaimed,
                "no-node-validation",
                "the record claims node validation; this model cannot hold that fact".to_string(),
            );
        }

        if class_known && tokens_known {
            // ── the aggregate against its own probe records ──
            for rule in [
                "aggregate-supported-by-probes",
                "residual-has-a-probe",
                "erroring-reads-disclosed",
                "synthetic-self-erroring-reads",
                "no-pass-on-exhausted-budget",
            ] {
                out.ran(rule);
            }
            let passes = |output: &str| {
                record
                    .probes
                    .iter()
                    .any(|p| p.output == output && p.verdict == "pass")
            };
            let any_pass = record.probes.iter().any(|p| p.verdict == "pass");
            // Two-way: the class must have the evidence it names, and must not
            // carry evidence that names a different class. See `AGGREGATE_CLASSES`.
            let supported = match record.verdict.as_str() {
                // A passing attacker sample *is* this class, and no other class
                // tolerates one.
                "spendableByAnyone" => passes("attacker"),
                // The only-preserve class: a preserve sample passed and no
                // attacker sample did.
                "movableByAnyone" => passes("preserve") && !passes("attacker"),
                // Nothing passed, and at least one probe reduced to a
                // proposition. A pass of either shape contradicts the class
                // whatever the residuals say.
                "requiresProof" => !any_pass && !record.residuals.is_empty(),
                // The no-pass class. It says nothing about residuals, exactly
                // as the instrument's own priority order does not.
                "notUnderProbes" => !any_pass,
                // Unreachable: an unrecognised class is a refusal above, which
                // leaves this rule unchecked. Never report it as a violation.
                _ => true,
            };
            if !supported {
                out.add(
                    DivergenceKind::AggregateUnsupportedByProbes,
                    "aggregate-supported-by-probes",
                    trunc(&format!(
                        "`{}` over {} probes: attackerPass={} preservePass={} anyPass={} \
                         residuals={}",
                        record.verdict,
                        record.probes.len(),
                        passes("attacker"),
                        passes("preserve"),
                        any_pass,
                        record.residuals.len()
                    )),
                );
            }
            // Every residual must be a `needsProof` probe's own proposition.
            for residual in &record.residuals {
                let sourced = record.probes.iter().any(|p| {
                    p.verdict == "needsProof" && p.reduced_to.as_deref() == Some(residual.as_str())
                });
                if !sourced {
                    out.add(
                        DivergenceKind::ResidualWithoutProbe,
                        "residual-has-a-probe",
                        trunc(&format!("residual `{}` has no needsProof probe", residual)),
                    );
                }
            }
            // A read a probe reports must be one the record discloses.
            for probe in &record.probes {
                for read in &probe.erroring_reads {
                    if !record.register_reads.iter().any(|d| d == read) {
                        out.add(
                            DivergenceKind::HiddenRegisterRead,
                            "erroring-reads-disclosed",
                            trunc(&format!(
                                "probe `{}` reports an undisclosed read",
                                probe.kind
                            )),
                        );
                    }
                }
            }
            // A synthetic SELF has no registers, so every read of one errors.
            if record.self_synthetic
                && !record.register_reads.is_empty()
                && record.probes.iter().any(|p| p.verdict != "error")
            {
                out.add(
                    DivergenceKind::SyntheticSelfPassedWithRegisterRead,
                    "synthetic-self-erroring-reads",
                    trunc(&format!(
                        "synthetic SELF with {} disclosed reads and {} non-erroring probes",
                        record.register_reads.len(),
                        record
                            .probes
                            .iter()
                            .filter(|p| p.verdict != "error")
                            .count()
                    )),
                );
            }
            // A probe that ran out of budget did not pass.
            for probe in &record.probes {
                if probe.cost_exhausted && probe.verdict == "pass" {
                    out.add(
                        DivergenceKind::PassOnExhaustedBudget,
                        "no-pass-on-exhausted-budget",
                        trunc(&format!(
                            "probe `{}` passed at cost {}",
                            probe.kind, probe.cost
                        )),
                    );
                }
            }
        }

        // ── the caller's namespace policy ──
        if let Some(expected) = self.policy.expected_probe_count {
            out.ran("policy.probe-count");
            if record.probes.len() != expected {
                out.add(
                    DivergenceKind::ProbeCountDiffersFromPolicy,
                    "policy.probe-count",
                    trunc(&format!(
                        "observed {} probes, the policy pinned {expected}",
                        record.probes.len()
                    )),
                );
            }
        }
        if self.policy.require_paired_output_shapes {
            out.ran("policy.shape-pairing");
            let attacker = record.probes.iter().any(|p| p.output == "attacker");
            let preserve = record.probes.iter().any(|p| p.output == "preserve");
            if !(attacker && preserve) {
                out.add(
                    DivergenceKind::ShapeAxisUnpaired,
                    "policy.shape-pairing",
                    trunc(&format!("attacker={attacker} preserve={preserve}")),
                );
            }
        }
        if self.policy.require_probe_labels {
            out.ran("policy.probe-labels");
            for probe in &record.probes {
                if probe.kind.trim().is_empty() {
                    out.add(
                        DivergenceKind::UnlabelledProbe,
                        "policy.probe-labels",
                        "a probe carries no family label".to_string(),
                    );
                }
            }
        }

        let signal = if out.has(DivergenceClass::Violation) {
            Signal::Divergent
        } else if out.has(DivergenceClass::Refusal) {
            Signal::Underdetermined
        } else {
            Signal::Consistent
        };
        let mut not_evaluated: Vec<&'static str> = NOT_EVALUATED.to_vec();
        if record.truncated {
            not_evaluated.push("probe-coverage (the record discloses a truncation)");
        }
        ShadowReport {
            subject: subject.to_string(),
            method: METHOD,
            authority: AUTHORITY,
            node_validated: false,
            severity_meaning: SEVERITY_MEANING,
            signal,
            evaluated: out.evaluated,
            unchecked: out.unchecked,
            divergences: out.divergences,
            not_evaluated,
            prose_used: false,
            refusal: refusal(),
        }
    }
}

/// Rule bookkeeping, and the one place a rule declares that it could not run.
#[derive(Default)]
struct Findings {
    divergences: Vec<Divergence>,
    evaluated: Vec<&'static str>,
    unchecked: Vec<&'static str>,
}

impl Findings {
    fn add(&mut self, kind: DivergenceKind, rule: &'static str, detail: String) {
        let class = kind.class();
        self.divergences.push(Divergence {
            kind,
            class,
            rule,
            detail,
        });
        if class == DivergenceClass::Refusal {
            // A refused input leaves every relation that needed it unchecked;
            // the rules that did run are still reported as run.
            for held in [
                "aggregate-supported-by-probes",
                "residual-has-a-probe",
                "erroring-reads-disclosed",
                "synthetic-self-erroring-reads",
                "no-pass-on-exhausted-budget",
            ] {
                if !self.unchecked.contains(&held) {
                    self.unchecked.push(held);
                }
            }
        }
    }

    /// Record that a rule reached a conclusion, finding or not. A report that
    /// lists no evaluated rule has demonstrated nothing, and a rule that could
    /// not run must not appear here.
    fn ran(&mut self, rule: &'static str) {
        if !self.evaluated.contains(&rule) {
            self.evaluated.push(rule);
        }
    }

    fn has(&self, class: DivergenceClass) -> bool {
        self.divergences.iter().any(|d| d.class == class)
    }
}

/// Cap a detail string so a record cannot make a report unbounded. The cap
/// covers the truncation marker, so every detail this module emits is at most
/// [`MAX_RESIDUAL_TEXT`] characters.
fn trunc(text: &str) -> String {
    if text.chars().count() <= MAX_RESIDUAL_TEXT {
        return text.to_string();
    }
    let head: String = text.chars().take(MAX_RESIDUAL_TEXT - 1).collect();
    format!("{head}…")
}
