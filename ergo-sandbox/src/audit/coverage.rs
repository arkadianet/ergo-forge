//! Availability of review instruments over one audited contract.
//!
//! The projection is static and descriptive: one row per instrument this
//! module can name — the embedded review catalogue's instruments, the audit
//! registry's lints, and any lint an observation names — saying whether a
//! registered instrument could run over the recovered material and how many
//! observations the completed run recorded.
//!
//! Availability is the audit registry's statement, never the catalogue's: a
//! catalogue name the registry does not carry is reported as unregistered
//! rather than as an instrument that ran. `Ran` says a registered lint ran
//! over the recovered tree; it says nothing about what that lint's own
//! analysis bound covered, and it is not a clean bill of health. The report
//! carries no total, share, score, rank or verdict. A row without an
//! observation proves nothing, and an available row is not a clean contract.
use super::{is_registered_lint, registered_lints, Audit, Completeness};
use crate::claim::ClaimMetadata;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Whether a registered instrument could observe this material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    /// A registered lint ran over a fully recovered tree, so it saw every
    /// construct the lift produced. The lint's own analysis bound still
    /// applies to what it looked for; this says nothing about that bound.
    Ran,
    /// A registered lint ran over an incomplete recovery (raw placeholders or
    /// the depth ceiling), so what it did not record may be missing rather
    /// than absent.
    Partial,
    /// A document names this lint but the audit registry does not carry it, so
    /// no registered instrument could have run it. It is neither available nor
    /// a measured absence.
    Unregistered,
    /// Not a static instrument. It needs caller-supplied material this
    /// projection does not hold, so nothing is reported for it here.
    NotStatic,
}

/// One instrument's availability for one audited contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// Registered id: `lint:<lint-id>`, or a non-static instrument name.
    pub instrument: String,
    pub availability: Availability,
    /// Observations this static run recorded. Zero is not evidence of absence.
    pub observations: usize,
    /// The strongest statement this projection supports for the row.
    pub note: &'static str,
}

/// Availability projection of one completed static audit.
///
/// The rows are the whole report: no counts of rows, no comparison between
/// instruments, and nothing a caller could read as a completeness or safety
/// figure. `observations` is a raw per-instrument count, not a denominator or
/// a rate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    #[serde(flatten)]
    pub claim: ClaimMetadata,
    /// The recovery state as a stable lower-case label, `"complete"` or
    /// `"partial"`. The [`Completeness`] enum is not serialized here, so one
    /// response never spells the same recovery fact two conflicting ways.
    pub completeness: &'static str,
    pub rows: Vec<Row>,
}

impl Coverage {
    /// The row for a registered instrument id, if the run projected it.
    pub fn row(&self, instrument: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.instrument == instrument)
    }
}

#[derive(Deserialize)]
struct Catalogue {
    vectors: Vec<Vector>,
}

#[derive(Deserialize)]
struct Vector {
    instrument: Vec<String>,
}

/// Every instrument the embedded review catalogue registers, deduplicated and
/// ordered. Names only: whether a name is available is the registry's call.
fn catalogue_instruments() -> BTreeSet<String> {
    let catalogue: Catalogue =
        serde_json::from_str(include_str!("../../../docs/security/vectors.json"))
            .expect("the embedded review catalogue is valid JSON");
    catalogue
        .vectors
        .into_iter()
        .flat_map(|v| v.instrument)
        .collect()
}

fn note(availability: Availability, observations: usize) -> &'static str {
    match (availability, observations) {
        (Availability::Partial, 0) => {
            "Ran over an incomplete recovery and recorded nothing; the gap may be unrecovered, not absent, and the lint's own bound still applies."
        }
        (Availability::Partial, _) => {
            "Ran over an incomplete recovery; these observations may themselves be incomplete."
        }
        (Availability::Ran, 0) => {
            "Ran and recorded nothing; the lint's own bound still applies and its silence proves nothing."
        }
        (Availability::Ran, _) => {
            "Static observations for review, limited to what the lint's own bound examines; no defect, mechanism or safety conclusion."
        }
        (Availability::Unregistered, 0) => {
            "Named outside the audit registry, so this instrument did not run; nothing about the contract was examined by it and its silence proves nothing."
        }
        (Availability::Unregistered, _) => {
            "Observations name an id the audit registry does not carry, so no registered instrument could have produced them; this row claims no availability."
        }
        (Availability::NotStatic, _) => {
            "Requires caller-supplied material this projection does not hold; nothing is reported for it here."
        }
    }
}

/// Project one instrument's row from what a run recorded.
///
/// `instrument` is a registered id, `lint:<lint-id>`, or a non-static
/// instrument name. A `lint:` id the audit registry does not carry is reported
/// as [`Availability::Unregistered`] whatever the recovery state and whatever
/// was observed for it.
#[must_use]
pub fn project(instrument: &str, observations: usize, completeness: Completeness) -> Row {
    let lint = instrument.strip_prefix("lint:");
    let availability = match lint {
        Some(id) if !is_registered_lint(id) => Availability::Unregistered,
        Some(_) if completeness == Completeness::Complete => Availability::Ran,
        Some(_) => Availability::Partial,
        None => Availability::NotStatic,
    };
    Row {
        instrument: instrument.to_owned(),
        availability,
        observations,
        note: note(availability, observations),
    }
}

/// Project instrument availability from an audit that has already run.
///
/// Total: cannot fail. Every registered static instrument ran inside
/// [`super::audit`]; this reads that run and never re-runs a lint.
#[must_use]
pub fn coverage(audit: &Audit) -> Coverage {
    let mut observations: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in &audit.findings {
        *observations.entry(finding.lint).or_default() += 1;
    }
    // Three sets of names, and every row is attributed to the audit registry
    // rather than to the document that named it: what the catalogue registers,
    // what the registry runs (so a registered lint is never silently dropped
    // for lack of a catalogue row), and whatever an observation names (so a
    // fired instrument stays visible, marked unregistered if nothing claims it).
    let mut instruments = catalogue_instruments();
    for lint in registered_lints() {
        instruments.insert(format!("lint:{lint}"));
    }
    for lint in observations.keys() {
        instruments.insert(format!("lint:{lint}"));
    }
    let rows = instruments
        .into_iter()
        .map(|instrument| {
            let observed = instrument
                .strip_prefix("lint:")
                .and_then(|lint| observations.get(lint))
                .copied()
                .unwrap_or(0);
            project(&instrument, observed, audit.completeness)
        })
        .collect();
    Coverage {
        claim: ClaimMetadata::STATIC,
        completeness: audit.completeness.label(),
        rows,
    }
}
