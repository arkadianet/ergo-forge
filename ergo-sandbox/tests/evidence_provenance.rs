use ergo_sandbox::evidence::{CasePremises, EvidenceCase, Origin, Premise};
use ergo_sandbox::ingest::{ingest_source, IngestOptions};
use serde_json::{json, Value};

fn round_trip(case: &EvidenceCase) -> EvidenceCase {
    let value = serde_json::to_value(case).unwrap();
    let imported: EvidenceCase = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(value, serde_json::to_value(&imported).unwrap());
    assert_eq!(case.fingerprint(), imported.fingerprint());
    assert_eq!(value["nodeValidated"], false);
    assert_eq!(value["deploymentIdentity"], "unknown");
    imported
}
fn source_case(source: &str) -> EvidenceCase {
    ingest_source(source, &IngestOptions::default())
        .artifact
        .unwrap()
        .evidence_case()
        .unwrap()
        .clone()
}
fn policy() -> Value {
    let doc = include_str!("../../docs/ROADMAP.md");
    let block = doc
        .split("<!-- roadmap-policy:v1 -->")
        .nth(1)
        .unwrap()
        .split("```json")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    serde_json::from_str(block).unwrap()
}

#[test]
fn synthetic_binding_survives_export_and_import() {
    let mut options = IngestOptions::default();
    options.overrides.insert(
        "declared".into(),
        ergo_sandbox::ingest::Override::Type {
            r#type: "Long".into(),
        },
    );
    options.overrides.insert(
        "provided".into(),
        ergo_sandbox::ingest::Override::Value(
            serde_json::from_value(json!({"type":"Long","value":77})).unwrap(),
        ),
    );
    let result = ingest_source(
        "sigmaProp(HEIGHT > limit && SELF.value > declared && SELF.value > provided)",
        &options,
    );
    let mut artifact = result.artifact.unwrap();
    let case = round_trip(artifact.evidence_case().unwrap());
    assert_eq!(case, result.report.evidence_case);
    let bindings = &case.premises().constants.value().unwrap().bindings;
    for (name, mechanism, origin) in [
        ("limit", "inferred", Origin::Hypothetical),
        ("declared", "type-override", Origin::Hypothetical),
        ("provided", "value-override", Origin::CallerSupplied),
    ] {
        let binding = bindings.iter().find(|b| b.name == name).unwrap();
        assert_eq!(binding.mechanism, mechanism);
        assert_eq!(binding.origin, origin);
        assert!(!binding.typed_value.is_null());
    }
    let analysis = artifact.analyze().unwrap();
    let exported = serde_json::to_value(&analysis).unwrap();
    assert_eq!(exported["case"], serde_json::to_value(&case).unwrap());
    assert_eq!(
        exported["sourceDigestCheck"]["origin"],
        "independently-checked"
    );
    assert_eq!(
        exported["sourceDigestCheck"]["method"],
        "sha256-of-attached-source-only"
    );
    artifact.tree_bytes = source_case("sigmaProp(false)").target_bytes().unwrap();
    assert!(
        artifact.analyze().is_err(),
        "detached bytes must not inherit source/binding provenance"
    );
    let failed = ingest_source("sigmaProp(HEIGHT >", &options);
    assert!(failed.artifact.is_none());
    assert!(
        !failed
            .report
            .evidence_case
            .premises()
            .constants
            .value()
            .unwrap()
            .complete
    );
    round_trip(&failed.report.evidence_case);

    // Retain the final recorded rows and binding origins, including failures.
    // Source text and target bytes were not archived; do not invent them.
    let inventory: Value =
        serde_json::from_str(include_str!("../../docs/ingestion-lithos-results.json")).unwrap();
    let rows =
        ergo_sandbox::ingest::recorded_inventory(&inventory, "docs/ingestion-lithos-results.json")
            .unwrap();
    let thresholds = policy()["thresholds"].clone();
    assert_eq!(
        rows.len() as u64,
        thresholds["ingestRows"].as_u64().unwrap()
    );
    assert!(
        rows.iter()
            .filter(|r| r.measurement["status"] == "compiled")
            .count() as u64
            >= thresholds["ingestCompiledFloor"].as_u64().unwrap()
    );
    for (row, original) in rows
        .iter()
        .zip(inventory["after"]["contracts"].as_array().unwrap())
    {
        assert_eq!(&row.measurement, original);
        assert_eq!(row.binding_origin_status, "recorded");
        let bindings = row.case.premises().constants.value().unwrap();
        assert_eq!(bindings.complete, original["status"] == "compiled");
        assert_eq!(
            bindings.bindings.len(),
            original["bindings"].as_array().unwrap().len()
        );
        for (bound, recorded) in bindings
            .bindings
            .iter()
            .zip(original["bindings"].as_array().unwrap())
        {
            assert_eq!(bound.name, recorded["parameter"]);
            assert_eq!(bound.typed_value, recorded["bound"]);
            assert_eq!(
                serde_json::to_value(&bound.source_lines).unwrap(),
                recorded["lines"]
            );
            let (origin, mechanism) = match recorded["origin"].as_str().unwrap() {
                "inferred" => (Origin::Hypothetical, "inferred"),
                "type_override" => (Origin::Hypothetical, "type-override"),
                "value_override" => (Origin::CallerSupplied, "value-override"),
                _ => panic!("unknown recorded origin"),
            };
            assert_eq!(bound.origin, origin);
            assert_eq!(bound.mechanism, mechanism);
        }
        assert!(row.case.analyze().is_err());
        round_trip(&row.case);
    }
    let mut missing_origin = inventory.clone();
    missing_origin["after"]["contracts"][0]["bindings"][0]
        .as_object_mut()
        .unwrap()
        .remove("origin");
    assert!(ergo_sandbox::ingest::recorded_inventory(&missing_origin, "archive").is_err());
    let mut missing_row = inventory.clone();
    missing_row["after"]["contracts"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(ergo_sandbox::ingest::recorded_inventory(&missing_row, "archive").is_err());
    println!("recorded inventory: {} rows retained; recorded binding values/origins retained; missing source text/target bytes not invented", rows.len());
}

#[test]
fn structural_match_does_not_establish_deployment() {
    let a = source_case("sigmaProp(HEIGHT > 100)");
    let b = source_case("sigmaProp(HEIGHT > 200)");
    let comparison = ergo_sandbox::identity::match_cases(&a, &b).unwrap();
    let result = comparison.result_for(&a, &[&b]).unwrap();
    assert_eq!(
        result.verdict,
        ergo_sandbox::identity::MatchVerdict::SameProgramWithDifferingConstants
    );
    let record = serde_json::to_value(&comparison).unwrap();
    assert_eq!(record["deploymentIdentity"], "unknown");
    assert_eq!(record["nodeValidated"], false);
    assert_eq!(record["dependencies"][0], serde_json::to_value(&b).unwrap());
    assert!(comparison.result_for(&a, &[&a]).is_none());
    let same = a.compare(&a).unwrap();
    assert!(same.result_for(&a, &[&a]).unwrap().byte_identical);
    assert_eq!(
        serde_json::to_value(same).unwrap()["deploymentIdentity"],
        "unknown"
    );
    for (field, value) in [
        ("deploymentIdentity", json!("verified")),
        ("nodeValidated", json!(true)),
        ("formatVersion", json!(0)),
    ] {
        let mut forged = serde_json::to_value(&a).unwrap();
        forged[field] = value;
        assert!(serde_json::from_value::<EvidenceCase>(forged).is_err());
    }
    let mut forged = serde_json::to_value(&a).unwrap();
    forged["confirmed"] = json!(true);
    assert!(serde_json::from_value::<EvidenceCase>(forged).is_err());
}

#[test]
fn missing_registers_are_not_empty_registers() {
    let missing = json!({"boxId":"11".repeat(32),"ergoTree":"10010101d17300","value":1});
    let mut empty = missing.clone();
    empty["additionalRegisters"] = json!({});
    let mut null = missing.clone();
    null["additionalRegisters"] = Value::Null;
    let records = [missing.clone(), empty.clone(), null.clone()].map(|v| {
        ergo_sandbox::map::source::record_box(
            v,
            "fixture:box".into(),
            Some("record-revision".into()),
        )
        .unwrap()
    });
    assert!(matches!(records[0].registers(), Premise::Missing { .. }));
    assert_eq!(records[1].registers().value(), Some(&json!({})));
    assert!(matches!(records[2].registers(), Premise::Missing { .. }));
    let base = source_case("sigmaProp(true)");
    let mut fingerprints = std::collections::BTreeSet::new();
    for (record, original) in records.into_iter().zip([missing, empty, null]) {
        let mut p = base.premises().clone();
        p.boxes = Premise::supplied(vec![record]);
        p.context = Premise::Present {
            value: json!({"height":1,"message":"","headers":[],"defaultPreHeader":true}),
            origin: Origin::Hypothetical,
        };
        p.assumptions.insert(
            "unspentAtHeight".into(),
            Premise::missing("source recording does not establish UTXO membership"),
        );
        let case = round_trip(&EvidenceCase::new(p).unwrap());
        assert_eq!(
            case.premises().boxes.value().unwrap()[0].document(),
            &original
        );
        fingerprints.insert(case.fingerprint());
    }
    assert_eq!(fingerprints.len(), 3);
    let mut p = CasePremises::unspecified();
    p.context = Premise::Present {
        value: json!({}),
        origin: Origin::IndependentlyChecked,
    };
    assert!(
        EvidenceCase::new(p).is_err(),
        "no independent state verifier exists in P01"
    );
}

#[test]
fn changing_any_premise_invalidates_cached_evidence() {
    let base = source_case("sigmaProp(HEIGHT > limit)");
    let cached = base.analyze().unwrap();
    let original = serde_json::to_value(&base).unwrap();
    assert!(cached.result_for(&round_trip(&base), &[]).is_some());
    // Each top-level premise, plus nested provenance/assumptions, changes identity.
    type Edit = Box<dyn Fn(&mut Value)>;
    let mut edits: Vec<Edit> = vec![
        Box::new(|v| v["premises"]["engineRevision"] = json!("aa".repeat(20))),
        Box::new(|v| v["premises"]["targetBytes"]["value"] = json!("10010100d17300")),
        Box::new(|v| {
            v["premises"]["source"]["value"]["record"]["locator"] = json!("different-source")
        }),
        Box::new(|v| {
            v["premises"]["source"]["value"]["record"]["revision"] = json!("other-revision")
        }),
        Box::new(|v| {
            v["premises"]["constants"]["value"]["bindings"][0]["typedValue"]["value"] = json!(999)
        }),
        Box::new(|v| {
            v["premises"]["constants"]["value"]["bindings"][0]["origin"] = json!("caller-supplied")
        }),
        Box::new(|v| {
            v["premises"]["boxes"] = json!({"status":"present","value":[],"origin":"hypothetical"})
        }),
        Box::new(|v| {
            v["premises"]["context"] =
                json!({"status":"present","value":{"height":99},"origin":"caller-supplied"})
        }),
        Box::new(|v| {
            v["premises"]["assumptions"]["roles"] =
                json!({"status":"present","value":["protected"],"origin":"caller-supplied"})
        }),
        Box::new(|v| {
            v["premises"]["assumptions"]["protocolNfts"] =
                json!({"status":"present","value":["aa"],"origin":"hypothetical"})
        }),
    ];
    for key in ["budget", "objective", "network", "defaultMessage"] {
        edits.push(Box::new(
            move |v| {
                v["premises"]["assumptions"][key] =
                    json!({"status":"present","value":"changed","origin":"caller-supplied"})
            },
        ));
    }
    for edit in edits {
        let mut value = original.clone();
        edit(&mut value);
        let changed: EvidenceCase = serde_json::from_value(value).unwrap();
        assert_ne!(base.fingerprint(), changed.fingerprint());
        assert!(cached.result_for(&changed, &[]).is_none());
    }
    let mut tampered = original.clone();
    tampered["premises"]["source"]["value"]["text"]["value"] = json!("different source");
    assert!(serde_json::from_value::<EvidenceCase>(tampered).is_err());
    let mut lost = original;
    lost["premises"].as_object_mut().unwrap().remove("context");
    assert!(
        serde_json::from_value::<EvidenceCase>(lost).is_err(),
        "missing serialization fields cannot silently default"
    );
    println!("all case premises and comparison dependencies are bound; stale cache entries return no result");
}
