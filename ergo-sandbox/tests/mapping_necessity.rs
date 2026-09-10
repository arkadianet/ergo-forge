//! M03: independent pinned answers, exact-code checking and full node replay.
#[allow(dead_code)]
mod mapping_support;
use ergo_sandbox::{
    evidence::{
        case::{CasePremises, EvidenceCase, Origin, Premise},
        validate::{validate, ValidationRequest},
        wire::{WireBox, WireTransaction},
    },
    map::{
        check_relation::{check, StateDomain},
        necessity::{propose, Derivation},
        relations::*,
    },
};
use ergo_ser::input::{ContextExtension, DataInput, Input, SpendingProof};
use mapping_support::{canonical_box, read, root, sha, wire_spec};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
fn hypothetical<T>(value: T) -> Premise<T> {
    Premise::Present {
        value,
        origin: Origin::Hypothetical,
    }
}
fn inventory() -> Vec<(Value, Value)> {
    let m = read("manifest.json");
    let e = read("expected.json");
    assert_eq!(
        sha(&std::fs::read(root().join("manifest.json")).unwrap()),
        "7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f"
    );
    assert_eq!(
        sha(&std::fs::read(root().join("expected.json")).unwrap()),
        "291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e"
    );
    let mut ids = BTreeSet::new();
    let rows = m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(e["cases"].as_array().unwrap())
        .map(|(entry, answer)| {
            let raw = std::fs::read(root().join(entry["path"].as_str().unwrap())).unwrap();
            assert_eq!(sha(&raw), entry["sha256"]);
            let c: Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(c["id"], answer["id"]);
            assert!(ids.insert(c["id"].as_str().unwrap().to_owned()));
            (c, answer.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 32);
    rows
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
            assert_eq!(hex::encode(w.bytes().unwrap()), b["bytes"]);
            assert_eq!(w.id().unwrap(), b["boxId"]);
            (b["name"].as_str().unwrap().into(), w)
        })
        .collect()
}
fn template() -> ValidationRequest {
    serde_json::from_str(include_str!(
        "fixtures/evidence/node-vectors/accepted-keyless-spend.fixture"
    ))
    .unwrap()
}
/// Independently specified full transactions for the pinned scenario inputs.
/// Balance outputs are explicit fixture construction, never a product search.
fn authored_request(c: &Value, v: &Value) -> ValidationRequest {
    let bs = boxes(c);
    let mut request = template();
    let mut context = request.block_context.value().unwrap().clone();
    context.height = v["height"].as_u64().unwrap() as u32;
    context.activated_script_version = 3;
    context.pre_header_version = 4;
    request.block_context = hypothetical(context);
    let mut p = CasePremises::unspecified();
    p.target_bytes = hypothetical(c["treeHex"].as_str().unwrap().to_owned());
    p.boxes = hypothetical(bs.values().map(|b| b.record().clone()).collect());
    request.case = EvidenceCase::new(p).unwrap();
    let mut tokens = BTreeMap::<String, u64>::new();
    let mut value = 0;
    let inputs = v["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let b = &bs[name.as_str().unwrap()];
            value += b.node().candidate.value;
            for t in &b.node().candidate.tokens {
                *tokens
                    .entry(hex::encode(t.token_id.as_bytes()))
                    .or_default() += t.amount;
            }
            let mut extension = ContextExtension::empty();
            if i == 0 {
                for (id, tv) in v["contextVars"].as_object().unwrap() {
                    let (t, x) = ergo_sandbox::scenario::parse_typed_value(
                        tv["type"].as_str().unwrap(),
                        &tv["value"],
                    )
                    .unwrap();
                    extension.values.insert(id.parse().unwrap(), (t, x));
                }
            }
            Input {
                box_id: b.node().box_id().unwrap(),
                spending_proof: SpendingProof::new(vec![], extension).unwrap(),
            }
        })
        .collect();
    let data_inputs = v["dataInputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| DataInput {
            box_id: bs[name.as_str().unwrap()].node().box_id().unwrap(),
        })
        .collect();
    let out_tree = v["outputs"]
        .as_array()
        .unwrap()
        .first()
        .map(|name| {
            hex::encode(
                bs[name.as_str().unwrap()]
                    .node()
                    .candidate
                    .ergo_tree_bytes(),
            )
        })
        .unwrap_or_else(|| "10010101d17300".into());
    let output = ergo_sandbox::evidence::wire::CandidateSpec {
        value,
        ergo_tree: out_tree,
        creation_height: v["height"].as_u64().unwrap() as u32,
        tokens: tokens
            .into_iter()
            .map(|(id, amount)| ergo_sandbox::evidence::wire::TokenSpec { id, amount })
            .collect(),
        registers: "00".into(),
    };
    request.transaction_bytes = hex::encode(
        WireTransaction::build(inputs, data_inputs, &[output])
            .unwrap()
            .bytes(),
    );
    request
}
fn claim(c: &Value, a: &Value) -> RelationProposal {
    let bs = boxes(c);
    let mut req = template();
    let mut ctx = req.block_context.value().unwrap().clone();
    ctx.height = c["vectors"][0]["height"].as_u64().unwrap() as u32;
    ctx.activated_script_version = 3;
    ctx.pre_header_version = 4;
    req.block_context = hypothetical(ctx);
    let selector = &a["relation"]["selector"];
    let target = match a["relation"]["kind"].as_str().unwrap() {
        "Spend" => Relation::Spend {
            selector: match selector["kind"].as_str().unwrap() {
                "exact-proposition-bytes" => Selector::PropositionBytes {
                    hex: hex::encode(
                        bs[selector["universeName"].as_str().unwrap()]
                            .node()
                            .candidate
                            .ergo_tree_bytes(),
                    ),
                },
                "exact-proposition-hash" => Selector::PropositionHash {
                    hex: selector["hex"].as_str().unwrap().into(),
                },
                "token-at-literal-position" => Selector::TokenAt {
                    index: selector["position"].as_u64().unwrap() as u32,
                    id: selector["id"].as_str().unwrap().into(),
                    amount: selector["minimumAmount"].as_u64().unwrap(),
                },
                _ => Selector::BoxId {
                    hex: bs[selector["universeName"].as_str().unwrap()].id().unwrap(),
                },
            },
        },
        "Execute" => Relation::Execute {
            input: "0".into(),
            variable: 1,
            code_digest: "unresolved-M05".into(),
        },
        _ => Relation::Authenticates {
            field: "unsupported-avl".into(),
            expected: Literal::Bytes("00".into()),
        },
    };
    let domain = StateDomain {
        block_context: req.block_context.value().unwrap().clone(),
        parameters: req.parameters.value().unwrap().clone(),
        network_rules: req.network_rules.value().unwrap().clone(),
        headers: req.headers.value().unwrap().clone(),
        prior_block_cost: *req.prior_block_cost.value().unwrap(),
        local_policy: req.local_policy.value().unwrap().clone(),
        positive_input_tokens: true,
    };
    RelationProposal {
        subject: Subject::BoxId {
            hex: bs["A"].id().unwrap(),
        },
        premises: Premises {
            root_bytes: hypothetical(c["treeHex"].as_str().unwrap().into()),
            self_box: hypothetical(bs["A"].record().clone()),
            node_revision: hypothetical(ergo_sandbox::evidence::case::engine_revision().into()),
            compiler_revision: hypothetical(ergo_sandbox::evidence::case::engine_revision().into()),
            network: hypothetical("hypothetical-mainnet-activation-3".into()),
            activation_rules: hypothetical(
                json!({"activatedScriptVersion":3,"rootEvaluation":"required-before-storage-rent"}),
            ),
            state_constraints: hypothetical(serde_json::to_value(domain).unwrap()),
            guard: if !a["guardObject"].is_null() {
                serde_json::from_value(a["guardObject"].clone()).unwrap()
            } else if a["relation"]["guard"] == "HEIGHT >= 100" {
                Guard::HeightAtLeast { value: 100 }
            } else {
                Guard::True
            },
            authentication_roots: if c["family"] == "register_identity" {
                bs.values().map(|b| b.record().clone()).collect()
            } else {
                vec![]
            },
            provenance: hypothetical(
                json!({"source":"M00 independently authored synthetic; full-node M03 state conditional"}),
            ),
            analysis_caps: BTreeMap::from([("nodes".into(), 10000)]),
        },
        target,
        status: ProposalStatus::Unresolved,
        reason: "independently nominated exact-code claim".into(),
        proposed_derivation: Value::Null,
        dependency_ids: vec![],
        exact_anchors: vec![],
        authenticated_bytes: vec![],
    }
}
fn result_path() -> std::path::PathBuf {
    root().join("../../../../docs/mapping/m03-mapping-results.json")
}
fn m03_supported(a: &Value) -> bool {
    a["supported"] == true && a["relation"]["kind"] == "Spend"
}
#[test]
fn exact_guarded_necessity_matches_pinned_answers() {
    let mut rows = vec![];
    let mut established = 0;
    for (c, a) in inventory() {
        let p = claim(&c, &a);
        let d = propose(&p).unwrap();
        let result = check(&p, &d);
        assert_eq!(
            result.is_ok(),
            m03_supported(&a),
            "{}: {:?}",
            c["id"],
            result
        );
        if result.is_ok() {
            established += 1;
        }
        rows.push(json!({"id":c["id"],"claimDigest":p.claim_digest(),"premiseDigest":p.premises.digest(),"claim":p,"derivation":d,"checkedReport":result.as_ref().ok().map(|r|r.report()),"status":if result.is_ok(){"established-under-premises"}else{"unresolved"},"reason":result.as_ref().err(),"supportedInPinnedAnswer":a["supported"],"deferredToM05":a["relation"]["kind"]=="Execute"}));
    }
    assert_eq!(established, 12);
    let mut extra_rows = vec![];
    for (c, a) in supplemental() {
        assert_eq!(a["truth"] == "true", a["supported"] == true);
        let p = claim(&c, &a);
        let result = check(&p, &propose(&p).unwrap());
        assert_eq!(
            result.is_ok(),
            a["supported"] == true,
            "{}: {:?}",
            c["id"],
            result
        );
        assert_eq!(
            hex::encode(mapping_support::compile(c["source"].as_str().unwrap())),
            c["treeHex"]
        );
        extra_rows.push(json!({"id":c["id"],"claim":p,"checkedReport":result.as_ref().ok().map(|r|r.report()),"reason":result.as_ref().err()}));
    }

    let result = json!({"version":"m03-mapping-results:v2","implementationBase":"0545b16347a29dbe9b60bc1a3df46a5eb602bd8d","canonicalManifestSha256":"cdf73504cc3d5f98141bac9e44b46621a76baf8ac5219826d03367d171383853","supplementalSha256":"25b20fb46cdd45ee199e3847321419fef765505a0db8d61b0bbc0499c40adfda","nodeRevision":ergo_sandbox::evidence::case::engine_revision(),"manifestSha256":"7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f","expectedSha256":"291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e","cases":rows,"supplementalCases":extra_rows,"supplementalMetrics":{"registered":10,"established":5,"unresolved":5},"metrics":{"registered":32,"established":12,"supportedM03":12,"unresolved":20,"incorrectEstablished":0,"allSupportedIncludingM05":14}});
    let bytes = std::fs::read(result_path()).unwrap();
    assert_eq!(result, serde_json::from_slice::<Value>(&bytes).unwrap());
    println!(
        "evidence {} sha256={}; supported 12/12; established 12; incorrect 0/12; unresolved 20/32",
        result_path().display(),
        sha(&bytes)
    );
}
#[test]
fn alternatives_dead_checks_and_self_do_not_prove_cospend() {
    for (c, a) in inventory() {
        if a["supported"] == false {
            let p = claim(&c, &a);
            assert!(check(&p, &propose(&p).unwrap()).is_err(), "{}", c["id"]);
        }
    }
    let (c, a) = inventory()
        .into_iter()
        .find(|(c, _)| c["id"] == "action_alternatives-positive")
        .unwrap();
    let mut p = claim(&c, &a);
    p.premises.guard = Guard::True;
    assert!(check(&p, &propose(&p).unwrap()).is_err());
    let (c, a) = inventory()
        .into_iter()
        .find(|(c, _)| c["id"] == "literal_spend-positive")
        .unwrap();
    let mut p = claim(&c, &a);
    p.target = Relation::Authenticates {
        field: "inputs/1/id".into(),
        expected: Literal::Bytes(boxes(&c)["B"].id().unwrap()),
    };
    assert!(check(&p, &propose(&p).unwrap()).is_ok());
    p.target = Relation::Authenticates {
        field: "data-inputs/1/id".into(),
        expected: Literal::Bytes(boxes(&c)["B"].id().unwrap()),
    };
    assert!(check(&p, &propose(&p).unwrap()).is_err());
    for (c, a) in inventory().into_iter().chain(supplemental()) {
        if c["id"] == "script_identity-positive" || c["id"] == "m03-script-hash-positive" {
            let mut p = claim(&c, &a);
            let (field, expected) = match &p.target {
                Relation::Spend {
                    selector: Selector::PropositionBytes { hex },
                } => ("inputs/1/proposition-bytes", hex.clone()),
                Relation::Spend {
                    selector: Selector::PropositionHash { hex },
                } => ("inputs/1/proposition-hash", hex.clone()),
                _ => unreachable!(),
            };
            p.target = Relation::Authenticates {
                field: field.into(),
                expected: Literal::Bytes(expected),
            };
            check(&p, &propose(&p).unwrap()).unwrap();
            if let Relation::Authenticates { expected, .. } = &mut p.target {
                *expected = Literal::Bytes("00".repeat(32));
            }
            assert!(check(&p, &propose(&p).unwrap()).is_err());
        }
    }
    println!("18 pinned adversarial/unsupported members refused; unconditional alternative and wrong authentication scope refused");
}
#[test]
fn satisfying_omission_and_relaxed_controls_use_full_validator() {
    let mut manifest: Value = read("m03/manifest.json");
    let extra: Value = read("m03/supplemental-v2.fixture");
    assert_eq!(
        sha(&std::fs::read(root().join("m03/manifest.json")).unwrap()),
        "cdf73504cc3d5f98141bac9e44b46621a76baf8ac5219826d03367d171383853"
    );
    manifest["vectors"]
        .as_array_mut()
        .unwrap()
        .extend(extra["vectors"].as_array().unwrap().clone());
    let mut actual = vec![];
    let mut accepted = 0;
    let mut rejected = 0;
    let mut refutations = vec![];
    for (c, a) in inventory().into_iter().chain(supplemental()) {
        if matches!(
            c["family"].as_str().unwrap(),
            "context_code" | "context_scope" | "avl_identity"
        ) {
            continue;
        }
        for (v, expected) in c["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .zip(a["actions"].as_array().unwrap())
        {
            let id = format!(
                "{}/{}",
                c["id"].as_str().unwrap(),
                v["id"].as_str().unwrap()
            );
            let entry = manifest["vectors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == id)
                .unwrap();
            let raw =
                std::fs::read(root().join("m03").join(entry["path"].as_str().unwrap())).unwrap();
            assert_eq!(sha(&raw), entry["sha256"]);
            let req: ValidationRequest = serde_json::from_slice(&raw).unwrap();
            assert_eq!(
                serde_json::to_value(&req).unwrap(),
                serde_json::to_value(authored_request(&c, v)).unwrap()
            );
            let disposition = match validate(&req) {
                Ok(ok) => {
                    assert_eq!(expected["engineVerdict"], "pass", "{id}");
                    assert_eq!(ok.report()["nodeValidated"], true);
                    let mut p = claim(&c, &a);
                    // Each vector names its complete P; never compare across a
                    // changed context. A variant at another height is another claim.
                    let domain = StateDomain {
                        block_context: req.block_context.value().unwrap().clone(),
                        parameters: req.parameters.value().unwrap().clone(),
                        network_rules: req.network_rules.value().unwrap().clone(),
                        headers: req.headers.value().unwrap().clone(),
                        prior_block_cost: *req.prior_block_cost.value().unwrap(),
                        local_policy: req.local_policy.value().unwrap().clone(),
                        positive_input_tokens: true,
                    };
                    p.premises.state_constraints =
                        hypothetical(serde_json::to_value(domain).unwrap());
                    let assessment = accepted_omission(&p, &ok);
                    if a["supported"] == true {
                        assert!(
                            assessment.is_some(),
                            "positive must have a true guard: {id}"
                        );
                    }
                    let omission = assessment == Some(true);
                    if omission {
                        assert!(
                            check(&p, &propose(&p).unwrap()).is_err(),
                            "accepted contradiction: {id}"
                        );
                        refutations.push(json!({"vector":id,"claimDigest":p.claim_digest(),"premiseDigest":p.premises.digest(),"claim":p,"executionFingerprint":req.fingerprint(),"assessment":"refuted-by-accepted-omission","scope":"M03 test assessment; M04 product refutation import not implemented"}));
                    }
                    accepted += 1;
                    "node-accepted"
                }
                Err(f) => {
                    assert_eq!(expected["engineVerdict"], "fail", "{id}: {}", f.detail);
                    assert!(f.pipeline_invoked, "{id}: {}", f.detail);
                    assert_eq!(f.stage, "node-validation", "{id}: {}", f.detail);
                    assert!(
                        f.detail.starts_with("ProofFailed { index: 0 }"),
                        "{id}: {}",
                        f.detail
                    );
                    rejected += 1;
                    "node-rejected"
                }
            };
            assert_eq!(entry["status"], disposition);
            actual.push(id);
        }
    }
    assert!(refutations
        .iter()
        .any(|r| r["vector"] == "literal_spend-control/omission"));
    assert!(refutations
        .iter()
        .any(|r| r["vector"] == "action_alternatives-control/omission"));
    assert!(refutations
        .iter()
        .any(|r| r["vector"] == "exists_literal-control/omission"));
    assert_eq!(
        serde_json::from_slice::<Value>(
            &std::fs::read(root().join("../../../../docs/mapping/m03-omission-results.json"))
                .unwrap()
        )
        .unwrap(),
        json!(refutations)
    );
    for path in [
        "../../../../docs/mapping/m03-omission-results.json",
        "m03/supplemental-v2.fixture",
    ] {
        let full = root().join(path);
        println!(
            "evidence {} sha256={}",
            full.display(),
            sha(&std::fs::read(&full).unwrap())
        );
    }
    println!(
        "{} accepted premise-matching omission assessments refute their own exact claims",
        refutations.len()
    );
    assert_eq!(actual.len(), manifest["vectors"].as_array().unwrap().len());
    assert_eq!(actual.iter().collect::<BTreeSet<_>>().len(), actual.len());
    println!("full validator: {accepted} accepted, {rejected} script rejections; {} canonical vectors; evidence {} sha256={}",actual.len(),root().join("m03/manifest.json").display(),sha(&std::fs::read(root().join("m03/manifest.json")).unwrap()));
}
#[test]
fn unsupported_anchors_and_cyclic_proofs_are_rejected() {
    let (c, a) = inventory()
        .into_iter()
        .find(|(c, _)| c["id"] == "literal_spend-positive")
        .unwrap();
    let p = claim(&c, &a);
    let d = propose(&p).unwrap();
    check(&p, &d).unwrap();
    let mut variants = vec![];
    let mut x = d.clone();
    x.anchors[0].rule = "common-alternatives".into();
    variants.push(x);
    let mut x = d.clone();
    x.anchors.pop();
    variants.push(x);
    let mut x = d.clone();
    x.anchors[0].opcode ^= 1;
    variants.push(x);
    let mut x = d.clone();
    x.anchors.swap(0, 1);
    variants.push(x);
    let mut x = d.clone();
    x.dependencies.push(p.claim_digest());
    variants.push(x);
    let mut x = d.clone();
    x.version = "necessity-derivation:v2".into();
    variants.push(x);
    for x in variants {
        assert!(check(&p, &x).is_err());
    }
    let mut x = p.clone();
    x.subject = Subject::BoxId {
        hex: "00".repeat(32),
    };
    assert!(check(&x, &propose(&x).unwrap()).is_err());
    let mut x = p.clone();
    x.premises.analysis_caps.insert("nodes".into(), 1);
    assert!(check(&x, &propose(&x).unwrap()).is_err());
    let mut x = p.clone();
    let mut domain: StateDomain =
        serde_json::from_value(x.premises.state_constraints.value().unwrap().clone()).unwrap();
    domain.block_context.height = domain.parameters.storage_period + 1;
    x.premises.state_constraints = hypothetical(serde_json::to_value(domain).unwrap());
    assert!(check(&x, &propose(&x).unwrap())
        .unwrap_err()
        .contains("storage-rent"));
    let mut x = p.clone();
    x.premises.guard = Guard::HeightAtLeast { value: 100 };
    assert!(check(&x, &d).is_err());
    let mut x = p.clone();
    x.premises.node_revision = hypothetical("00".repeat(20));
    assert!(check(&x, &propose(&x).unwrap()).is_err());
    let mut x = p.clone();
    x.premises.root_bytes = hypothetical("10010101d17300".into());
    assert!(check(&x, &propose(&x).unwrap()).is_err());
    let (c, a) = inventory()
        .into_iter()
        .find(|(c, _)| c["id"] == "register_identity-positive")
        .unwrap();
    let mut x = claim(&c, &a);
    x.premises.authentication_roots.clear();
    assert!(check(&x, &propose(&x).unwrap()).is_err());
    let mut noncanonical = p.clone();
    if let Relation::Spend {
        selector: Selector::BoxId { hex },
    } = &mut noncanonical.target
    {
        *hex = hex.to_uppercase();
    }
    assert!(check(&noncanonical, &propose(&noncanonical).unwrap())
        .unwrap_err()
        .contains("canonical lowercase"));
    let imported: Derivation = serde_json::from_value(serde_json::to_value(&d).unwrap()).unwrap();
    check(&p, &imported).unwrap();
    println!("altered/missing/reordered anchors, claim/guard changes, cycles, caps, wrong subject and storage-rent acceptance domain rejected");
}
fn supplemental() -> Vec<(Value, Value)> {
    let raw = std::fs::read(root().join("m03/supplemental-v2.fixture")).unwrap();
    assert_eq!(
        sha(&raw),
        "25b20fb46cdd45ee199e3847321419fef765505a0db8d61b0bbc0499c40adfda"
    );
    let doc: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(doc["version"], "m03-supplemental:v2");
    assert_eq!(doc["cases"].as_array().unwrap().len(), 10);
    doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v["case"].clone(), v["answer"].clone()))
        .collect()
}

// Test-only relation assessment. Inputs come from the actual checked node
// execution. This does not implement M04's action or witness-import APIs.
fn accepted_omission(
    p: &RelationProposal,
    ok: &ergo_sandbox::evidence::validate::AcceptedExecution,
) -> Option<bool> {
    let domain: StateDomain =
        serde_json::from_value(p.premises.state_constraints.value().unwrap().clone()).unwrap();
    let req = ok.request();
    assert_eq!(
        serde_json::to_value(req.network_rules.value().unwrap()).unwrap(),
        serde_json::to_value(&domain.network_rules).unwrap()
    );
    assert_eq!(req.headers.value().unwrap(), &domain.headers);
    assert_eq!(
        *req.prior_block_cost.value().unwrap(),
        domain.prior_block_cost
    );
    assert_eq!(
        serde_json::to_value(req.local_policy.value().unwrap()).unwrap(),
        serde_json::to_value(&domain.local_policy).unwrap()
    );
    assert_eq!(
        serde_json::to_value(req.block_context.value().unwrap()).unwrap(),
        serde_json::to_value(&domain.block_context).unwrap()
    );
    assert_eq!(
        serde_json::to_value(req.parameters.value().unwrap()).unwrap(),
        serde_json::to_value(&domain.parameters).unwrap()
    );
    assert_eq!(
        req.case.premises().engine_revision,
        *p.premises.node_revision.value().unwrap()
    );
    let fixed = WireBox::from_record(p.premises.self_box.value().unwrap().clone()).unwrap();
    let inputs = ok.checked().resolved_inputs();
    let subject = inputs
        .iter()
        .find(|b| hex::encode(b.box_id().unwrap().as_bytes()) == fixed.id().unwrap())
        .unwrap();
    assert_eq!(subject, &fixed.node().clone());
    assert_eq!(
        hex::encode(subject.candidate.ergo_tree_bytes()),
        *p.premises.root_bytes.value().unwrap()
    );
    match &p.subject {
        Subject::BoxId { hex } => assert_eq!(*hex, fixed.id().unwrap()),
        Subject::Script { hex } => {
            assert_eq!(*hex, hex::encode(subject.candidate.ergo_tree_bytes()))
        }
    }
    assert!(inputs
        .iter()
        .all(|b| b.candidate.tokens.iter().all(|t| t.amount > 0)));
    for root in &p.premises.authentication_roots {
        let b = WireBox::from_record(root.clone()).unwrap();
        assert!(req
            .case
            .premises()
            .boxes
            .value()
            .unwrap()
            .iter()
            .any(|r| r == b.record()));
    }
    fn guard(g: &Guard, height: u32, value: u64) -> bool {
        match g {
            Guard::True => true,
            Guard::HeightAtLeast { value } => i64::from(height) >= i64::from(*value),
            Guard::Equals {
                field: GuardField::Context { name },
                literal: Literal::Int(i),
            } if name == "HEIGHT" => i64::from(height) == i64::from(*i),
            Guard::Equals {
                field: GuardField::SelfBox { name },
                literal: Literal::Long(i),
            } if name == "value" => i128::from(value) == i128::from(*i),
            Guard::And { guards } => guards.iter().all(|g| guard(g, height, value)),
            Guard::Or { guards } => guards.iter().any(|g| guard(g, height, value)),
            Guard::Not { guard: g } => !guard(g, height, value),
            _ => panic!("unsupported test guard"),
        }
    }
    if !guard(
        &p.premises.guard,
        domain.block_context.height,
        subject.candidate.value,
    ) {
        return None;
    }
    fn matches(s: &Selector, b: &ergo_ser::ergo_box::ErgoBox) -> bool {
        match s {
            Selector::BoxId { hex } => *hex == hex::encode(b.box_id().unwrap().as_bytes()),
            Selector::PropositionBytes { hex } => {
                *hex == hex::encode(b.candidate.ergo_tree_bytes())
            }
            Selector::PropositionHash { hex } => {
                *hex == hex::encode(
                    ergo_primitives::digest::blake2b256(b.candidate.ergo_tree_bytes()).as_bytes(),
                )
            }
            Selector::TokenAt { index, id, amount } => {
                b.candidate.tokens.get(*index as usize).is_some_and(|t| {
                    hex::encode(t.token_id.as_bytes()) == *id && t.amount >= *amount
                })
            }
            Selector::TokenMember { id, amount } => b
                .candidate
                .tokens
                .iter()
                .any(|t| hex::encode(t.token_id.as_bytes()) == *id && t.amount >= *amount),
            Selector::And { predicates } => predicates.iter().all(|s| matches(s, b)),
        }
    }
    let Relation::Spend { selector } = &p.target else {
        panic!("M03 only assesses Spend")
    };
    Some(
        !inputs
            .iter()
            .any(|b| b.box_id().unwrap() != subject.box_id().unwrap() && matches(selector, b)),
    )
}
