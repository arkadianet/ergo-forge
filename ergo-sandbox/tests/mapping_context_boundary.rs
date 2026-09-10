//! M05 stop evidence: mandatory local execution does not authenticate a code digest.
//! This diagnostic is not the registered M05 capability target.
#[allow(dead_code)]
mod mapping_support;
use ergo_sandbox::{
    evidence::{
        validate::{validate, ValidationRequest},
        wire::{WireBox, WireTransaction},
    },
    map::{
        check_relation::StateDomain,
        relations::{Guard, Relation, RelationProposal, Subject},
    },
};
use ergo_ser::{
    sigma_type::SigmaType,
    sigma_value::{CollValue, SigmaValue},
};
use mapping_support::{read, root, sha};
use serde_json::{json, Value};

#[test]
fn mandatory_local_execution_does_not_authenticate_exact_code() {
    let manifest = read("manifest.json");
    assert_eq!(
        sha(&std::fs::read(root().join("manifest.json")).unwrap()),
        "7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f"
    );
    let answers = read("expected.json");
    assert_eq!(
        sha(&std::fs::read(root().join("expected.json")).unwrap()),
        "291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e"
    );
    let entry = manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "context_scope-positive")
        .unwrap();
    let raw = std::fs::read(root().join(entry["path"].as_str().unwrap())).unwrap();
    assert_eq!(sha(&raw), entry["sha256"]);
    let fixture: Value = serde_json::from_slice(&raw).unwrap();
    let answer = answers["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == fixture["id"])
        .unwrap();
    assert_eq!(answer["supported"], true);
    assert_eq!(answer["relation"]["kind"], "Execute");
    let claims_raw =
        std::fs::read(root().join("../../../../docs/mapping/m03-mapping-results.json")).unwrap();
    assert_eq!(
        sha(&claims_raw),
        "7c64e6d0a26c05720128d2dc4891e408d76bdbd6a7d5ff226e0bf4d9b9087ba6"
    );
    let claims: Value = serde_json::from_slice(&claims_raw).unwrap();
    let old = claims["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == fixture["id"])
        .unwrap();
    let claim: RelationProposal = serde_json::from_value(old["claim"].clone()).unwrap();
    let domain: StateDomain =
        serde_json::from_value(claim.premises.state_constraints.value().unwrap().clone()).unwrap();
    let subject = WireBox::from_record(claim.premises.self_box.value().unwrap().clone()).unwrap();
    assert_eq!(
        claim.subject,
        Subject::BoxId {
            hex: subject.id().unwrap()
        }
    );
    assert_eq!(claim.premises.guard, Guard::True);
    assert!(claim.premises.authentication_roots.is_empty());
    assert_eq!(
        claim.premises.root_bytes.value().unwrap(),
        fixture["treeHex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(subject.node().candidate.ergo_tree_bytes()),
        fixture["treeHex"]
    );
    let mut rows = vec![];
    let mut requests = vec![];
    let mut transactions = vec![];
    // Raw pinned wire Boolean true; Eq(HEIGHT, HEIGHT); raw Boolean false.
    for (id, bytes, accepted, hash) in [
        (
            "original",
            "7f",
            true,
            "0a875ce747ee45efcf2d7bbcd932388192fd14ed9b91f58523ffa4febcf31d8b",
        ),
        (
            "different-true-expression",
            "93a3a3",
            true,
            "0c1daddc6a9b2c6a96e4297ca036eed77f6b19f9b1264579f7af324b95f744da",
        ),
        (
            "false-expression",
            "80",
            false,
            "476e6d0b1136336ed078ded89ac56c8ce8dd33388e484e03601463ef6b90c119",
        ),
    ] {
        let raw = std::fs::read(root().join("m05-stop").join(format!("{id}.fixture"))).unwrap();
        assert_eq!(sha(&raw), hash);
        let request: ValidationRequest = serde_json::from_slice(&raw).unwrap();
        // All state-domain components agree with the original normalized P.
        let actual = StateDomain {
            block_context: request.block_context.value().unwrap().clone(),
            parameters: request.parameters.value().unwrap().clone(),
            network_rules: request.network_rules.value().unwrap().clone(),
            headers: request.headers.value().unwrap().clone(),
            prior_block_cost: *request.prior_block_cost.value().unwrap(),
            local_policy: request.local_policy.value().unwrap().clone(),
            positive_input_tokens: true,
        };
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(&domain).unwrap()
        );
        assert_eq!(
            request.case.premises().engine_revision,
            *claim.premises.node_revision.value().unwrap()
        );
        assert_eq!(domain.block_context.activated_script_version, 3);
        assert!(
            domain.block_context.height - subject.node().candidate.creation_height
                < domain.parameters.storage_period
        );
        let outcome = validate(&request);
        assert_eq!(outcome.is_ok(), accepted, "{id}: {outcome:?}");
        let report = match outcome {
            Ok(v) => {
                assert_eq!(&v.checked().resolved_inputs()[0], subject.node());
                assert!(v.checked().resolved_inputs().iter().all(|b| b
                    .candidate
                    .tokens
                    .iter()
                    .all(|t| t.amount > 0)));
                let tx = v.checked().transaction().clone();
                assert_eq!(
                    tx.inputs[0].spending_proof.extension().values.get(&1),
                    Some(&(
                        SigmaType::SColl(Box::new(SigmaType::SByte)),
                        SigmaValue::Coll(CollValue::Bytes(hex::decode(bytes).unwrap()))
                    ))
                );
                transactions.push(tx);
                v.report()
            }
            Err(f) => {
                assert_eq!(f.stage, "node-validation");
                assert!(f.detail.starts_with("ProofFailed { index: 0 }"));
                serde_json::to_value(f).unwrap()
            }
        };
        let code_digest = hex::encode(
            ergo_primitives::digest::blake2b256(&hex::decode(bytes).unwrap()).as_bytes(),
        );
        println!(
            "{id}: extension={bytes} blake2b256={code_digest} {}",
            if accepted {
                "node-accepted"
            } else {
                "node-rejected: ProofFailed { index: 0 }"
            }
        );
        rows.push(json!({"id":id,"extensionBytes":bytes,"blake2b256":code_digest,"accepted":accepted,"requestSha256":hash,"executionFingerprint":request.fingerprint(),"execution":report}));
        requests.push(request);
    }
    assert_ne!(rows[0]["blake2b256"], rows[1]["blake2b256"]);
    // Prove the counterexample changes only the extension, not the transaction's
    // subject, other inputs, outputs, proof bytes or any request premise.
    assert_eq!(
        transactions[1].inputs[0].spending_proof.proof,
        transactions[0].inputs[0].spending_proof.proof
    );
    transactions[1].inputs[0].spending_proof = transactions[0].inputs[0].spending_proof.clone();
    assert_eq!(transactions[0], transactions[1]);
    assert_eq!(
        hex::encode(
            WireTransaction::from_node(transactions[0].clone())
                .unwrap()
                .bytes()
        ),
        requests[0].transaction_bytes
    );
    for request in &requests[1..] {
        let mut normalized = request.clone();
        normalized.transaction_bytes = requests[0].transaction_bytes.clone();
        assert_eq!(
            serde_json::to_value(normalized).unwrap(),
            serde_json::to_value(&requests[0]).unwrap()
        );
    }
    let mut proposed = claim.clone();
    proposed.target = Relation::Execute {
        input: "0".into(),
        variable: 1,
        code_digest: rows[0]["blake2b256"].as_str().unwrap().into(),
    };
    proposed.reason = "M05 exact-digest interpretation tested against a supplied accepted counterexample; not established".into();
    let result = json!({"version":"m05-stop-evidence:v1","caseId":fixture["id"],"claim":claim,"claimDigest":claim.claim_digest(),"premiseDigest":claim.premises.digest(),"proposedExactDigestClaim":proposed,"executions":rows,"conclusion":"Same root, SELF, state and guard accept distinct code digests. Mandatory local execution alone cannot establish exact authenticated Execute(input,var,codeDigest)."});
    let path = root().join("../../../../docs/mapping/m05-stop-results.json");
    let raw = std::fs::read(&path).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&raw).unwrap(), result);
    println!("evidence {} sha256={}", path.display(), sha(&raw));
    println!("M05 coverage stop: pinned supported context_scope-positive lacks exact-code authentication; M00-M04 claims and all 67 expected action dispositions preserved");
}
