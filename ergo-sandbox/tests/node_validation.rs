use ergo_primitives::reader::VlqReader;
use ergo_sandbox::evidence::{
    validate::{validate, ValidationRequest},
    wire::WireTransaction,
    Premise,
};
use ergo_ser::{
    input::{ContextExtension, SpendingProof},
    transaction::read_transaction,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

fn policy() -> Value {
    let text = include_str!("../../docs/ROADMAP.md");
    serde_json::from_str(
        text.split("<!-- roadmap-policy:v1 -->")
            .nth(1)
            .unwrap()
            .split("```json")
            .nth(1)
            .unwrap()
            .split("```")
            .next()
            .unwrap(),
    )
    .unwrap()
}
/// Authenticate every fixture before executing any vector. Names and count
/// obligations are read from the governing policy, never duplicated here.
fn cases() -> Vec<(Value, ValidationRequest)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evidence");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["formatVersion"], 1);
    let rev = ergo_sandbox::evidence::validate::node_revision();
    assert_eq!(rev, ergo_sandbox::evidence::case::engine_revision());
    assert_eq!(manifest["nodeRevision"], rev);
    let rows = manifest["cases"].as_array().unwrap();
    let p = policy();
    assert!(rows.len() as u64 >= p["thresholds"]["nodeVectorCasesMin"].as_u64().unwrap());
    let required = p["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "P03")
        .unwrap()["caseIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let actual = rows
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(rows.len(), actual.len(), "duplicate IDs");
    assert_eq!(actual, required);
    let mut loaded = vec![];
    for row in rows {
        assert_eq!(row["nodeRevision"], rev);
        assert_eq!(row["propertyVersion"], "none-P03");
        assert_eq!(row["claimStatus"], "no-property-claim");
        assert_eq!(row["publicationEligibility"], "public-authored");
        assert_eq!(row["sourceKind"], "authored-full-node-vector");
        assert_eq!(row["family"], "transaction-pipeline");
        let files = row["files"].as_array().unwrap();
        assert_eq!(files.len(), 1, "one self-contained request per vector");
        let path = Path::new(files[0]["path"].as_str().unwrap());
        assert!(path.components().all(|c| matches!(c, Component::Normal(_))));
        let full = root.join(path);
        assert!(full
            .canonicalize()
            .unwrap()
            .starts_with(root.canonicalize().unwrap()));
        let bytes = std::fs::read(full).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            files[0]["sha256"],
            "{} hash changed",
            row["id"]
        );
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let request: ValidationRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            value,
            "no premise disappears during import"
        );
        assert_eq!(request.case.premises().engine_revision, rev);
        loaded.push((row.clone(), request));
    }
    loaded
}

#[test]
fn full_pipeline_matches_pinned_node_vectors() {
    for (row, request) in cases() {
        let expected = &row["expected"];
        match validate(&request) {
            Ok(accepted) => {
                let report = accepted.report();
                assert_eq!(report["status"], expected["status"], "{}", row["id"]);
                assert_eq!(report["stage"], expected["stage"]);
                assert_eq!(report["nodeValidated"], true);
                assert_eq!(report["pipelineInvoked"], true);
                assert_eq!(report["nodeRevision"], row["nodeRevision"]);
                assert_eq!(report["transactionId"], expected["transactionId"]);
                assert_eq!(report["totalBlockCost"], expected["totalBlockCost"]);
                assert_eq!(report["request"], serde_json::to_value(&request).unwrap());
                assert_eq!(report["requestFingerprint"], request.fingerprint());
                assert_eq!(
                    hex::encode(accepted.checked().tx_id()),
                    expected["transactionId"]
                );
            }
            Err(failure) => {
                assert_eq!(
                    failure.status,
                    expected["status"].as_str().unwrap(),
                    "{}: {}",
                    row["id"],
                    failure.detail
                );
                assert!(failure.pipeline_invoked);
                assert!(!failure.node_validated);
                assert_eq!(
                    failure.stage,
                    expected["stage"].as_str().unwrap(),
                    "{}: {}",
                    row["id"],
                    failure.detail
                );
                assert!(failure
                    .detail
                    .starts_with(expected["errorVariant"].as_str().unwrap()));
                assert!(
                    failure
                        .detail
                        .starts_with(&format!("{}: ", expected["detail"].as_str().unwrap())),
                    "node diagnostic differs from pinned full-pipeline result"
                );
                assert_eq!(
                    serde_json::to_value(failure.request).unwrap(),
                    serde_json::to_value(&request).unwrap()
                );
            }
        }
        if row["id"] == "storage-rent-acceptance" {
            let mut control = request.clone();
            let bytes = hex::decode(&control.transaction_bytes).unwrap();
            let mut tx = read_transaction(&mut VlqReader::new(&bytes)).unwrap();
            tx.inputs[0].spending_proof =
                SpendingProof::new(vec![], ContextExtension::empty()).unwrap();
            control.transaction_bytes =
                hex::encode(WireTransaction::from_node(tx).unwrap().bytes());
            let failed = validate(&control).unwrap_err();
            assert!(failed.pipeline_invoked);
            assert_eq!(
                failed.status, "node-rejected",
                "rent must be the accepting path for this false-script input"
            );
        }
        if row["id"] == "aggregate-cost-rejection" {
            let mut control = request.clone();
            control.prior_block_cost = Premise::supplied(0);
            assert!(
                validate(&control).is_ok(),
                "prior block cost must be carried into the node accumulator"
            );
        }
        if row["id"] == "reemission-rule-rejection" {
            let mut value = serde_json::to_value(&request).unwrap();
            value["networkRules"]["value"] = json!({"mode":"disabled","description":"explicit control","reason":"hypothetical network without reemission"});
            let control: ValidationRequest = serde_json::from_value(value).unwrap();
            assert!(
                validate(&control).is_ok(),
                "enabled rules must reach the node"
            );
            assert_ne!(control.fingerprint(), request.fingerprint());
        }
    }
}

#[test]
fn incomplete_context_is_not_node_acceptance() {
    let (_, base) = cases()
        .into_iter()
        .find(|(r, _)| r["id"] == "accepted-keyless-spend")
        .unwrap();
    let original = serde_json::to_value(&base).unwrap();
    for field in [
        "parameters",
        "networkRules",
        "blockContext",
        "headers",
        "localPolicy",
        "priorBlockCost",
    ] {
        let mut value = original.clone();
        value[field] = json!({"status":"missing","reason":"not supplied"});
        let request: ValidationRequest = serde_json::from_value(value).unwrap();
        let failure = validate(&request).unwrap_err();
        assert!(!failure.pipeline_invoked);
        assert!(!failure.node_validated);
        assert_eq!(failure.stage, "premises");
        let mut absent = original.clone();
        absent.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ValidationRequest>(absent).is_err());
    }
    let mut missing_activation = original.clone();
    missing_activation["blockContext"]["value"]
        .as_object_mut()
        .unwrap()
        .remove("activatedScriptVersion");
    assert!(serde_json::from_value::<ValidationRequest>(missing_activation).is_err());
    for (group, field) in [
        ("parameters", "minValuePerByte"),
        ("blockContext", "preHeaderVersion"),
    ] {
        let mut incomplete = original.clone();
        incomplete[group]["value"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<ValidationRequest>(incomplete).is_err());
    }
    let mut unsupported = original.clone();
    unsupported["case"]["premises"]["engineRevision"] = json!("aa".repeat(20));
    let request = serde_json::from_value(unsupported).unwrap();
    assert!(!validate(&request).unwrap_err().pipeline_invoked);
    let mut changed = original.clone();
    changed["parameters"]["origin"] = json!("independently-checked");
    let request = serde_json::from_value(changed).unwrap();
    assert!(!validate(&request).unwrap_err().pipeline_invoked);
    let mut conflict = original.clone();
    conflict["case"]["premises"]["context"] =
        json!({"status":"present","origin":"hypothetical","value":{"height":999}});
    let conflict: ValidationRequest = serde_json::from_value(conflict).unwrap();
    assert!(!validate(&conflict).unwrap_err().pipeline_invoked);
    let mut missing = base;
    let mut p = missing.case.premises().clone();
    p.boxes = Premise::missing("no supplied state");
    missing.case = EvidenceCase::new(p).unwrap();
    assert!(!validate(&missing).unwrap_err().pipeline_invoked);
}

use ergo_sandbox::evidence::EvidenceCase;
#[test]
fn accepted_execution_cannot_be_deserialized_or_fabricated() {
    // Compile-fail doctests on AcceptedExecution additionally exercise both
    // forbidden operations against the Rust compiler in the workspace gate.
    let (_, request) = cases()
        .into_iter()
        .find(|(r, _)| r["id"] == "accepted-keyless-spend")
        .unwrap();
    let accepted = validate(&request).unwrap();
    let stored = accepted.report();
    assert!(serde_json::from_value::<ValidationRequest>(stored.clone()).is_err());
    let mut graft = stored["request"].clone();
    graft["nodeValidated"] = json!(true);
    assert!(serde_json::from_value::<ValidationRequest>(graft).is_err());
    let imported: ValidationRequest = serde_json::from_value(stored["request"].clone()).unwrap();
    let fresh = validate(&imported).unwrap();
    assert_eq!(fresh.checked().tx_id(), accepted.checked().tx_id());
    let mut changed = stored["request"].clone();
    changed["case"]["premises"]["boxes"]["value"] = json!([]);
    let changed: ValidationRequest = serde_json::from_value(changed).unwrap();
    assert_ne!(changed.fingerprint(), request.fingerprint());
    let rejected = validate(&changed).unwrap_err();
    assert!(rejected.pipeline_invoked);
    assert_eq!(rejected.stage, "utxo-resolution");
    assert_eq!(
        stored["status"], "node-accepted",
        "stored JSON cannot change the new node result"
    );
}
