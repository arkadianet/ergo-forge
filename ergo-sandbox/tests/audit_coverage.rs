//! Availability projection: which review instruments the static run could see.
//!
//! Availability comes from the audit registry, so a name a document carries but
//! the registry does not run is never reported as an instrument that ran.

use ergo_sandbox::audit::coverage::{coverage, project, Availability, Coverage, Row};
use ergo_sandbox::audit::{audit as run_audit, registered_lints, triage, Audit, Completeness};
use ergo_sandbox::{compile_source, inspect, lift_tree, Finding, Lifted, Severity};
use ergo_ser::address::NetworkPrefix;
use serde_json::Value;
use std::collections::BTreeSet;

const CATALOGUE: &str = include_str!("../../docs/security/vectors.json");

const PARTIAL: Completeness = Completeness::Partial {
    raw_placeholders: 1,
    truncated: false,
};

fn lifted(src: &str) -> Lifted {
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    let tree = inspect::parse_tree(&bytes).expect("parse");
    lift_tree(&tree, true)
}

fn projected(src: &str) -> Coverage {
    coverage(&run_audit(&lifted(src)))
}

fn row<'a>(report: &'a Coverage, instrument: &str) -> &'a Row {
    report
        .row(instrument)
        .unwrap_or_else(|| panic!("no row for {instrument}"))
}

fn registered_instruments() -> Vec<String> {
    let catalogue: Value = serde_json::from_str(CATALOGUE).expect("catalogue json");
    let mut instruments: Vec<String> = catalogue["vectors"]
        .as_array()
        .expect("vectors")
        .iter()
        .flat_map(|v| {
            v["instrument"]
                .as_array()
                .expect("instrument list")
                .iter()
                .map(|i| i.as_str().expect("instrument name").to_string())
        })
        .collect();
    instruments.sort();
    instruments.dedup();
    instruments
}

/// The lint ids the embedded review catalogue names.
fn catalogue_lints() -> BTreeSet<String> {
    registered_instruments()
        .iter()
        .filter_map(|i| i.strip_prefix("lint:").map(str::to_owned))
        .collect()
}

/// The lint ids the audit registry actually runs.
fn registry() -> BTreeSet<String> {
    registered_lints().map(str::to_owned).collect()
}

/// The `lint:` ids the report projects.
fn projected_lints(report: &Coverage) -> BTreeSet<String> {
    report
        .rows
        .iter()
        .filter_map(|r| r.instrument.strip_prefix("lint:").map(str::to_owned))
        .collect()
}

fn available(report: &Coverage) -> Vec<&Row> {
    report
        .rows
        .iter()
        .filter(|r| matches!(r.availability, Availability::Ran | Availability::Partial))
        .collect()
}

fn audit_of(lint: &'static str, observations: usize) -> Audit {
    Audit {
        obligations: vec![],
        findings: (0..observations)
            .map(|i| Finding {
                triage: triage::Triage::default(),
                lint,
                severity: Severity::Low,
                node_id: 1 + i as u64,
                ir_id: Some(1),
                message: "synthetic".into(),
                snippet: "SELF".into(),
            })
            .collect(),
        completeness: Completeness::Complete,
    }
}

#[test]
fn every_catalogue_lint_id_is_carried_by_the_audit_registry() {
    let registered = registry();
    let catalogue = catalogue_lints();
    assert!(!registered.is_empty(), "the registry runs lints");
    assert!(!catalogue.is_empty(), "the catalogue names lints");
    let unknown: Vec<&String> = catalogue
        .iter()
        .filter(|lint| !registered.contains(*lint))
        .collect();
    assert!(
        unknown.is_empty(),
        "a catalogue lint the registry does not run can never be reported as available: {unknown:?}"
    );
}

#[test]
fn every_registered_instrument_gets_exactly_one_row() {
    let report = projected("sigmaProp(HEIGHT > 100)");
    let projected: Vec<&str> = report.rows.iter().map(|r| r.instrument.as_str()).collect();
    let mut sorted = projected.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(projected, sorted, "rows must be unique and ordered");
    for instrument in registered_instruments() {
        assert!(
            report.row(&instrument).is_some(),
            "catalogue instrument {instrument} is not projected"
        );
    }
}

#[test]
fn the_lint_rows_are_exactly_the_registry() {
    // A registered lint the catalogue does not name still ran, so it keeps a
    // row. An observation naming an unregistered id is the only way an extra
    // lint row appears, and it is covered separately below.
    for src in [
        "sigmaProp(HEIGHT > 100)",
        "sigmaProp(SELF.R4[Int].get > 5 && SELF.R5[Int].get > 6)",
    ] {
        assert_eq!(projected_lints(&projected(src)), registry(), "{src}");
    }
}

#[test]
fn a_complete_recovery_ran_every_registered_lint() {
    let report = projected("sigmaProp(HEIGHT > 100)");
    assert_eq!(report.completeness, "complete");
    let rows = available(&report);
    assert!(!rows.is_empty());
    for r in &rows {
        assert_eq!(r.availability, Availability::Ran, "{}", r.instrument);
    }
    assert_eq!(rows.len(), registry().len());
}

#[test]
fn an_incomplete_recovery_degrades_only_the_static_rows() {
    let mut degraded = lifted("sigmaProp(SELF.R4[Int].get > 5)");
    degraded.raw_placeholders = 1;
    let report = coverage(&run_audit(&degraded));
    assert_eq!(report.completeness, "partial");
    for r in &report.rows {
        let expected = if r.instrument.starts_with("lint:") {
            Availability::Partial
        } else {
            Availability::NotStatic
        };
        assert_eq!(r.availability, expected, "{}", r.instrument);
    }
    assert!(row(&report, "lint:unchecked-get")
        .note
        .contains("incomplete recovery"));
}

#[test]
fn only_a_registered_lint_is_ever_reported_as_available() {
    let registered = registry();
    let report = projected("sigmaProp(SELF.R4[Int].get > 5)");
    for r in available(&report) {
        let id = r.instrument.strip_prefix("lint:").expect("a lint row");
        assert!(
            registered.contains(id),
            "{} is available but the audit registry does not run it",
            r.instrument
        );
    }
}

#[test]
fn an_instrument_outside_the_registry_is_never_available() {
    for completeness in [Completeness::Complete, PARTIAL] {
        for observations in [0, 3] {
            let r = project("lint:not-a-registered-lint", observations, completeness);
            assert_eq!(r.availability, Availability::Unregistered);
            assert_eq!(r.observations, observations);
            assert!(r.note.contains("audit registry"), "{}", r.note);
        }
    }
}

#[test]
fn a_registered_lint_is_available_and_never_unregistered() {
    for id in registered_lints() {
        for (completeness, expected) in [
            (Completeness::Complete, Availability::Ran),
            (PARTIAL, Availability::Partial),
        ] {
            let r = project(&format!("lint:{id}"), 0, completeness);
            assert_eq!(r.availability, expected, "{id}");
        }
    }
}

#[test]
fn observations_are_counted_per_lint_and_zero_stays_a_non_conclusion() {
    let report = projected("sigmaProp(SELF.R4[Int].get > 5 && SELF.R5[Int].get > 6)");
    let fired = row(&report, "lint:unchecked-get");
    assert_eq!(fired.observations, 2);
    assert_eq!(fired.availability, Availability::Ran);
    let quiet = row(&report, "lint:upgrade-hook");
    assert_eq!(quiet.observations, 0);
    assert_eq!(quiet.availability, Availability::Ran);
    assert!(quiet.note.contains("proves nothing"));
    for r in &report.rows {
        if r.observations > 0 {
            assert!(
                !r.note.contains("proves nothing"),
                "an observation row must not read as an absence claim: {}",
                r.instrument
            );
        }
    }
}

#[test]
fn an_available_row_never_claims_the_lints_own_bound_was_exhaustive() {
    for src in ["sigmaProp(SELF.R4[Int].get > 5)", "sigmaProp(HEIGHT > 100)"] {
        for r in available(&projected(src)) {
            assert!(
                r.note.contains("bound"),
                "{} must disclose that the lint's own analysis bound still applies: {}",
                r.instrument,
                r.note
            );
        }
    }
}

#[test]
fn non_static_instruments_are_reported_as_needing_supplied_material() {
    let report = projected("sigmaProp(HEIGHT > 100)");
    let seen: Vec<&str> = report
        .rows
        .iter()
        .filter(|r| r.availability == Availability::NotStatic)
        .map(|r| r.instrument.as_str())
        .collect();
    assert_eq!(
        seen,
        vec!["drain", "hunt", "manual", "node-validated", "scenario"]
    );
    for r in &report.rows {
        if r.availability == Availability::NotStatic {
            assert_eq!(r.observations, 0, "{}", r.instrument);
        }
    }
}

#[test]
fn an_observation_naming_an_unregistered_lint_is_reported_as_unregistered() {
    let report = coverage(&audit_of("unregistered-lint", 2));
    let r = row(&report, "lint:unregistered-lint");
    assert_eq!(r.availability, Availability::Unregistered);
    assert_eq!(r.observations, 2);
    // The observations stay visible: a fired instrument is never dropped, but
    // the report does not claim an available instrument produced them.
    assert!(projected_lints(&report).contains("unregistered-lint"));
    let registered = available(&report);
    assert_eq!(registered.len(), registry().len());
}

#[test]
fn the_report_carries_no_score_share_or_verdict() {
    let report = coverage(&run_audit(&{
        let mut l = lifted("sigmaProp(SELF.R4[Int].get > 5)");
        l.raw_placeholders = 1;
        l
    }));
    let value = serde_json::to_value(&report).expect("serializable");
    let object = value.as_object().expect("object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "completeness",
            "method",
            "nodeValidated",
            "provenance",
            "rows"
        ]
    );
    assert_eq!(object["method"], "static-analysis");
    assert_eq!(object["nodeValidated"], false);
    // One stable lower-case spelling, never the enum's own serialization.
    assert_eq!(object["completeness"], "partial");
    let text = serde_json::to_string(&value).expect("serializable");
    for banned in [
        "%",
        "score",
        "verdict",
        "rank",
        "pass",
        "\"Complete\"",
        "\"Partial\"",
    ] {
        assert!(
            !text.contains(banned),
            "report text mentions {banned}: {text}"
        );
    }
    for r in &report.rows {
        let serialized = serde_json::to_value(r).expect("serializable");
        let keys: Vec<&str> = serialized
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            vec!["availability", "instrument", "note", "observations"]
        );
    }
}

#[test]
fn completeness_has_one_lower_case_wire_label() {
    assert_eq!(Completeness::Complete.label(), "complete");
    assert_eq!(PARTIAL.label(), "partial");
    assert_eq!(
        Completeness::Partial {
            raw_placeholders: 0,
            truncated: true
        }
        .label(),
        "partial"
    );
    let report = projected("sigmaProp(HEIGHT > 100)");
    let value = serde_json::to_value(&report).expect("serializable");
    assert_eq!(value["completeness"], "complete");
}

#[test]
fn the_projection_is_deterministic_for_one_audit() {
    let audit = run_audit(&lifted("sigmaProp(SELF.R4[Int].get > 5)"));
    let first = serde_json::to_string(&coverage(&audit)).expect("serializable");
    let second = serde_json::to_string(&coverage(&audit)).expect("serializable");
    assert_eq!(first, second);
    let other = coverage(&run_audit(&lifted("sigmaProp(HEIGHT > 100)")));
    assert_ne!(serde_json::to_string(&other).expect("serializable"), first);
}
