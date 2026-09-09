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
