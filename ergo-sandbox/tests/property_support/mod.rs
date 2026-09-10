//! D00 fixture authentication and existing APIs only; no author-property evaluator.
use ergo_sandbox::evidence::{
    replay::{replay, ReplayBundle},
    validate::{validate, ValidationRequest},
    wire::WireBox,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf};
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/properties")
}
pub fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub fn read(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
}
pub const MANIFEST_SHA: &str = "72609b827694feca9fb78ae440afc69644abb29238d9fd48dd7fe3467658304f";
pub const ANSWER_SHA: &str = "311f160e08a8875de546263474fb10ec621e881b34afc28bb65323db32c7f319";
pub fn check(
    manifest: &[u8],
    answers: &[u8],
    load: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(Value, Value), String> {
    if sha(manifest) != MANIFEST_SHA || sha(answers) != ANSWER_SHA {
        return Err("inventory/answers byte pin changed".into());
    }
    let m: Value = serde_json::from_slice(manifest).map_err(|e| e.to_string())?;
    let e: Value = serde_json::from_slice(answers).map_err(|e| e.to_string())?;
    check_members(&m, &e)?;
    for row in m["cases"].as_array().unwrap() {
        let bytes = load(row["path"].as_str().ok_or("missing path")?)?;
        if sha(&bytes) != row["sha256"] {
            return Err("reference bytes changed".into());
        }
        let c: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if c["id"] != row["id"] || c["family"] != row["family"] {
            return Err("reference identity changed".into());
        }
        if sha(&serde_json::to_vec(&c["authoring"]["property"]).unwrap()) != row["propertySha256"] {
            return Err("property identity changed".into());
        }
    }
    for (path, key) in [
        ("transfer-registration.json", "transferSha256"),
        ("authored-inputs.json", "authoringSha256"),
    ] {
        if sha(&load(path)?) != m[key] {
            return Err(format!("{path} changed"));
        }
    }
    Ok((m, e))
}
pub fn check_members(m: &Value, e: &Value) -> Result<(), String> {
    let mut expected = BTreeSet::new();
    for family in ["reserve", "issuance", "continuation", "bounded_response"] {
        for disposition in ["violation", "control"] {
            for i in 1..=2 {
                expected.insert(format!("{family}_{disposition}_{i}"));
            }
        }
    }
    for id in [
        "missing_role",
        "ambiguous_role",
        "wrong_register_type",
        "arithmetic_overflow",
        "false_guard",
        "short_response_horizon",
        "unbounded_eventuality",
        "unsupported_dynamic_expression",
    ] {
        expected.insert(id.into());
    }
    for doc in [m, e] {
        let rows = doc["cases"].as_array().ok_or("missing cases")?;
        if rows.len() != 24 {
            return Err("missing inventory member".into());
        }
        let ids = rows
            .iter()
            .map(|r| r["id"].as_str().unwrap_or("").to_owned())
            .collect::<BTreeSet<_>>();
        if ids.len() != rows.len() {
            return Err("duplicate inventory member".into());
        }
        if ids != expected {
            return Err("unregistered inventory member".into());
        }
    }
    Ok(())
}
pub fn inventory() -> (Value, Value) {
    check(
        &std::fs::read(root().join("manifest.json")).unwrap(),
        &std::fs::read(root().join("expected.json")).unwrap(),
        |p| std::fs::read(root().join(p)).map_err(|e| e.to_string()),
    )
    .unwrap()
}
pub fn measure(m: &Value) -> Value {
    let mut cases = vec![];
    let mut count = 0;
    for row in m["cases"].as_array().unwrap() {
        let c = read(row["path"].as_str().unwrap());
        let mut references = vec![];
        for r in c["references"].as_array().unwrap() {
            let request: ValidationRequest = serde_json::from_value(r["request"].clone()).unwrap();
            // Call the full node adapter independently for EVERY supplied reference.
            let accepted = validate(&request)
                .unwrap_or_else(|e| panic!("{} step {}: {:?}", c["id"], r["index"], e));
            assert_eq!(accepted.report()["transactionId"], r["transactionId"]);
            assert!(accepted.report()["nodeValidated"].as_bool().unwrap());
            let bundle = ReplayBundle {
                format_version: 1,
                execution: request,
                property: serde_json::from_value(r["legacyProperty"].clone()).unwrap(),
            };
            let legacy = replay(&bundle); // Existing extraction API also freshly validates.
            assert_eq!(legacy["nodeValidated"], true);
            let checked = accepted.checked();
            let observations = json!({
                "inputs":checked.resolved_inputs().iter().map(|b|json!({"boxId":hex::encode(b.box_id().unwrap()),"value":b.candidate.value,"propositionHex":hex::encode(b.candidate.ergo_tree_bytes()),"tokens":b.candidate.tokens.iter().map(|t|json!({"id":hex::encode(t.token_id.as_bytes()),"amount":t.amount})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
                "outputs":r["outputBoxes"].as_array().unwrap().iter().map(|b| {let w=WireBox::from_record(serde_json::from_value(b.clone()).unwrap()).unwrap(); json!({"boxId":w.id().unwrap(),"value":w.node().candidate.value,"propositionHex":hex::encode(w.node().candidate.ergo_tree_bytes())})}).collect::<Vec<_>>()
            });
            references.push(json!({"index":r["index"],"node":accepted.report(),"legacyExtraction":legacy,"observations":observations,"linkedTraceChecked":false,"reachability":"assumed-state"}));
            count += 1;
        }
        cases.push(json!({"id":c["id"],"family":c["family"],"references":references,"newPropertyResult":null,"newPropertyProducer":"unsupported-not-implemented"}));
    }
    json!({"version":"property-legacy-measurement:v1","manifestSha256":MANIFEST_SHA,"builtInPropertyVersions":[ergo_sandbox::evidence::claim::PROPERTY_VERSION],"registeredSyntheticCases":24,"referenceTransactions":count,"nodeValidationCalls":count*2,"linkedTraceReplay":false,"evaluatorAgreement":null,"transferDenominator":null,"semanticMeasurement":"incomplete-independent-human-review-missing","discoveryCredit":0,"cases":cases})
}

/// Keep raw node DTOs in a byte-preserved .fixture file: the frozen decompiler
/// corpus recursively discovers ergoTree fields in JSON files. The public JSON
/// measurement index contains the exact statuses and an authenticated raw link.
pub fn legacy_summary(raw: &Value) -> Value {
    let mut summary = raw.clone();
    summary["raw"] = json!({"path":"legacy-results-raw.fixture","sha256":sha(&serde_json::to_vec_pretty(raw).unwrap())});
    summary["cases"] = json!(raw["cases"].as_array().unwrap().iter().map(|c| json!({
        "id":c["id"],"family":c["family"],"newPropertyResult":c["newPropertyResult"],"newPropertyProducer":c["newPropertyProducer"],
        "references":c["references"].as_array().unwrap().iter().map(|r|json!({
            "index":r["index"],"nodeStatus":r["node"]["status"],"transactionId":r["node"]["transactionId"],
            "legacyExtractionStatus":r["legacyExtraction"]["status"],"linkedTraceChecked":r["linkedTraceChecked"],"reachability":r["reachability"]
        })).collect::<Vec<_>>()
    })).collect::<Vec<_>>());
    summary
}
