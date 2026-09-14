//! P06 — promoting a generated candidate through the evidence boundary.
//!
//! Offline. Every box below is reconstructed from a recorded block response in
//! `docs/p06-recovery/` and is admitted only if its canonical id matches the id
//! the incident request declares. Nothing is fetched here and nothing is
//! broadcast: promotion only ever *removes* claim authority from a candidate
//! that cannot be built, signed and validated.

use ergo_sandbox::drain::{drain_hunt, DrainRequest};
use ergo_sandbox::evidence::{
    promotion::{candidate_spec, promote},
    validate::ValidationRequest,
    wire::{BoxMaterial, CandidateSpec, CreationReference, WireBox, WireTransaction},
    EvidenceCase,
};
use ergo_ser::input::{ContextExtension, Input, SpendingProof};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[path = "../../docs/p05-recovery/derive/src/recover.rs"]
mod use_recovery;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn read(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(repo().join(path)).unwrap()).unwrap()
}
/// The attacker funds the spend from a box they own. The protocol's own boxes
/// below are the recorded historical ones; only this funding box is
/// hypothetical, and its premise is archived as such.
fn funding_key() -> ergo_sandbox::evidence::sign::OwnedDlogSecret {
    ergo_sandbox::evidence::sign::OwnedDlogSecret::from_bytes(zeroize::Zeroizing::new([0x24u8; 32]))
        .unwrap()
}
fn funding_reference() -> CreationReference {
    CreationReference {
        transaction_id: "aa".repeat(32),
        index: 0,
    }
}
fn funding_spec(template: &Value) -> CandidateSpec {
    let tree = format!("0008cd{}", hex::encode(funding_key().public_key()));
    serde_json::from_value(json!({
        "value": template["value"],
        "ergoTree": tree,
        "creationHeight": template["creationHeight"],
        "tokens": template["tokens"].as_array().cloned().unwrap_or_default(),
        "registers": "00",
    }))
    .unwrap()
}
fn funding_box(template: &Value) -> WireBox {
    WireBox::hypothetical(&funding_spec(template), &funding_reference()).unwrap()
}

/// Transaction-input position of the box the attacker funds from.
fn funding_input(request: &DrainRequest) -> usize {
    let report = drain_hunt(request).unwrap();
    let hit = report.best.as_ref().expect("a generated candidate");
    hit.witness
        .roles
        .iter()
        .position(|r| format!("{r:?}").to_lowercase().contains("attacker"))
        .expect("an attacker input")
}

fn request() -> DrainRequest {
    let mut raw =
        read("ergo-sandbox/tests/fixtures/evidence/promotion-vectors/use-honest-request.fixture");
    let inputs = raw["inputs"].as_array_mut().unwrap();
    let position = inputs
        .iter()
        .position(|i| i["role"] == "attacker")
        .expect("an attacker input");
    let template = inputs[position].clone();
    let owned = funding_box(&template);
    let entry = &mut inputs[position];
    entry["ergoTree"] = json!(hex::encode(owned.node().candidate.ergo_tree_bytes()));
    entry["boxId"] = json!(owned.id().unwrap());
    entry["registers"] = json!({});
    serde_json::from_value(raw).unwrap()
}

/// Every box the incident request declares, rebuilt from the recorded block that
/// created it. A reconstruction whose canonical id differs from the declared id
/// is refused here rather than admitted as material.
fn recovered_boxes() -> Vec<WireBox> {
    let retrievals = read("docs/p06-recovery/retrievals.json");
    let locator = |name: &str| -> (String, Option<String>) {
        let r = retrievals
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .expect("recorded retrieval");
        (
            r["url"].as_str().unwrap().into(),
            Some(r["sha256"].as_str().unwrap().into()),
        )
    };
    let mut out = vec![];
    for (block, tx_id, index) in [
        (
            "block-1868090",
            "18d03837d3d169afb5894fc4471aefb27c8808778a69591bc47f0f58a2e70507",
            0u16,
        ),
        (
            "block-1868090",
            "18d03837d3d169afb5894fc4471aefb27c8808778a69591bc47f0f58a2e70507",
            1,
        ),
    ] {
        let doc = read(&format!("docs/p06-recovery/{block}.fixture"));
        let tx = doc["blockTransactions"]["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == tx_id)
            .expect("creating transaction in recorded block");
        let output = &tx["outputs"][index as usize];
        let spec = candidate_spec(output).expect("complete box material");
        let reference = CreationReference {
            transaction_id: tx_id.into(),
            index,
        };
        let rebuilt = WireBox::hypothetical(&spec, &reference).expect("serializable box");
        let declared = output["boxId"].as_str().unwrap();
        assert_eq!(
            rebuilt.id().unwrap(),
            declared,
            "reconstruction must hash to the recorded box id"
        );
        let (url, revision) = locator(block);
        out.push(
            WireBox::recorded(
                BoxMaterial {
                    bytes: hex::encode(rebuilt.bytes().unwrap()),
                    ergo_tree: output["ergoTree"].as_str().unwrap().into(),
                    box_id: declared.into(),
                },
                url,
                revision,
            )
            .expect("recorded box"),
        );
    }
    out
}

/// Canonical material for exactly the candidate the search produced. Block
/// context, parameters, rules, headers and prior block cost are the ones P05
/// recovered for this height; only the case and transaction are new.
fn material_for(request: &DrainRequest) -> (ValidationRequest, Vec<WireBox>) {
    let report = drain_hunt(request).unwrap();
    let hit = report.best.as_ref().expect("a generated candidate");
    let candidate = &hit.witness.tx_request;

    let mut boxes = recovered_boxes();
    let template =
        read("ergo-sandbox/tests/fixtures/evidence/promotion-vectors/use-honest-request.fixture")
            ["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["role"] == "attacker")
            .unwrap()
            .clone();
    boxes.push(funding_box(&template));
    let ordered: Vec<WireBox> = candidate
        .tx
        .inputs
        .iter()
        .map(|i| {
            boxes
                .iter()
                .find(|b| b.id().unwrap() == i.box_id)
                .expect("declared input is backed by recovered material")
                .clone()
        })
        .collect();

    let inputs: Vec<Input> = ordered
        .iter()
        .map(|b| Input {
            box_id: b.node().box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![], ContextExtension::empty()).unwrap(),
        })
        .collect();
    let specs: Vec<_> = candidate
        .tx
        .outputs
        .iter()
        .map(|o| candidate_spec(o).expect("complete output material"))
        .collect();
    let wire = WireTransaction::build(inputs, vec![], &specs).expect("canonical transaction");

    let base = use_recovery::recover(&repo())
        .pop()
        .expect("recovered context");
    let mut premises = base.case.premises().clone();
    premises.boxes =
        ergo_sandbox::evidence::Premise::missing("rebound below from recovered material");
    premises.assumptions.remove("wireTransaction");
    let case = EvidenceCase::new(premises).unwrap();
    let case = wire.bind_case(&case, &ordered, &[]).expect("bound case");

    (
        ValidationRequest {
            format_version: base.format_version,
            case,
            transaction_bytes: hex::encode(wire.bytes()),
            parameters: base.parameters.clone(),
            network_rules: base.network_rules.clone(),
            block_context: base.block_context.clone(),
            headers: base.headers.clone(),
            local_policy: base.local_policy.clone(),
            prior_block_cost: base.prior_block_cost.clone(),
        },
        ordered,
    )
}

#[test]
fn generated_public_incident_candidate_promotes() {
    let request = request();
    let (material, _) = material_for(&request);
    let key = funding_key();
    let report = promote(&request, &material, Some((funding_input(&request), &key))).unwrap();
    let promotion = report.promotion.as_ref().expect("a promotion outcome");
    assert_eq!(
        promotion.failure, None,
        "promotion failed: {:?}",
        promotion.failure
    );
    assert_eq!(promotion.status, "confirmed-violation");
    let replay = promotion.replay.as_ref().unwrap();
    assert_eq!(replay["nodeValidated"], true);
    assert!(promotion.claim_reference().is_some());
    // The preflight verdict is recorded separately and is never rewritten by
    // promotion.
    assert!(!report.preflight.node_validated);
    assert_eq!(report.preflight.method, "unsigned-preflight");
}

#[test]
fn unbacked_synthetic_material_stays_preflight() {
    let request = request();
    let (mut material, ordered) = material_for(&request);
    // Invent one filler-token holding on an otherwise real box. The transaction
    // still parses; the material simply is not the box the chain recorded.
    let mut records = material.case.premises().boxes.value().cloned().unwrap();
    let mut doc = records[0].document().clone();
    doc["bytes"] = json!(format!("{}ff", doc["bytes"].as_str().unwrap()));
    records[0] = ergo_sandbox::evidence::RecordedBox::new(
        doc,
        ergo_sandbox::evidence::Origin::Hypothetical,
        None,
    )
    .unwrap();
    let mut premises = material.case.premises().clone();
    premises.boxes = ergo_sandbox::evidence::Premise::supplied(records);
    material.case = EvidenceCase::new(premises).unwrap();

    let report = promote(&request, &material, None).unwrap();
    let promotion = report.promotion.as_ref().unwrap();
    assert_ne!(promotion.status, "confirmed-violation");
    assert!(promotion.failure.is_some(), "a named failure is required");
    assert!(promotion.claim_reference().is_none());
    // The candidate is unchanged and still reported as what it is.
    assert!(!report.preflight.node_validated);
    assert_eq!(ordered.len(), 3);
}

#[test]
fn missing_funding_proof_blocks_promotion() {
    let request = request();
    let (material, _) = material_for(&request);
    // The attacker input is a real P2PK box whose key nobody here holds. Naming
    // it as funding without a usable secret must fail, not succeed by label.
    let key = ergo_sandbox::evidence::sign::OwnedDlogSecret::from_bytes(zeroize::Zeroizing::new(
        [0x11u8; 32],
    ))
    .unwrap();
    let report = promote(&request, &material, Some((funding_input(&request), &key))).unwrap();
    let promotion = report.promotion.as_ref().unwrap();
    assert_ne!(promotion.status, "confirmed-violation");
    assert!(
        promotion.failure.is_some(),
        "an unusable funding key must name a failure"
    );
    assert!(promotion.claim_reference().is_none());
}

#[test]
fn associated_lint_is_not_the_confirmed_claim() {
    let request = request();
    let (material, _) = material_for(&request);
    let key = funding_key();
    let report = promote(&request, &material, Some((funding_input(&request), &key))).unwrap();
    let reference = report
        .promotion
        .as_ref()
        .unwrap()
        .claim_reference()
        .expect("a claim reference");
    // The claim belongs to the property and the case. Nothing here promotes a
    // static finding, and causation is explicitly not established.
    assert_eq!(reference["kind"], "declared-property-claim");
    assert_eq!(reference["lintCausationEstablished"], false);
    let serialized = serde_json::to_value(&report).unwrap();
    let text = serialized.to_string();
    assert!(
        !text.contains("\"findings\""),
        "a drain report must not carry findings that could inherit its claim"
    );
}

// ── S03: the two-instance family ─────────────────────────────────────────────
//
// The counterexample/control pair is the S00 class-11 vector pair, registered
// for the family in `examples/mutants/search.json` with the caps used. The
// family runs the unchanged hunt on derived requests; promotion of a hit uses
// the unchanged P06 path on an explicit request that declares the derived box.

fn search_pair() -> Value {
    let doc = read("examples/mutants/search.json");
    doc["searchFamilyPairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "S03-01-v1")
        .cloned()
        .expect("the S03 pair is registered")
}

fn compile(path: &str) -> String {
    let source = std::fs::read_to_string(repo().join(path)).unwrap();
    hex::encode(
        ergo_sandbox::compile_source(&source, 3, ergo_ser::address::NetworkPrefix::Mainnet)
            .expect("vector compiles")
            .tree_bytes,
    )
}

/// One declared protected instance of `tree`, a funding input owned by the
/// test key, one fixed payment output and one free output — the shape the
/// class-11 vector's suite reproduces, minus the second instance, which is
/// the family's job to derive.
fn pair_request(tree: &str, family_on: bool) -> DrainRequest {
    let pair = search_pair();
    let r = &pair["request"];
    let nft = r["protocolNft"].as_str().unwrap();
    let value = r["protectedValue"].as_i64().unwrap();
    let funding_tree = format!("0008cd{}", hex::encode(funding_key().public_key()));
    let caps = &pair["caps"];
    let mut req: DrainRequest = serde_json::from_value(json!({
        "inputs": [
            {"role": "protected", "value": value, "ergoTree": tree, "creationHeight": 1868000,
             "tokens": [{"id": nft, "amount": 1}]},
            {"role": "attacker", "value": 2000000, "ergoTree": funding_tree, "creationHeight": 1868001}
        ],
        "outputs": [
            // The payment carries the instance's singleton: the vector contract never
            // binds its token, so without this the existing families already report
            // the NFT leaving, and the pair would not isolate the two-instance mechanism.
            {"payee": "fixed", "value": value, "ergoTree": r["paymentTree"], "creationHeight": 1868204,
             "tokens": [{"id": nft, "amount": 1}]},
            {"payee": "free", "value": 0, "ergoTree": funding_tree, "creationHeight": 1868204}
        ],
        "protocolNfts": [nft],
        "height": r["height"],
        "network": r["network"],
        "maxProbes": caps["maxProbes"],
        "maxPermutations": caps["maxPermutations"],
        "synthesis": {"multiInstance": family_on}
    }))
    .unwrap();
    req.objective.get_or_insert_with(Default::default);
    req
}

fn hunt(req: &DrainRequest) -> ergo_sandbox::drain::DrainReport {
    let req = req.clone();
    ergo_sandbox::decompile::with_large_stack(move || drain_hunt(&req)).expect("hunt runs")
}

#[test]
fn two_instance_mutant_is_found_and_control_is_not() {
    let pair = search_pair();
    let counterexample = compile(pair["counterexample"].as_str().unwrap());
    let control = compile(pair["control"].as_str().unwrap());

    // Without the family, one declared instance cannot leak: the payment
    // output must carry at least the instance's own value.
    let single = hunt(&pair_request(&counterexample, false));
    assert_eq!(
        format!("{:?}", single.verdict).to_lowercase(),
        pair["expectedWithoutFamily"]
            .as_str()
            .unwrap()
            .to_lowercase(),
        "without the family: {:?}",
        single.notes
    );
    assert!(single.synthesis.multi_instance.is_none());

    // With the family, the derived second instance lets one payment satisfy
    // both instances while the other instance's value leaves through the
    // free output: found, from the derived run, as an unsigned preflight
    // candidate.
    let found = hunt(&pair_request(&counterexample, true));
    assert_eq!(
        format!("{:?}", found.verdict).to_lowercase(),
        pair["expectedCounterexample"]
            .as_str()
            .unwrap()
            .to_lowercase(),
        "with the family: {:?}",
        found.notes
    );
    let hit = found.best.as_ref().expect("a hit");
    assert!(
        hit.shape.starts_with("two-instance(protected=0)"),
        "the hit comes from the derived run: {}",
        hit.shape
    );
    assert_eq!(
        hit.witness
            .roles
            .iter()
            .filter(|r| format!("{r:?}") == "Protected")
            .count(),
        2
    );
    // The second instance's whole value leaves: one payment discharged both.
    let leaked = pair["request"]["protectedValue"]
        .as_u64()
        .unwrap()
        .to_string();
    assert!(
        hit.accounting.extracted.values().any(|v| *v == leaked),
        "extraction {:?} should include the second instance's value {leaked}",
        hit.accounting.extracted
    );
    assert!(!found.preflight.node_validated);
    assert_eq!(found.preflight.method, "unsigned-preflight");
    assert!(found.promotion.is_none());

    // The control binds instance count; the family runs and finds nothing.
    let control_report = hunt(&pair_request(&control, true));
    assert_eq!(
        format!("{:?}", control_report.verdict).to_lowercase(),
        pair["expectedControl"].as_str().unwrap().to_lowercase(),
        "control: {:?}",
        control_report.notes
    );
    assert_eq!(control_report.hits, 0);
    let record = control_report.synthesis.multi_instance.as_ref().unwrap();
    assert!(record.runs.iter().all(|r| r.hits == 0));
    assert!(record
        .runs
        .iter()
        .any(|r| r.derived_from == Some(0) && r.probes_run > 0));
}

#[test]
fn caps_and_truncation_are_recorded() {
    let pair = search_pair();
    let counterexample = compile(pair["counterexample"].as_str().unwrap());
    let mut req = pair_request(&counterexample, true);
    // A cap of three probes across two runs: every run gets a slice, the
    // slices sum to the cap, and each run reports whether it was cut off.
    req.max_probes = Some(3);
    let report = hunt(&req);
    let record = report
        .synthesis
        .multi_instance
        .as_ref()
        .expect("family record");
    assert_eq!(record.protected_inputs, 1);
    assert_eq!(record.runs.len(), 2);
    assert_eq!(record.cap_per_run.iter().sum::<usize>(), 3);
    assert_eq!(
        report.synthesis.caps.max_probes, 3,
        "the total cap is the request's cap"
    );
    assert_eq!(
        report.probes_run,
        record.runs.iter().map(|r| r.probes_run).sum::<usize>()
    );
    assert_eq!(
        report.probes_total,
        record.runs.iter().map(|r| r.probes_total).sum::<usize>()
    );
    for (run, cap) in record.runs.iter().zip(&record.cap_per_run) {
        assert!(
            run.probes_run <= *cap,
            "{}: ran {} over its slice {cap}",
            run.label,
            run.probes_run
        );
        assert_eq!(
            run.capped,
            run.probes_run < run.probes_total,
            "{}: truncation is recorded",
            run.label
        );
    }
    assert!(report.capped, "a three-probe cap truncates the space");
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("two-instance family: the probe cap 3 was split across 2 runs")));
    assert!(record.runs[1].derived_input.is_some());
    assert_eq!(record.runs[1].derived_from, Some(0));
    assert_eq!(record.runs[1].label, "two-instance(protected=0)");
    assert!(record
        .position_in_axis_order
        .contains("unchanged pinned order"));
    // The objective and the claim label are the hunt's own, unchanged.
    assert_eq!(report.objective_version, "recognized-attacker-receipts-v1");
    assert_eq!(report.preflight.method, "unsigned-preflight");
    // And under the full cap the pair's caps are the ones the registry records.
    assert_eq!(pair["caps"]["maxProbes"], 50000);

    // Fewer probes than runs: the slices still sum to the cap; the run with no
    // budget is recorded as truncated and never run.
    let mut starved = pair_request(&counterexample, true);
    starved.max_probes = Some(1);
    let report = hunt(&starved);
    let record = report.synthesis.multi_instance.as_ref().unwrap();
    assert_eq!(record.cap_per_run, vec![1, 0]);
    assert_eq!(report.synthesis.caps.max_probes, 1);
    assert_eq!(report.probes_run, 1);
    assert_eq!(record.runs[1].probes_run, 0);
    assert_eq!(record.runs[1].oracle_calls, 0);
    assert!(record.runs[1].capped);
    assert!(report.capped);
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("no probe budget left after allocation")));
}

/// The family reports the derived box; a caller who declares it (with a
/// hypothetical id) promotes the explicit two-instance request through the
/// unchanged P06 path. The family itself never promotes: every candidate
/// input must be a declared input, and the derived box is only declared here.
#[test]
fn two_instance_candidate_promotes_when_declared() {
    let pair = search_pair();
    let counterexample = compile(pair["counterexample"].as_str().unwrap());
    let found = hunt(&pair_request(&counterexample, true));
    let record = found.synthesis.multi_instance.as_ref().unwrap();
    let derived = record.runs[1]
        .derived_input
        .clone()
        .expect("the derived box");

    // Declare every input as a hypothetical box with a canonical id.
    let spec = |b: &ergo_sandbox::scenario::ScenarioBox| -> CandidateSpec {
        serde_json::from_value(json!({
            "value": b.value, "ergoTree": b.ergo_tree, "creationHeight": b.creation_height,
            "tokens": b.tokens.iter().map(|t| json!({"id": t.id, "amount": t.amount})).collect::<Vec<_>>(),
            "registers": "00",
        }))
        .unwrap()
    };
    let reference = |i: u16| CreationReference {
        transaction_id: "bb".repeat(32),
        index: i,
    };
    let base = pair_request(&counterexample, false);
    let first = WireBox::hypothetical(&spec(&base.inputs[0].box_), &reference(0)).unwrap();
    let second = WireBox::hypothetical(&spec(&derived), &reference(1)).unwrap();
    let funding = WireBox::hypothetical(&spec(&base.inputs[1].box_), &reference(2)).unwrap();

    let mut explicit = base.clone();
    explicit.inputs[0].box_.box_id = Some(first.id().unwrap());
    explicit.inputs[1].box_.box_id = Some(funding.id().unwrap());
    let mut declared_second = derived.clone();
    declared_second.box_id = Some(second.id().unwrap());
    explicit.inputs.push(ergo_sandbox::drain::DrainInput {
        role: ergo_sandbox::drain::DrainRole::Protected,
        box_: declared_second,
    });
    explicit.protocol_nfts.push(
        record.runs[1]
            .derived_nft
            .clone()
            .expect("derived singleton"),
    );

    // The explicit request drains without the family (it is what the family found).
    let report = hunt(&explicit);
    let hit = report
        .best
        .as_ref()
        .expect("the declared two-instance shape drains");
    let candidate = &hit.witness.tx_request;
    let boxes = [first, second, funding];
    let ordered: Vec<WireBox> = candidate
        .tx
        .inputs
        .iter()
        .map(|i| {
            boxes
                .iter()
                .find(|b| b.id().unwrap() == i.box_id)
                .expect("declared")
                .clone()
        })
        .collect();
    let inputs: Vec<Input> = ordered
        .iter()
        .map(|b| Input {
            box_id: b.node().box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![], ContextExtension::empty()).unwrap(),
        })
        .collect();
    let specs: Vec<_> = candidate
        .tx
        .outputs
        .iter()
        .map(|o| candidate_spec(o).unwrap())
        .collect();
    let wire = WireTransaction::build(inputs, vec![], &specs).expect("canonical transaction");
    let base_ctx = use_recovery::recover(&repo())
        .pop()
        .expect("recovered context");
    let mut premises = base_ctx.case.premises().clone();
    premises.boxes =
        ergo_sandbox::evidence::Premise::missing("rebound from declared hypothetical material");
    premises.assumptions.remove("wireTransaction");
    let case = EvidenceCase::new(premises).unwrap();
    let case = wire.bind_case(&case, &ordered, &[]).expect("bound case");
    let material = ValidationRequest {
        format_version: base_ctx.format_version,
        case,
        transaction_bytes: hex::encode(wire.bytes()),
        parameters: base_ctx.parameters.clone(),
        network_rules: base_ctx.network_rules.clone(),
        block_context: base_ctx.block_context.clone(),
        headers: base_ctx.headers.clone(),
        local_policy: base_ctx.local_policy.clone(),
        prior_block_cost: base_ctx.prior_block_cost.clone(),
    };
    let funding_position = hit
        .witness
        .roles
        .iter()
        .position(|r| format!("{r:?}") == "Attacker")
        .unwrap();
    let key = funding_key();
    let promoted = promote(&explicit, &material, Some((funding_position, &key))).unwrap();
    let promotion = promoted.promotion.as_ref().expect("a promotion outcome");
    assert_eq!(
        promotion.failure, None,
        "promotion failed: {:?}",
        promotion.failure
    );
    assert_eq!(promotion.status, "confirmed-violation");
    assert_eq!(promotion.replay.as_ref().unwrap()["nodeValidated"], true);
    assert!(
        !promoted.preflight.node_validated,
        "the preflight record is never rewritten"
    );
}
