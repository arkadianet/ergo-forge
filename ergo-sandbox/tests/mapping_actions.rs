//! M04 fixed-inventory action checks and fresh canonical omission replay.
#[allow(dead_code)]
mod mapping_support;
use ergo_sandbox::{
    evidence::{case::Premise, validate::ValidationRequest, wire::WireBox},
    map::{action::*, check_relation::StateDomain, necessity::propose, relations::*},
};
use mapping_support::{canonical_box, root, sha, wire_spec};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
fn pinned(path: &str, hash: &str) -> Value {
    let bytes = std::fs::read(root().join(path)).unwrap();
    assert_eq!(sha(&bytes), hash, "{path}");
    serde_json::from_slice(&bytes).unwrap()
}
fn inventory() -> Vec<(Value, Value, Requirement)> {
    let manifest = pinned(
        "manifest.json",
        "7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f",
    );
    let answers = pinned(
        "expected.json",
        "291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e",
    );
    let claims = pinned(
        "../../../../docs/mapping/m03-mapping-results.json",
        "7c64e6d0a26c05720128d2dc4891e408d76bdbd6a7d5ff226e0bf4d9b9087ba6",
    );
    assert_eq!(manifest["cases"].as_array().unwrap().len(), 32);
    assert_eq!(answers["cases"].as_array().unwrap().len(), 32);
    assert_eq!(claims["cases"].as_array().unwrap().len(), 32);
    let mut ids = BTreeSet::new();
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(answers["cases"].as_array().unwrap())
        .zip(claims["cases"].as_array().unwrap())
        .map(|((entry, answer), claim)| {
            let c = pinned(
                entry["path"].as_str().unwrap(),
                entry["sha256"].as_str().unwrap(),
            );
            assert_eq!(c["id"], answer["id"]);
            assert_eq!(c["id"], claim["id"]);
            assert!(ids.insert(c["id"].as_str().unwrap().to_owned()));
            let p: RelationProposal = serde_json::from_value(claim["claim"].clone()).unwrap();
            let d = propose(&p).unwrap();
            (
                c,
                answer.clone(),
                Requirement {
                    claim: p,
                    derivation: Some(d),
                },
            )
        })
        .collect()
}
fn boxes(c: &Value) -> BTreeMap<String, WireBox> {
    c["universe"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            let w = canonical_box(
                &wire_spec(&b["spec"]),
                b["reference"]["index"].as_u64().unwrap() as u16,
            );
            assert_eq!(w.id().unwrap(), b["boxId"]);
            assert_eq!(hex::encode(w.bytes().unwrap()), b["bytes"]);
            (b["name"].as_str().unwrap().into(), w)
        })
        .collect()
}
fn action(c: &Value, v: &Value, r: &Requirement) -> Action {
    let bs = boxes(c);
    let named = |key: &str| {
        v[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| NamedInput {
                name: n.as_str().unwrap().into(),
                material: bs[n.as_str().unwrap()].record().clone(),
            })
            .collect()
    };
    Action {
        version: ActionVersion::V1,
        id: format!(
            "{}/{}",
            c["id"].as_str().unwrap(),
            v["id"].as_str().unwrap()
        ),
        spending_inputs: named("inputs"),
        data_inputs: named("dataInputs"),
        outputs: v["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                let b = c["universe"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|b| b["name"] == *n)
                    .unwrap();
                wire_spec(&b["spec"])
            })
            .collect(),
        extensions: BTreeMap::from([("A".into(), v["contextVars"].clone())]),
        height: Some(v["height"].as_u64().unwrap() as u32),
        relation_ids: vec![r.claim.claim_digest()],
    }
}
fn disposition(action: Action, r: Requirement) -> Disposition {
    check_actions(&[action], &[r]).remove(0).disposition
}
fn evidence(path: &str, result: &Value) {
    let path = root().join("../../../../docs/mapping").join(path);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), *result);
    println!("evidence {} sha256={}", path.display(), sha(&bytes));
}
#[test]
fn alternative_action_sets_are_checked_separately() {
    let mut rows = vec![];
    let mut correct = 0;
    let mut deferred = 0;
    for (c, a, r) in inventory() {
        assert_eq!(
            c["vectors"].as_array().unwrap().len(),
            a["actions"].as_array().unwrap().len()
        );
        let actions: Vec<_> = c["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| action(&c, v, &r))
            .collect();
        let results = check_actions(&actions, std::slice::from_ref(&r));
        for ((input, result), expected) in actions
            .iter()
            .zip(results)
            .zip(a["actions"].as_array().unwrap())
        {
            let pending = a["supported"] == true && a["relation"]["kind"] == "Execute";
            if pending {
                assert_eq!(result.disposition, Disposition::Unresolved);
                deferred += 1;
            } else {
                assert_eq!(
                    serde_json::to_value(&result.disposition).unwrap(),
                    expected["necessityDisposition"],
                    "{}: {:?}",
                    input.id,
                    result
                );
                correct += 1;
            }
            rows.push(json!({"id":input.id,"expected":expected["necessityDisposition"],"deferredToM05":pending,"result":result}));
        }
    }
    assert_eq!((correct, deferred), (63, 4));
    evidence(
        "m04-action-results.json",
        &json!({"version":"m04-action-results:v1","implementationBase":"3f349fc6b970889346be20775db255ba74c9381a","nodeRevision":ergo_sandbox::evidence::case::engine_revision(),"manifestSha256":"7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f","expectedSha256":"291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e","cases":32,"actions":rows,"metrics":{"registered":67,"correctThroughM04":63,"eligibleThroughM04":63,"deferredToM05":4}}),
    );
    let (c, _, r) = inventory()
        .into_iter()
        .find(|(c, _, _)| c["id"] == "action_alternatives-positive")
        .unwrap();
    let mut a = action(&c, &c["vectors"][0], &r);
    a.height = None;
    assert_eq!(disposition(a.clone(), r.clone()), Disposition::Unresolved);
    a.height = Some(50);
    assert_eq!(disposition(a.clone(), r.clone()), Disposition::Unresolved);
    // A separately declared lower-height domain makes the B guard false.
    let mut lower = r.clone();
    let mut domain: StateDomain = serde_json::from_value(
        lower
            .claim
            .premises
            .state_constraints
            .value()
            .unwrap()
            .clone(),
    )
    .unwrap();
    domain.block_context.height = 50;
    lower.claim.premises.state_constraints =
        Premise::supplied(serde_json::to_value(domain).unwrap());
    lower.derivation = Some(propose(&lower.claim).unwrap());
    a.relation_ids = vec![lower.claim.claim_digest()];
    a.spending_inputs[1] = NamedInput {
        name: "C".into(),
        material: boxes(&c)["C"].record().clone(),
    };
    assert_eq!(
        disposition(a, lower),
        Disposition::SatisfiesDeclaredRequirements
    );
    println!("63/63 M04-eligible action dispositions; all 67 retained; 4 Execute expectations deferred to M05; alternatives never unioned");
}
#[test]
fn data_outputs_and_unrelated_inputs_cannot_satisfy_spend() {
    let (c, _, r) = inventory()
        .into_iter()
        .find(|(c, _, _)| c["id"] == "literal_spend-positive")
        .unwrap();
    let mut a = action(&c, &c["vectors"][1], &r);
    let bs = boxes(&c);
    a.data_inputs.push(NamedInput {
        name: "B".into(),
        material: bs["B"].record().clone(),
    });
    a.outputs.push(wire_spec(
        &c["universe"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "B")
            .unwrap()["spec"],
    ));
    assert_eq!(
        disposition(a.clone(), r.clone()),
        Disposition::ViolatesDeclaredRequirement
    );
    a.spending_inputs.push(a.spending_inputs[0].clone());
    assert_eq!(disposition(a, r.clone()), Disposition::Unresolved);
    let a = action(&c, &c["vectors"][0], &r);
    let mut missing = r.clone();
    missing.derivation = None;
    assert_eq!(disposition(a.clone(), missing), Disposition::Unresolved);
    let mut stale = r.clone();
    stale.derivation.as_mut().unwrap().anchors.clear();
    assert_eq!(disposition(a.clone(), stale), Disposition::Unresolved);
    let mut no_subject = a.clone();
    no_subject.spending_inputs.remove(0);
    no_subject.extensions.clear();
    assert_eq!(disposition(no_subject, r.clone()), Disposition::Unresolved);
    let mut unknown = a.clone();
    unknown.relation_ids.push("unknown".into());
    assert_eq!(disposition(unknown, r.clone()), Disposition::Unresolved);
    let mut raw = serde_json::to_value(&a).unwrap();
    raw["version"] = json!("action-check:v2");
    assert!(serde_json::from_value::<Action>(raw).is_err());
    let report = serde_json::to_value(check_actions(&[a], &[r])).unwrap();
    assert!(serde_json::from_value::<Vec<Action>>(report).is_err());
    let extra = pinned(
        "m03/supplemental-v2.fixture",
        "25b20fb46cdd45ee199e3847321419fef765505a0db8d61b0bbc0499c40adfda",
    );
    let claims = pinned(
        "../../../../docs/mapping/m03-mapping-results.json",
        "7c64e6d0a26c05720128d2dc4891e408d76bdbd6a7d5ff226e0bf4d9b9087ba6",
    );
    for (fixture, row) in extra["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(claims["supplementalCases"].as_array().unwrap())
    {
        let c = &fixture["case"];
        assert_eq!(c["id"], row["id"]);
        // M03 supplemental control action labels were never the M00 action
        // answer key and conflict with their unsupported claims. Preserve them;
        // only positive supplemental vectors add checks beyond the 67 actions.
        if fixture["answer"]["supported"] != true {
            continue;
        }
        let p: RelationProposal = serde_json::from_value(row["claim"].clone()).unwrap();
        let r = Requirement {
            derivation: Some(propose(&p).unwrap()),
            claim: p,
        };
        for (v, expected) in c["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .zip(fixture["answer"]["actions"].as_array().unwrap())
        {
            assert_eq!(
                serde_json::to_value(disposition(action(c, v, &r), r.clone())).unwrap(),
                expected["necessityDisposition"]
            );
        }
        if c["id"] == "m03-explicit-distinctness-positive" {
            let Relation::Spend {
                selector: Selector::TokenAt { id, index, amount },
            } = &r.claim.target
            else {
                panic!("pinned selector changed")
            };
            let bs = boxes(c);
            let token = &bs["A"].node().candidate.tokens[*index as usize];
            assert_eq!(hex::encode(token.token_id.as_bytes()), *id);
            assert!(token.amount >= *amount);
            let mut alone = action(c, &c["vectors"][0], &r);
            alone.spending_inputs.retain(|b| b.name == "A");
            assert_eq!(
                disposition(alone, r.clone()),
                Disposition::ViolatesDeclaredRequirement
            );
            let mut script_subject = r.clone();
            script_subject.claim.subject = Subject::Script {
                hex: script_subject
                    .claim
                    .premises
                    .root_bytes
                    .value()
                    .unwrap()
                    .clone(),
            };
            script_subject.derivation = Some(propose(&script_subject.claim).unwrap());
            assert_eq!(
                disposition(action(c, &c["vectors"][0], &script_subject), script_subject),
                Disposition::SatisfiesDeclaredRequirements
            );
        }
    }
    // The registered SELF-alias control's accepted omission must refute even
    // though SELF itself satisfies the selector (tested by the replay gate).
    println!("data/output/unrelated inputs cannot satisfy Spend; missing/duplicate subject, stale proof, unknown bindings and versions fail closed");
}
fn request(entry: &Value) -> ValidationRequest {
    serde_json::from_value(pinned(
        &format!("m03/{}", entry["path"].as_str().unwrap()),
        entry["sha256"].as_str().unwrap(),
    ))
    .unwrap()
}
fn witness(p: &RelationProposal, req: ValidationRequest) -> OmissionWitness {
    OmissionWitness {
        version: WitnessVersion::V1,
        claim_digest: p.claim_digest(),
        premise_digest: p.premises.digest(),
        subject_input_id: WireBox::from_record(p.premises.self_box.value().unwrap().clone())
            .unwrap()
            .id()
            .unwrap(),
        execution: req,
    }
}
#[test]
fn accepted_omission_refutes_only_matching_claim_and_premises() {
    let prior = pinned(
        "../../../../docs/mapping/m03-omission-results.json",
        "2b4dd32bcdc5d41c0974e3718383744e0f6c903eae4ce5e4048695ad40c42ae1",
    );
    let manifest = pinned(
        "m03/manifest.json",
        "cdf73504cc3d5f98141bac9e44b46621a76baf8ac5219826d03367d171383853",
    );
    let extra = pinned(
        "m03/supplemental-v2.fixture",
        "25b20fb46cdd45ee199e3847321419fef765505a0db8d61b0bbc0499c40adfda",
    );
    let entries: Vec<_> = manifest["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .chain(extra["vectors"].as_array().unwrap())
        .collect();
    let mut rows = vec![];
    for old in prior.as_array().unwrap() {
        let p: RelationProposal = serde_json::from_value(old["claim"].clone()).unwrap();
        let entry = entries.iter().find(|e| e["id"] == old["vector"]).unwrap();
        let w = witness(&p, request(entry));
        assert_eq!(w.execution.fingerprint(), old["executionFingerprint"]);
        let result = refute(&p, &w).unwrap();
        assert!(
            !result.blocks_release(),
            "unsound-relation: {}",
            result.report()
        );
        assert_eq!(result.report()["status"], "refuted");
        assert_eq!(result.report()["claimDigest"], old["claimDigest"]);
        rows.push(json!({"vector":old["vector"],"claimDigest":p.claim_digest(),"premiseDigest":p.premises.digest(),"executionFingerprint":w.execution.fingerprint(),"status":result.report()["status"],"conflictWithEstablished":result.blocks_release()}));
        let mut wrong = w.clone();
        wrong.claim_digest = "00".repeat(32);
        assert!(refute(&p, &wrong).is_err());
        wrong = w.clone();
        wrong.premise_digest = "00".repeat(32);
        assert!(refute(&p, &wrong).is_err());
        wrong = w.clone();
        wrong.subject_input_id = "00".repeat(32);
        assert!(refute(&p, &wrong).is_err());
        // Rebound digests cannot hide a changed state or guard from replay.
        let mut changed = p.clone();
        let mut domain: StateDomain =
            serde_json::from_value(p.premises.state_constraints.value().unwrap().clone()).unwrap();
        domain.block_context.height += 1;
        changed.premises.state_constraints =
            Premise::supplied(serde_json::to_value(domain).unwrap());
        assert!(refute(&changed, &witness(&changed, w.execution.clone())).is_err());
        changed = p.clone();
        changed.premises.guard = Guard::Not {
            guard: Box::new(Guard::True),
        };
        assert!(refute(&changed, &witness(&changed, w.execution.clone())).is_err());
        changed = p.clone();
        changed.premises.root_bytes = Premise::supplied("10010101d17300".into());
        if changed.claim_digest() != p.claim_digest() {
            assert!(refute(&changed, &witness(&changed, w.execution.clone())).is_err());
        }
    }
    assert_eq!(rows.len(), 27);
    for family in [
        "literal_spend-control/omission",
        "action_alternatives-control/omission",
        "exists_literal-control/omission",
        "self_alias-control/omission",
    ] {
        assert!(rows.iter().any(|r| r["vector"] == family));
    }
    let old = &prior[0];
    let p: RelationProposal = serde_json::from_value(old["claim"].clone()).unwrap();
    let entry = entries.iter().find(|e| e["id"] == old["vector"]).unwrap();
    let req = request(entry);
    // Every fixed validator context component is checked even with freshly rebound digests.
    for field in [
        "parameters",
        "networkRules",
        "headers",
        "priorBlockCost",
        "localPolicy",
    ] {
        let mut value = p.premises.state_constraints.value().unwrap().clone();
        match field {
            "parameters" => value[field]["maxBlockCost"] = json!(123456789),
            "networkRules" => value[field]["description"] = json!("different domain"),
            "headers" => value[field] = json!(["00"]),
            "priorBlockCost" => value[field] = json!(1),
            "localPolicy" => value[field]["maxTransactionSize"] = json!(1),
            _ => unreachable!(),
        }
        let mut changed = p.clone();
        changed.premises.state_constraints = Premise::supplied(value);
        assert!(
            refute(&changed, &witness(&changed, req.clone())).is_err(),
            "{field}"
        );
    }
    let mut changed = p.clone();
    changed.subject = Subject::BoxId {
        hex: "00".repeat(32),
    };
    assert!(refute(&changed, &witness(&changed, req.clone())).is_err());
    changed = p.clone();
    changed.premises.node_revision = Premise::supplied("00".repeat(20));
    assert!(refute(&changed, &witness(&changed, req.clone())).is_err());
    let mut invalid = witness(&p, req.clone());
    invalid.execution.transaction_bytes = "00".into();
    let failure = refute(&p, &invalid).unwrap_err();
    assert_ne!(failure["stage"], "complete");
    let mut raw = serde_json::to_value(witness(&p, req)).unwrap();
    raw["version"] = json!("omission-witness:v2");
    assert!(serde_json::from_value::<OmissionWitness>(raw).is_err());
    let mut accepted = 0;
    let mut rejected = 0;
    // Replay every M03 canonical vector, including satisfying and script failures.
    for entry in &entries {
        match ergo_sandbox::evidence::validate::validate(&request(entry)) {
            Ok(_) => {
                assert_eq!(entry["status"], "node-accepted");
                accepted += 1;
            }
            Err(f) => {
                assert_eq!(entry["status"], "node-rejected");
                assert_eq!(f.stage, "node-validation");
                assert!(f.detail.starts_with("ProofFailed { index: 0 }"));
                rejected += 1;
            }
        }
    }
    let (_, _, r) = inventory()
        .into_iter()
        .find(|(c, _, _)| c["id"] == "literal_spend-positive")
        .unwrap();
    for suffix in ["satisfying", "omission"] {
        let entry = entries
            .iter()
            .find(|e| e["id"] == format!("literal_spend-positive/{suffix}"))
            .unwrap();
        let err = refute(&r.claim, &witness(&r.claim, request(entry))).unwrap_err();
        if suffix == "omission" {
            assert_eq!(err["stage"], "node-validation");
        } else {
            assert_eq!(
                err["reason"],
                "distinct matching spending companion is present"
            );
        }
    }
    evidence(
        "m04-omission-results.json",
        &json!({"version":"m04-omission-results:v1","implementationBase":"3f349fc6b970889346be20775db255ba74c9381a","canonicalManifestSha256":"cdf73504cc3d5f98141bac9e44b46621a76baf8ac5219826d03367d171383853","priorOmissionsSha256":"2b4dd32bcdc5d41c0974e3718383744e0f6c903eae4ce5e4048695ad40c42ae1","refutations":rows,"metrics":{"registeredAcceptedOmissions":27,"refuted":27,"conflicts":0,"canonicalAccepted":accepted,"canonicalRejected":rejected}}),
    );
    println!("27/27 digest-matched accepted omissions refuted; {accepted} accepted / {rejected} script rejections; altered bindings and accepted satisfying spends cannot refute");
}
