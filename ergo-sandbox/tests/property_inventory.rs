//! Frozen D00 inputs/answers and measured existing APIs. No property evaluator.
mod property_support;
use ergo_primitives::writer::VlqWriter;
use ergo_sandbox::evidence::{
    validate::{validate, ValidationRequest},
    wire::{CandidateSpec, WireBox, WireTransaction},
    EvidenceCase, Premise,
};
use property_support::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
const LEGACY_SHA: &str = "9a2031b9324ab62b9bb91f7cf6960aa8a00209148b1b2eca0c1dfbd6323ea451";
fn bytes(path: &str) -> Vec<u8> {
    std::fs::read(root().join(path)).unwrap()
}

#[test]
fn property_inventory_pins_24_cases_and_independent_answers() {
    let (m, e) = inventory();
    let disk = |p: &str| std::fs::read(root().join(p)).map_err(|e| e.to_string());
    let mut missing = m.clone();
    missing["cases"].as_array_mut().unwrap().pop();
    assert_eq!(
        check_members(&missing, &e).unwrap_err(),
        "missing inventory member"
    );
    let mut duplicate = m.clone();
    duplicate["cases"][1] = duplicate["cases"][0].clone();
    assert_eq!(
        check_members(&duplicate, &e).unwrap_err(),
        "duplicate inventory member"
    );
    let mut unknown = e.clone();
    unknown["cases"][0]["id"] = json!("renamed");
    assert_eq!(
        check_members(&m, &unknown).unwrap_err(),
        "unregistered inventory member"
    );
    let mut wrong = e.clone();
    wrong["cases"][0]["operands"]["left"] = json!(999);
    assert!(check(
        &bytes("manifest.json"),
        &serde_json::to_vec(&wrong).unwrap(),
        disk
    )
    .is_err());
    assert!(check(
        &serde_json::to_vec(&m).unwrap(),
        &bytes("expected.json"),
        disk
    )
    .is_err());
    assert!(
        check(&bytes("manifest.json"), &bytes("expected.json"), |p| {
            let mut b = disk(p)?;
            b.push(b' ');
            Ok(b)
        })
        .is_err()
    );
    assert!(
        check(&bytes("manifest.json"), &bytes("expected.json"), |_| Err(
            "missing fixture".into()
        ))
        .is_err()
    );
    let mut counts = BTreeMap::<String, usize>::new();
    for row in e["cases"].as_array().unwrap() {
        *counts
            .entry(row["expectedDisposition"].as_str().unwrap().into())
            .or_default() += 1;
        assert!(!row["explanation"].as_str().unwrap().is_empty());
        assert_eq!(row["reviewStatus"], "independent-human-review-missing");
        assert_eq!(row["expectedGuard"], row["id"] != "false_guard");
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("violated".into(), 8),
            ("holds-on-execution".into(), 8),
            ("unresolved".into(), 5),
            ("not-applicable".into(), 1),
            ("unsupported-property".into(), 2)
        ])
    );
    assert_eq!(
        m["denominators"],
        json!({"synthetic":24,"supported":16,"violations":8,"controls":8,"boundary":8,"transfer":null,"eligibleDiscovery":null})
    );
    let workspace = root().join("../../../..");
    for (path, digest) in m["baselineArtifacts"].as_object().unwrap() {
        assert_eq!(
            &json!(sha(&std::fs::read(workspace.join(path)).unwrap())),
            digest,
            "baseline {path}"
        );
    }
    assert_eq!(
        m["nodeRevision"],
        ergo_sandbox::evidence::validate::node_revision()
    );
    println!("manifest sha256={MANIFEST_SHA}; answers sha256={ANSWER_SHA}; 24 pinned rows: 8 authored violations, 8 controls, 8 boundaries; independent semantic review incomplete");
}

// Check canonical material against the independently authored input quantities,
// scripts, tokens and typed registers; never evaluate the new assertion here.
fn check_material(c: &Value, r: &Value, step: usize) {
    let request: ValidationRequest = serde_json::from_value(r["request"].clone()).unwrap();
    assert_eq!(serde_json::to_value(&request).unwrap(), r["request"]);
    let accepted = validate(&request).unwrap();
    let inputs = accepted.checked().resolved_inputs();
    let outputs = &accepted.checked().transaction().output_candidates;
    let author = &c["authoring"];
    let authored_inputs = if step == 0 {
        &author["inputs"]
    } else {
        &author["steps"][step - 1]
    };
    let authored_outputs = &author["steps"][step];
    let first_request: ValidationRequest =
        serde_json::from_value(c["references"][0]["request"].clone()).unwrap();
    let mint =
        WireBox::from_record(first_request.case.premises().boxes.value().unwrap()[0].clone())
            .unwrap()
            .id()
            .unwrap();
    for (actual, authored) in inputs
        .iter()
        .map(|b| &b.candidate)
        .zip(authored_inputs.as_array().unwrap())
        .chain(outputs.iter().zip(authored_outputs.as_array().unwrap()))
    {
        assert_eq!(actual.value, authored["value"].as_u64().unwrap());
        let source_id = authored["sourceId"].as_str().unwrap();
        assert_eq!(
            hex::encode(actual.ergo_tree_bytes()),
            c["treeHexBySource"][source_id]
        );
        let mut w = VlqWriter::new();
        w.put_u8(authored["registers"].as_array().unwrap().len() as u8);
        for register in authored["registers"].as_array().unwrap() {
            let (t, v) = ergo_sandbox::scenario::parse_typed_value(
                register["type"].as_str().unwrap(),
                &register["value"],
            )
            .unwrap();
            ergo_ser::sigma_value::write_constant(&mut w, &t, &v).unwrap();
        }
        let tokens = authored["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| ergo_sandbox::evidence::wire::TokenSpec {
                id: if t["id"] == "first-input-id" {
                    mint.clone()
                } else {
                    t["id"].as_str().unwrap().into()
                },
                amount: t["amount"].as_u64().unwrap(),
            })
            .collect();
        let expected = CandidateSpec {
            value: actual.value,
            ergo_tree: c["treeHexBySource"][source_id].as_str().unwrap().into(),
            creation_height: actual.creation_height,
            tokens,
            registers: hex::encode(w.result()),
        }
        .build()
        .unwrap();
        assert_eq!(actual, &expected, "authored input/output material differs");
    }
    assert_eq!(inputs.len(), authored_inputs.as_array().unwrap().len());
    assert_eq!(outputs.len(), authored_outputs.as_array().unwrap().len());
    let tx = WireTransaction::from_node(accepted.checked().transaction().clone()).unwrap();
    assert_eq!(hex::encode(tx.bytes()), request.transaction_bytes);
    assert_eq!(
        tx.output_boxes()
            .unwrap()
            .iter()
            .map(|b| serde_json::to_value(b.record()).unwrap())
            .collect::<Vec<_>>(),
        *r["outputBoxes"].as_array().unwrap()
    );
    // Real execution refusal: removing required input material cannot still accept.
    let mut bad = request.clone();
    let mut premises = bad.case.premises().clone();
    premises.boxes = Premise::supplied(vec![]);
    bad.case = EvidenceCase::new(premises).unwrap();
    assert!(validate(&bad).is_err());
}

#[test]
fn reference_executions_and_legacy_results_are_reproduced() {
    let (m, e) = inventory();
    for row in m["cases"].as_array().unwrap() {
        let c = read(row["path"].as_str().unwrap());
        let answer = e["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == c["id"])
            .unwrap();
        assert_eq!(
            answer["expectedBindings"],
            c["authoring"]["property"]["roles"]
        );
        for (name, source) in c["sources"].as_object().unwrap() {
            let compiled = ergo_sandbox::compile_source(
                source.as_str().unwrap(),
                0,
                ergo_ser::address::NetworkPrefix::Mainnet,
            )
            .unwrap();
            assert_eq!(hex::encode(compiled.tree_bytes), c["treeHexBySource"][name]);
        }
        assert_eq!(
            answer["expectedReferenceAcceptance"]
                .as_array()
                .unwrap()
                .len(),
            c["references"].as_array().unwrap().len()
        );
        for (step, r) in c["references"].as_array().unwrap().iter().enumerate() {
            check_material(&c, r, step);
            assert_eq!(answer["expectedReferenceAcceptance"][step], "accepted");
        }
    }
    assert_eq!(sha(&bytes("legacy-results-raw.fixture")), LEGACY_SHA);
    let measured = measure(&m);
    assert_eq!(
        measured,
        read("legacy-results-raw.fixture"),
        "actual existing API output drifted"
    );
    assert_eq!(legacy_summary(&measured), read("legacy-results.json"));
    assert_eq!(measured["referenceTransactions"], 35);
    let mut statuses = BTreeMap::<String, usize>::new();
    for c in measured["cases"].as_array().unwrap() {
        for r in c["references"].as_array().unwrap() {
            *statuses
                .entry(r["legacyExtraction"]["status"].as_str().unwrap().into())
                .or_default() += 1;
            assert_eq!(r["linkedTraceChecked"], false);
        }
    }
    println!("35 individually accepted references / 24 rows; legacy extraction statuses {statuses:?}; legacy-results sha256={LEGACY_SHA}; no linked-trace or new-property evaluation");
}

#[test]
fn transfer_registration_and_exposure_are_accounted() {
    let (m, _) = inventory();
    let t = read("transfer-registration.json");
    assert_eq!(
        sha(&bytes("transfer-registration.json")),
        m["transferSha256"]
    );
    assert_eq!(t["plannedCases"], 2);
    assert!(t["denominator"].is_null());
    assert!(t["cases"].as_array().unwrap().is_empty());
    assert!(t["reviewer"].is_null());
    assert_eq!(t["utilityPass"], false);
    assert_eq!(
        t["semanticMeasurement"],
        "incomplete-independent-review-missing"
    );
    assert!(!t["operatorExposure"].as_array().unwrap().is_empty());
    // The actual inventory checker must reject fabricated transfer/review credit.
    for (key, value) in [
        ("denominator", json!(2)),
        ("utilityPass", json!(true)),
        ("reviewer", json!("fabricated")),
        ("cases", json!([{"id":"synthetic-renamed-as-transfer"}])),
    ] {
        let mut bad = t.clone();
        bad[key] = value;
        assert!(
            check(&bytes("manifest.json"), &bytes("expected.json"), |p| {
                if p == "transfer-registration.json" {
                    Ok(serde_json::to_vec(&bad).unwrap())
                } else {
                    std::fs::read(root().join(p)).map_err(|e| e.to_string())
                }
            })
            .is_err()
        );
    }
    println!("transfer planned 2; denominator null; no accepted pair, independent reviewer or utility credit; checkpoint must cancel capabilities 2 and 3 for this arc if this registration remains");
}
