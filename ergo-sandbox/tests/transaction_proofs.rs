use ergo_primitives::reader::VlqReader;
use ergo_sandbox::evidence::{
    sign::{sign_owned_p2pk, OwnedDlogSecret, SigningFailure},
    validate::{validate, ValidationRequest},
    wire::WireTransaction,
    EvidenceCase, Premise,
};
use ergo_ser::{
    input::{ContextExtension, SpendingProof},
    sigma_value::SigmaValue,
    transaction::{read_transaction, Transaction},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
use zeroize::Zeroizing;

fn key() -> OwnedDlogSecret {
    // Published test-only key; all boxes/state are hypothetical. Never use for funds.
    OwnedDlogSecret::from_bytes(Zeroizing::new([0x42; 32])).unwrap()
}
fn fixtures() -> (ValidationRequest, ValidationRequest, Value) {
    let text = include_str!("../../docs/ROADMAP.md");
    let policy: Value = serde_json::from_str(
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
    .unwrap();
    let unit = policy["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "P04")
        .unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let path = repo.join(unit["fixtureManifest"].as_str().unwrap());
    let root = path.parent().unwrap();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(manifest["formatVersion"], 1);
    assert_eq!(
        manifest["nodeRevision"],
        ergo_sandbox::evidence::validate::node_revision()
    );
    let rows = manifest["cases"].as_array().unwrap();
    let ids = rows
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let required = unit["caseIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids, required);
    assert_eq!(rows.len(), ids.len());
    let mut loaded = vec![];
    // Verify every revision and file hash before executing any signing/replay.
    for row in rows {
        assert_eq!(row["nodeRevision"], manifest["nodeRevision"]);
        assert_eq!(row["family"], "owned-funding-proof");
        assert_eq!(row["sourceKind"], "authored-hypothetical");
        assert_eq!(row["propertyVersion"], "none-P04");
        assert_eq!(row["claimStatus"], "no-property-claim");
        assert_eq!(row["publicationEligibility"], "public-authored");
        let mut by_role = std::collections::BTreeMap::new();
        for file in row["files"].as_array().unwrap() {
            let relative = Path::new(file["path"].as_str().unwrap());
            assert!(relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))));
            let full = root.join(relative).canonicalize().unwrap();
            assert!(full.starts_with(root.canonicalize().unwrap()));
            let bytes = std::fs::read(full).unwrap();
            assert_eq!(hex::encode(Sha256::digest(&bytes)), file["sha256"]);
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            let request: ValidationRequest = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(&request).unwrap(), value);
            assert_eq!(
                request.case.premises().engine_revision,
                manifest["nodeRevision"]
            );
            assert!(by_role
                .insert(file["role"].as_str().unwrap(), request)
                .is_none());
        }
        let unsigned = by_role.remove("unsigned").unwrap();
        let signed = by_role.remove("signed").unwrap();
        assert!(by_role.is_empty());
        loaded.push((unsigned, signed, row.clone()));
    }
    assert_eq!(
        loaded.len(),
        1,
        "P04 owns one funding fixture, not a search corpus"
    );
    loaded.pop().unwrap()
}
fn transaction(r: &ValidationRequest) -> Transaction {
    read_transaction(&mut VlqReader::new(
        &hex::decode(&r.transaction_bytes).unwrap(),
    ))
    .unwrap()
}
// A changed transaction is a new experiment; do not keep an old wire attachment
// claiming the previous bytes. State/context/provenance are otherwise retained.
fn changed_request(r: &ValidationRequest, tx: Transaction) -> ValidationRequest {
    let mut result = r.clone();
    let wire = WireTransaction::from_node(tx).unwrap();
    result.transaction_bytes = hex::encode(wire.bytes());
    let mut p = result.case.premises().clone();
    p.assumptions.remove("wireTransaction");
    p.assumptions.remove("ownedFundingProof");
    result.case = EvidenceCase::new(p).unwrap();
    result
}

#[test]
fn owned_p2pk_funding_signs_actual_transaction() {
    let (request, _, row) = fixtures();
    let original = serde_json::to_value(&request).unwrap();
    let index = row["fundingInput"].as_u64().unwrap() as usize;
    let key = key();
    assert_eq!(hex::encode(key.public_key()), row["publicKey"]);
    let accepted = sign_owned_p2pk(&request, index, &key).unwrap();
    assert_eq!(serde_json::to_value(&request).unwrap(), original);
    let before = transaction(&request);
    let after = accepted.checked().transaction();
    assert!(!after.inputs[index].spending_proof.proof.is_empty());
    for (i, (a, b)) in before.inputs.iter().zip(&after.inputs).enumerate() {
        assert_eq!(a.box_id, b.box_id);
        assert_eq!(a.spending_proof.extension(), b.spending_proof.extension());
        if i != index {
            assert_eq!(a.spending_proof, b.spending_proof);
        }
    }
    let a = WireTransaction::from_node(before).unwrap();
    let b = WireTransaction::from_node(after.clone()).unwrap();
    assert_eq!(a.bytes_to_sign(), b.bytes_to_sign());
    assert_eq!(a.id(), b.id());
    assert_ne!(a.bytes(), b.bytes());
    let report = accepted.report();
    assert_eq!(report["status"], row["expected"]["status"]);
    assert_eq!(report["stage"], row["expected"]["stage"]);
    assert_eq!(report["transactionId"], row["expected"]["transactionId"]);
    assert_eq!(report["nodeValidated"], true);
    assert_eq!(report["request"]["case"]["deploymentIdentity"], "unknown");
    assert_eq!(
        accepted.request().case.premises().boxes,
        request.case.premises().boxes
    );
    let metadata = accepted.request().case.premises().assumptions["ownedFundingProof"]
        .value()
        .unwrap();
    assert_eq!(metadata["parentRequestFingerprint"], request.fingerprint());
    assert_eq!(
        metadata["previousWireTransaction"],
        serde_json::to_value(&request.case.premises().assumptions["wireTransaction"]).unwrap()
    );
    // A proof is not acceptance: incomplete context and node-rejected outputs
    // must not yield an AcceptedExecution from the signing API.
    let mut incomplete = request.clone();
    incomplete.parameters = Premise::missing("not supplied");
    assert!(matches!(
        sign_owned_p2pk(&incomplete, index, &key),
        Err(SigningFailure::Validation(_))
    ));
    let mut future = transaction(&request);
    future.output_candidates[0].creation_height = 101;
    let future = changed_request(&request, future);
    match sign_owned_p2pk(&future, index, &key).unwrap_err() {
        SigningFailure::Validation(e) => {
            assert!(e.pipeline_invoked);
            assert_eq!(e.stage, "output-heights");
        }
        e => panic!("expected full node rejection: {e:?}"),
    }
}

#[test]
fn changed_transaction_invalidates_proof() {
    let (request, _, row) = fixtures();
    let index = row["fundingInput"].as_u64().unwrap() as usize;
    let key = key();
    let accepted = sign_owned_p2pk(&request, index, &key).unwrap();
    for axis in ["outputs", "extension", "input-order"] {
        let mut tx = transaction(accepted.request());
        let mut funding = index;
        match axis {
            "outputs" => {
                let mut output = tx.output_candidates[0].clone();
                output.value /= 2;
                tx.output_candidates = vec![output.clone(), output];
            }
            "extension" => {
                tx.inputs[index].spending_proof =
                    SpendingProof::new(tx.inputs[index].spending_proof.proof.clone(), extension(8))
                        .unwrap();
            }
            _ => {
                tx.inputs.swap(0, index);
                funding = 0;
            }
        }
        let changed = changed_request(accepted.request(), tx.clone());
        let rejected = validate(&changed).unwrap_err();
        assert!(rejected.pipeline_invoked, "{axis}");
        assert_eq!(rejected.status, "node-rejected", "{axis}");
        assert!(
            rejected
                .detail
                .starts_with(&format!("ProofFailed {{ index: {funding} }}:")),
            "{axis}: {}",
            rejected.detail
        );
        tx.inputs[funding].spending_proof = SpendingProof::new(
            vec![],
            tx.inputs[funding].spending_proof.extension().clone(),
        )
        .unwrap();
        let fresh = changed_request(&changed, tx);
        assert!(
            sign_owned_p2pk(&fresh, funding, &key).is_ok(),
            "{axis}: mutation is otherwise spendable; new proof must pass"
        );
    }
}

#[test]
fn declared_public_key_without_proof_is_not_acceptance() {
    let (mut request, _, row) = fixtures();
    let index = row["fundingInput"].as_u64().unwrap() as usize;
    let mut p = request.case.premises().clone();
    p.assumptions.insert(
        "declaredOwnership".into(),
        Premise::supplied(json!({"role":"attacker","publicKey":row["publicKey"],"owned":true})),
    );
    request.case = EvidenceCase::new(p).unwrap();
    let rejected = validate(&request).unwrap_err();
    assert!(rejected.pipeline_invoked);
    assert_eq!(rejected.status, "node-rejected");
    let wrong = OwnedDlogSecret::from_bytes(Zeroizing::new([0x43; 32])).unwrap();
    assert!(matches!(
        sign_owned_p2pk(&request, index, &wrong),
        Err(SigningFailure::Material(_))
    ));
    assert!(
        sign_owned_p2pk(&request, 0, &key()).is_err(),
        "keyless input is not owned standard P2PK"
    );
    assert!(sign_owned_p2pk(&request, usize::MAX, &key()).is_err());
    assert!(OwnedDlogSecret::from_bytes(Zeroizing::new([0; 32])).is_err());
    assert!(OwnedDlogSecret::from_bytes(Zeroizing::new([0xff; 32])).is_err());
}

#[test]
fn replay_bundle_contains_no_secret() {
    let (request, imported, row) = fixtures();
    // First replay an already recorded valid proof without constructing a key.
    let replayed = validate(&imported).unwrap();
    assert_eq!(
        replayed.report()["transactionId"],
        row["expected"]["transactionId"]
    );
    assert_eq!(
        replayed.request().transaction_bytes,
        imported.transaction_bytes
    );
    let owned = key();
    assert_eq!(format!("{owned:?}"), "OwnedDlogSecret([REDACTED])");
    let signed = sign_owned_p2pk(
        &request,
        row["fundingInput"].as_u64().unwrap() as usize,
        &owned,
    )
    .unwrap();
    let text = serde_json::to_string(&signed.report()).unwrap();
    let failure = format!(
        "{:?}",
        sign_owned_p2pk(&request, usize::MAX, &owned).unwrap_err()
    );
    drop(owned);
    for s in [&text, &failure] {
        assert!(!s.contains(&"42".repeat(32)), "raw test scalar leaked");
        for field in ["\"secret\"", "\"secrets\"", "\"scalar\"", "\"privateKey\""] {
            assert!(!s.contains(field), "{field}");
        }
    }
    let report: Value = serde_json::from_str(&text).unwrap();
    assert!(serde_json::from_value::<ValidationRequest>(report.clone()).is_err());
    let request: ValidationRequest = serde_json::from_value(report["request"].clone()).unwrap();
    let accepted = validate(&request).unwrap();
    assert_eq!(accepted.report()["transactionId"], report["transactionId"]);
    assert_eq!(accepted.request().fingerprint(), request.fingerprint());
    // Existing proof bytes must be replayed, never silently replaced by signing.
    assert!(sign_owned_p2pk(
        &request,
        row["fundingInput"].as_u64().unwrap() as usize,
        &key()
    )
    .is_err());
}

fn extension(value: i32) -> ContextExtension {
    let mut e = ContextExtension::empty();
    e.values.insert(
        1,
        (
            ergo_ser::sigma_type::SigmaType::SInt,
            SigmaValue::Int(value),
        ),
    );
    e
}
