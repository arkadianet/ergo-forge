use ergo_sandbox::{
    audit::{audit, audit_with_contracts, ContractSet, Execution, InputContract},
    compile_source,
    evidence::{
        case::json_digest,
        claim::{obligation_report, ReviewDecision},
        replay::ReplayBundle,
        CasePremises, EvidenceCase, Premise, SourceIdentity,
    },
    lift_tree, Lifted,
};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

fn policy() -> Value {
    serde_json::from_str(
        include_str!("../../docs/ROADMAP.md")
            .split("<!-- roadmap-policy:v1 -->")
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
fn target(source: &str) -> (EvidenceCase, Lifted) {
    let out = compile_source(source, 3, NetworkPrefix::Testnet).unwrap();
    let mut p = CasePremises::unspecified();
    p.target_bytes = Premise::supplied(hex::encode(&out.tree_bytes));
    p.source = Premise::supplied(SourceIdentity::supplied(source));
    (
        EvidenceCase::new(p).unwrap(),
        lift_tree(&out.ergo_tree, true),
    )
}
fn context(lifted: &Lifted) -> ergo_sandbox::audit::ContextAudit {
    audit_with_contracts(
        lifted,
        &ContractSet {
            inputs: &[],
            complete: false,
            singleton_tokens: &BTreeMap::new(),
        },
    )
}
fn duplicate() -> (EvidenceCase, Lifted) {
    target("sigmaProp(getVar[Int](0).get > 0)")
}
fn duplicate_lifted() -> Lifted {
    // Presentation fixture: twelve distinct occurrences of the same receiver.
    // The compiler CSEs repeated source reads, so author the recovered AST
    // explicitly. These anchors are lift-local; no fabricated IR mapping.
    let bytes =
        include_bytes!("fixtures/evidence/precision-vectors/duplicate-observations.fixture");
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/evidence/precision.json")).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        manifest["duplicateFixture"]["sha256"]
    );
    let fixture: Value = serde_json::from_slice(bytes).unwrap();
    let (_, single) = target(fixture["source"].as_str().unwrap());
    let count = fixture["occurrences"].as_u64().unwrap();
    fn reidentify(n: &mut ergo_sandbox::Node, next: &mut u64) {
        n.id = *next;
        *next += 1;
        match &mut n.kind {
            ergo_sandbox::NodeKind::Method(r, _, args) => {
                reidentify(r, next);
                for a in args {
                    reidentify(a, next);
                }
            }
            ergo_sandbox::NodeKind::Infix(_, a, b) => {
                reidentify(a, next);
                reidentify(b, next);
            }
            _ => (),
        }
    }
    let mut next = 1;
    let mut nodes = (0..count)
        .map(|_| {
            let mut node = single.node.clone();
            reidentify(&mut node, &mut next);
            node
        })
        .collect::<Vec<_>>();
    let mut node = nodes.pop().unwrap();
    for other in nodes {
        node = ergo_sandbox::Node {
            id: next,
            kind: ergo_sandbox::NodeKind::Infix("&&", Box::new(other), Box::new(node)),
        };
        next += 1;
    }
    Lifted {
        node,
        raw_placeholders: 0,
        truncated: false,
        ir_ids: Default::default(),
    }
}
#[test]
fn duplicate_observations_keep_all_anchors_in_one_obligation() {
    let lifted = duplicate_lifted();
    let a = audit(&lifted);
    let p = policy();
    assert_eq!(
        a.findings.len() as u64,
        p["thresholds"]["duplicateInputAnchors"]
    );
    assert_eq!(
        a.obligations.len() as u64,
        p["thresholds"]["duplicateOutputGroups"]
    );
    assert_eq!(
        a.obligations[0].anchors.len() as u64,
        p["thresholds"]["duplicateOutputAnchors"]
    );
    assert_eq!(
        a.findings
            .iter()
            .map(|f| f.node_id)
            .collect::<BTreeSet<_>>(),
        a.obligations[0].anchors.iter().map(|f| f.node_id).collect()
    );
    assert!(a.obligations[0].anchors.iter().all(|f| f.ir_id.is_none()));
    assert_eq!(a.obligations[0].status, "unresolved");
    let (_, unknown) = target("sigmaProp(SELF.R4[Int].get > 1 && SELF.R5[Int].get > 2)");
    let unknown = audit(&unknown);
    assert_eq!(unknown.obligations.len(), unknown.findings.len());
    assert!(unknown.obligations.len() >= 2);
    let (_, changed) = target("sigmaProp(getVar[Int](0).get > 90)");
    assert_eq!(audit(&changed).obligations[0].key, a.obligations[0].key);
    println!("duplicate: {} anchors -> {} obligation; all lift-local anchors retained; unknowns separate",a.findings.len(),a.obligations.len());
}
#[test]
fn suppression_invalidates_on_premise_change() {
    let (case, lifted) = duplicate();
    let c = context(&lifted);
    let r = obligation_report(&case, &c, &[], &[]).unwrap();
    let d = ReviewDecision {
        obligation_key: r["obligations"][0]["key"].as_str().unwrap().into(),
        premise_fingerprint: r["premiseFingerprint"].as_str().unwrap().into(),
        reason: "Caller reviewed witness rejection as intended under this policy".into(),
    };
    let d: ReviewDecision = serde_json::from_value(serde_json::to_value(d).unwrap()).unwrap();
    let original = obligation_report(&case, &c, std::slice::from_ref(&d), &[]).unwrap();
    assert_eq!(
        original["obligations"][0]["status"],
        "suppressed-under-premises"
    );
    for field in [
        "constants",
        "companionCode",
        "deploymentState",
        "policy",
        "sourceRevision",
    ] {
        let mut p = case.premises().clone();
        match field {
            "constants" => {
                p.constants = Premise::supplied(ergo_sandbox::evidence::BindingSet {
                    complete: true,
                    bindings: vec![ergo_sandbox::evidence::ConstantBinding {
                        name: "limit".into(),
                        typed_value: json!({"type":"Int","value":90}),
                        origin: ergo_sandbox::evidence::Origin::CallerSupplied,
                        mechanism: "authored test binding".into(),
                        source_lines: vec![1],
                    }],
                })
            }
            "deploymentState" => p.context = Premise::supplied(json!({"height":999})),
            "sourceRevision" => {
                let mut source = p.source.value().unwrap().clone();
                source.record.revision = Some("changed-revision".into());
                p.source = Premise::supplied(source);
            }
            _ => {
                p.assumptions
                    .insert(field.into(), Premise::supplied(json!({"changed":true})));
            }
        }
        let changed = EvidenceCase::new(p).unwrap();
        let r = obligation_report(&changed, &c, std::slice::from_ref(&d), &[]).unwrap();
        assert_eq!(r["reviewDecisions"][0]["valid"], false, "{field}");
        assert_eq!(r["obligations"][0]["status"], "unresolved");
        assert_eq!(
            r["obligations"][0]["anchors"],
            original["obligations"][0]["anchors"]
        );
    }
    let mut changed = c.clone();
    changed.premises["contracts"] = json!([{"recoveredCode":"sigmaProp(false)"}]);
    assert_eq!(
        obligation_report(&case, &changed, &[d], &[]).unwrap()["reviewDecisions"][0]["valid"],
        false
    );
    println!("suppression: constants, companion code, deployment state, policy and source changes invalidate; stale reasons and anchors retained");
}
fn conditional(
    case: &EvidenceCase,
    reader: &Lifted,
    binder: &Lifted,
    tokens: &BTreeMap<String, String>,
    complete: bool,
) -> Value {
    let inputs = [
        InputContract {
            name: "reader",
            execution: Execution::SpendingInput(0),
            lifted: reader,
        },
        InputContract {
            name: "binder",
            execution: Execution::SpendingInput(1),
            lifted: binder,
        },
    ];
    let c = audit_with_contracts(
        reader,
        &ContractSet {
            inputs: &inputs,
            complete,
            singleton_tokens: tokens,
        },
    );
    obligation_report(case, &c, &[], &[]).unwrap()
}
#[test]
fn conditional_discharge_exports_the_full_premise_set() {
    let (case, reader) = target("sigmaProp(INPUTS(0).value > SELF.value)");
    let (_, binder) = target(&format!(
        "sigmaProp(INPUTS(0).tokens(0)._1 == fromBase16(\"{}\"))",
        "11".repeat(32)
    ));
    let tokens = [(
        "11".repeat(32),
        "authored singleton emission assumption".into(),
    )]
    .into();
    let r = conditional(&case, &reader, &binder, &tokens, true);
    assert!(r["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["status"] == "conditionally-discharged"));
    let restored: Value = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(r, restored);
    assert_eq!(r["premises"]["case"], serde_json::to_value(&case).unwrap());
    assert_eq!(
        r["premises"]["conditionalContext"]["singletonTokens"],
        json!(tokens)
    );
    assert_eq!(
        r["premises"]["conditionalContext"]["contracts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for g in r["obligations"].as_array().unwrap() {
        for d in g["discharges"].as_array().unwrap() {
            assert!(!d["evidence"]["reason"].as_str().unwrap().is_empty());
            assert_eq!(d["evidence"]["companion"], "binder");
        }
    }
    for negative in [
        conditional(&case, &reader, &binder, &tokens, false),
        conditional(&case, &reader, &binder, &BTreeMap::new(), true),
    ] {
        assert!(negative["obligations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["status"] == "unresolved"));
        assert_ne!(negative["premiseFingerprint"], r["premiseFingerprint"]);
    }
    println!("conditional discharge: full case, both executing scripts, membership, singleton evidence, recovery gaps and reason survive export; omission controls remain unresolved");
}
#[test]
fn curated_precision_has_positive_and_negative_controls() {
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/evidence/precision.json")).unwrap();
    let rows = manifest["cases"].as_array().unwrap();
    let benign = [
        "malformed-optional-witness-rejection",
        "missing-candidate-output-register-rejection",
        "well-formed-self-register",
        "fixed-recipient-paid-output",
        "required-companion-identity-binding",
        "intended-permissionless-height-release",
    ];
    let positives = ["use-incident", "sale-mutant-unpaid"];
    assert_eq!(
        rows.iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        benign.into_iter().chain(positives).collect()
    );
    assert_eq!(rows.len(), benign.len() + positives.len());
    let p = policy();
    assert_eq!(benign.len() as u64, p["thresholds"]["benignPrecisionCases"]);
    assert_eq!(
        positives.len() as u64,
        p["thresholds"]["positivePrecisionCases"]
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evidence");
    let mut positive_claims = 0;
    let mut false_claims = 0;
    let mut anchors = 0;
    for row in rows {
        let id = row["id"].as_str().unwrap();
        assert_eq!(
            row["nodeRevision"],
            ergo_sandbox::evidence::validate::node_revision()
        );
        assert_eq!(
            row["publicationEligibility"],
            "public-authored-or-public-incident"
        );
        let path = root
            .join(row["path"].as_str().unwrap())
            .canonicalize()
            .unwrap();
        assert!(path.starts_with(root.canonicalize().unwrap()));
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)), row["sha256"]);
        let is_positive = positives.contains(&id);
        assert_eq!(
            row["label"],
            if is_positive { "positive" } else { "benign" }
        );
        let r = if is_positive || id == "fixed-recipient-paid-output" {
            let bundle: ReplayBundle = serde_json::from_slice(&bytes).unwrap();
            // Audit the exact protected script from the replay bundle, retaining
            // all supplied state premises without changing the pinned bundle.
            let protected = bundle
                .property
                .inputs
                .iter()
                .find(|i| i.role == ergo_sandbox::drain::DrainRole::Protected)
                .unwrap();
            let record = bundle
                .execution
                .case
                .premises()
                .boxes
                .value()
                .unwrap()
                .iter()
                .find(|r| r.document()["boxId"] == protected.box_id)
                .unwrap();
            let wire = ergo_sandbox::evidence::wire::WireBox::from_record(record.clone()).unwrap();
            let bytes = wire.node().candidate.ergo_tree_bytes();
            let mut premises = bundle.execution.case.premises().clone();
            premises.target_bytes = Premise::supplied(hex::encode(bytes));
            let case = EvidenceCase::new(premises).unwrap();
            let tree = ergo_sandbox::inspect::parse_tree(&case.target_bytes().unwrap()).unwrap();
            let lifted = lift_tree(&tree, true);
            let expected_anchors = audit(&lifted).findings.len();
            let r = obligation_report(&case, &context(&lifted), &[], &[bundle]).unwrap();
            assert!(r["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|g| g["status"] == "unresolved"));
            assert_eq!(
                r["obligations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g["anchors"].as_array().unwrap().len())
                    .sum::<usize>(),
                expected_anchors
            );
            assert_eq!(
                r["propertyResults"][0]["status"],
                row["expectedClaimStatus"]
            );
            assert_eq!(r["propertyResults"][0]["nodeValidated"], true);
            assert_eq!(row["expectedAcceptance"], "node-accepted");
            if !is_positive {
                assert_eq!(r["propertyResults"][0]["status"], "accepted-nonviolating");
            }
            r
        } else {
            assert_eq!(row["expectedClaimStatus"], "no-property-claim");
            let fixture: Value = serde_json::from_slice(&bytes).unwrap();
            let source = fixture["source"].as_str().unwrap();
            let (case, lifted) = target(source);
            let mut premises = case.premises().clone();
            premises.context = Premise::supplied(fixture["scenario"].clone());
            premises.assumptions.insert(
                "authorIntent".into(),
                Premise::supplied(fixture["reason"].clone()),
            );
            let case = EvidenceCase::new(premises).unwrap();
            let r = if id == "required-companion-identity-binding" {
                assert_eq!(row["expectedAcceptance"], "conditional-only");
                let (_, binder) = target(fixture["companionSource"].as_str().unwrap());
                let tokens = serde_json::from_value(fixture["singletonTokens"].clone()).unwrap();
                let r = conditional(&case, &lifted, &binder, &tokens, true);
                assert!(r["obligations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g["status"] == "conditionally-discharged"));
                r
            } else {
                let mut scenario = fixture["scenario"].clone();
                scenario["source"] = json!(source);
                if scenario.get("height").is_none() {
                    scenario["height"] = json!(200);
                }
                let outcome =
                    ergo_sandbox::eval_scenario(&serde_json::from_value(scenario).unwrap())
                        .unwrap();
                assert_eq!(
                    row["expectedAcceptance"],
                    format!("synthetic-{:?}", outcome.verdict).to_lowercase()
                );
                assert_eq!(
                    format!("{:?}", outcome.verdict),
                    fixture["expectedVerdict"].as_str().unwrap(),
                    "{id}: {:?}",
                    outcome.error
                );
                obligation_report(&case, &context(&lifted), &[], &[]).unwrap()
            };
            assert!(
                !r["obligations"].as_array().unwrap().is_empty(),
                "{id}: dropping observations cannot pass"
            );
            let retained: usize = r["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["anchors"].as_array().unwrap().len())
                .sum();
            assert_eq!(retained, audit(&lifted).findings.len());
            r
        };
        if !is_positive {
            anchors += r["obligations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["anchors"].as_array().unwrap().len())
                .sum::<usize>();
        }
        let claims = r["propertyResults"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["status"] == "confirmed-violation")
            .count();
        if is_positive {
            assert_eq!(claims, 1, "{id}");
            positive_claims += claims;
        } else {
            false_claims += claims;
        }
        println!(
            "{id}: {claims} confirmed violations; {} obligations",
            r["obligations"].as_array().unwrap().len()
        );
    }
    assert!(
        false_claims as u64
            <= p["thresholds"]["benignConfirmedViolationsMax"]
                .as_u64()
                .unwrap()
    );
    assert!(
        positive_claims as u64
            >= p["thresholds"]["positivePrecisionClaimsMin"]
                .as_u64()
                .unwrap()
    );
    println!("curated precision: {positive_claims}/2 positives retained, {false_claims}/6 benign falsely confirmed; {anchors} benign static anchors retained; manifest {}",json_digest(&manifest));
}
