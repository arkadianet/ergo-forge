//! One-time P04 fixture producer. Public, deliberately hypothetical test key.
//! Not run by gates; never regenerate a fixture to repair a failed gate.
use ergo_sandbox::evidence::{
    sign::{sign_owned_p2pk, OwnedDlogSecret},
    validate::ValidationRequest,
    wire::{CandidateSpec, CreationReference, WireBox, WireTransaction},
    CasePremises, EvidenceCase,
};
use ergo_ser::{
    address::build_p2pk_tree_bytes,
    input::{ContextExtension, Input, SpendingProof},
    sigma_value::SigmaValue,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use zeroize::Zeroizing;
fn main() {
    let root = Path::new("ergo-sandbox/tests/fixtures/evidence");
    let mut request: ValidationRequest = serde_json::from_slice(
        &fs::read(root.join("node-vectors/accepted-keyless-spend.fixture")).unwrap(),
    )
    .unwrap();
    let protocol =
        WireBox::from_record(request.case.premises().boxes.value().unwrap()[0].clone()).unwrap();
    let key = OwnedDlogSecret::from_bytes(Zeroizing::new([0x42; 32])).unwrap();
    let output = CandidateSpec {
        value: 2_000_000,
        ergo_tree: "10010101d17300".into(),
        creation_height: 100,
        tokens: vec![],
        registers: "00".into(),
    };
    let funding = WireBox::hypothetical(
        &CandidateSpec {
            value: 1_000_000,
            ergo_tree: hex::encode(build_p2pk_tree_bytes(&key.public_key()).unwrap()),
            ..output.clone()
        },
        &CreationReference {
            transaction_id: "22".repeat(32),
            index: 1,
        },
    )
    .unwrap();
    let inputs = vec![
        Input {
            box_id: protocol.node().box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![], ContextExtension::empty()).unwrap(),
        },
        Input {
            box_id: funding.node().box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![], extension(7)).unwrap(),
        },
    ];
    let wire = WireTransaction::build(inputs, vec![], &[output]).unwrap();
    request.case = wire
        .bind_case(
            &EvidenceCase::new(CasePremises::unspecified()).unwrap(),
            &[protocol, funding],
            &[],
        )
        .unwrap();
    request.transaction_bytes = hex::encode(wire.bytes());
    let accepted = sign_owned_p2pk(&request, 1, &key).unwrap();
    let mut files = vec![];
    fs::create_dir_all(root.join("proof-vectors")).unwrap();
    for (name, value) in [
        ("unsigned", serde_json::to_value(&request).unwrap()),
        ("signed", serde_json::to_value(accepted.request()).unwrap()),
    ] {
        let path = format!("proof-vectors/{name}.fixture");
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(root.join(&path), &bytes).unwrap();
        files.push(json!({"role":name,"path":path,"sha256":hex::encode(Sha256::digest(&bytes))}));
    }
    let report = accepted.report();
    let manifest = json!({"formatVersion":1,"nodeRevision":report["nodeRevision"],"cases":[{
        "id":"owned-p2pk-with-keyless-companion","family":"owned-funding-proof","sourceKind":"authored-hypothetical",
        "nodeRevision":report["nodeRevision"],"propertyVersion":"none-P04","claimStatus":"no-property-claim","publicationEligibility":"public-authored",
        "fundingInput":1,"publicKey":hex::encode(key.public_key()),"files":files,
        "expected":{"status":report["status"],"stage":report["stage"],"transactionId":report["transactionId"]}
    }]});
    fs::write(
        root.join("proof-manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    println!(
        "full node result: {} at {}; transactionId={}",
        report["status"], report["stage"], report["transactionId"]
    );
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
