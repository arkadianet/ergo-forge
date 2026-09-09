use ergo_primitives::{digest::blake2b256, reader::VlqReader};
use ergo_sandbox::evidence::{
    wire::{BoxMaterial, CandidateSpec, CreationReference, WireBox, WireTransaction},
    CasePremises, EvidenceCase, Premise,
};
use ergo_ser::{
    input::{ContextExtension, SpendingProof},
    sigma_type::SigmaType,
    sigma_value::SigmaValue,
    transaction::{bytes_to_sign, read_transaction, transaction_id},
};
use serde_json::{json, Value};

fn fixtures() -> Value {
    serde_json::from_str(include_str!("fixtures/evidence/wire-v1.fixture")).unwrap()
}
fn policy() -> Value {
    let block = include_str!("../../docs/ROADMAP.md")
        .split("<!-- roadmap-policy:v1 -->")
        .nth(1)
        .unwrap()
        .split("```json")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    serde_json::from_str(block).unwrap()
}
fn spec(f: &Value) -> CandidateSpec {
    serde_json::from_value(f["candidate"].clone()).unwrap()
}
fn boxes(f: &Value) -> Vec<WireBox> {
    f["boxes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            WireBox::hypothetical(
                &spec(f),
                &CreationReference {
                    transaction_id: r["transactionId"].as_str().unwrap().into(),
                    index: r["index"].as_u64().unwrap().try_into().unwrap(),
                },
            )
            .unwrap()
        })
        .collect()
}
fn material(b: &WireBox) -> BoxMaterial {
    serde_json::from_value(b.record().document().clone()).unwrap()
}

#[test]
fn wire_box_id_matches_full_serialization() {
    let f = fixtures();
    assert_eq!(
        f["engineRevision"],
        ergo_sandbox::evidence::case::engine_revision()
    );
    assert_eq!(f["nodeValidated"], false);
    let bs = boxes(&f);
    assert!(
        bs.len() as u64
            >= policy()["thresholds"]["wireBoxFixturesMin"]
                .as_u64()
                .unwrap()
    );
    let mut identities = std::collections::BTreeSet::new();
    for (b, row) in bs.iter().zip(f["boxes"].as_array().unwrap()) {
        assert_eq!(hex::encode(b.bytes().unwrap()), row["bytes"]);
        assert_eq!(b.id().unwrap(), row["boxId"]);
        assert_eq!(
            hex::encode(blake2b256(&b.bytes().unwrap()).as_bytes()),
            b.id().unwrap()
        );
        identities.insert(b.id().unwrap());
        let recorded = WireBox::recorded(
            material(b),
            "fixture:wire-v1".into(),
            Some(f["engineRevision"].as_str().unwrap().into()),
        )
        .unwrap();
        let mut p = CasePremises::unspecified();
        p.boxes = Premise::supplied(vec![recorded.record().clone()]);
        let c = EvidenceCase::new(p).unwrap();
        let exported = serde_json::to_value(&c).unwrap();
        assert_eq!(
            exported["premises"]["boxes"]["value"][0]["origin"],
            "source-recorded"
        );
        assert_eq!(exported["nodeValidated"], false);
        let imported: EvidenceCase = serde_json::from_value(exported).unwrap();
        let again =
            WireBox::from_record(imported.premises().boxes.value().unwrap()[0].clone()).unwrap();
        assert_eq!(again.bytes().unwrap(), b.bytes().unwrap());
        assert_eq!(again.id().unwrap(), b.id().unwrap());
    }
    assert_eq!(identities.len(), bs.len());
    let reference = CreationReference {
        transaction_id: f["boxes"][0]["transactionId"].as_str().unwrap().into(),
        index: 1,
    };
    assert_ne!(
        bs[0].id().unwrap(),
        WireBox::hypothetical(&spec(&f), &reference)
            .unwrap()
            .id()
            .unwrap()
    );
    let reference = CreationReference {
        transaction_id: f["boxes"][1]["transactionId"].as_str().unwrap().into(),
        index: 0,
    };
    assert_ne!(
        bs[0].id().unwrap(),
        WireBox::hypothetical(&spec(&f), &reference)
            .unwrap()
            .id()
            .unwrap()
    );
    let mut wrong = material(&bs[0]);
    wrong.box_id = bs[1].id().unwrap();
    assert!(WireBox::recorded(wrong, "fixture:bad-id".into(), None).is_err());
    println!(
        "{} pinned full boxes retain bytes/IDs/references; claimed ID mismatches are rejected",
        bs.len()
    );
}

#[test]
fn transaction_id_and_message_use_node_bytes_to_sign() {
    let f = fixtures();
    let rows = f["transactions"].as_array().unwrap();
    assert!(
        rows.len() as u64
            >= policy()["thresholds"]["wireTransactionFixturesMin"]
                .as_u64()
                .unwrap()
    );
    for row in rows {
        let bytes = hex::decode(row["bytes"].as_str().unwrap()).unwrap();
        let tx = WireTransaction::from_bytes(&bytes, row["id"].as_str().unwrap()).unwrap();
        let node = read_transaction(&mut VlqReader::new(&bytes)).unwrap();
        assert_eq!(tx.bytes(), bytes);
        assert_eq!(hex::encode(tx.bytes_to_sign()), row["bytesToSign"]);
        assert_eq!(tx.bytes_to_sign(), bytes_to_sign(&node).unwrap());
        assert_eq!(
            tx.id(),
            hex::encode(transaction_id(&node).unwrap().as_bytes())
        );
        assert_ne!(
            tx.bytes(),
            tx.bytes_to_sign(),
            "nonempty dummy proof distinguishes the two encodings"
        );
        let unsigned = ergo_ser::transaction::UnsignedTransaction {
            inputs: node
                .inputs
                .iter()
                .map(|i| ergo_ser::input::UnsignedInput {
                    box_id: i.box_id,
                    extension: i.spending_proof.extension().clone(),
                })
                .collect(),
            data_inputs: node.data_inputs.clone(),
            output_candidates: node.output_candidates.clone(),
        };
        let mut w = ergo_primitives::writer::VlqWriter::new();
        ergo_ser::transaction::write_unsigned_transaction(&mut w, &unsigned).unwrap();
        assert_ne!(
            w.result(),
            tx.bytes_to_sign(),
            "unsigned wire layout is not the signing message"
        );
        let mut stale = node.clone();
        stale.output_candidates[0]
            .additional_registers
            .registers
            .clear();
        assert!(
            WireTransaction::from_node(stale).is_err(),
            "a stale node byte cache must not override parsed fields"
        );
        let mut output = spec(&f);
        output.value = 9_000_000;
        let built =
            WireTransaction::build(node.inputs.clone(), node.data_inputs.clone(), &[output])
                .unwrap();
        assert_eq!(built.bytes(), bytes);
        let outputs = tx.output_boxes().unwrap();
        assert_eq!(outputs.len(), row["outputs"].as_array().unwrap().len());
        for (i, (b, expected)) in outputs
            .iter()
            .zip(row["outputs"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(hex::encode(b.bytes().unwrap()), expected["bytes"]);
            assert_eq!(b.id().unwrap(), expected["boxId"]);
            assert_eq!(b.node().index as usize, i);
            assert_eq!(hex::encode(b.node().transaction_id.as_bytes()), tx.id());
            assert_eq!(
                serde_json::to_value(b.record()).unwrap()["origin"],
                "hypothetical"
            );
        }
        let bs = boxes(&f);
        let case = EvidenceCase::new(CasePremises::unspecified()).unwrap();
        let bound = tx.bind_case(&case, &bs[..1], &bs[1..]).unwrap();
        assert!(
            tx.bind_case(&bound, &bs[..1], &bs[1..]).is_err(),
            "never overwrite existing case material"
        );
        assert!(tx.bind_case(&case, &bs[1..], &bs[..1]).is_err());
        let saved = serde_json::to_value(&bound).unwrap();
        assert_eq!(saved["nodeValidated"], false);
        assert_eq!(saved["deploymentIdentity"], "unknown");
        assert_eq!(
            saved["premises"]["assumptions"]["wireTransaction"]["value"]["bytes"],
            row["bytes"]
        );
        let imported: EvidenceCase = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(serde_json::to_value(imported).unwrap(), saved);
        let mut changed = node.clone();
        changed.inputs[0].spending_proof.proof = vec![9, 8];
        let changed = WireTransaction::from_node(changed).unwrap();
        assert_eq!(changed.bytes_to_sign(), tx.bytes_to_sign());
        assert_eq!(changed.id(), tx.id());
        assert_ne!(changed.bytes(), tx.bytes());
        assert_ne!(
            changed
                .bind_case(&case, &bs[..1], &bs[1..])
                .unwrap()
                .fingerprint(),
            bound.fingerprint()
        );
        let mut changed = node;
        let mut extension = ContextExtension::empty();
        extension
            .values
            .insert(0, (SigmaType::SInt, SigmaValue::Int(8)));
        changed.inputs[0].spending_proof = SpendingProof::new(vec![1, 2, 3], extension).unwrap();
        let changed = WireTransaction::from_node(changed).unwrap();
        assert_ne!(changed.bytes_to_sign(), tx.bytes_to_sign());
        assert_ne!(changed.id(), tx.id());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(WireTransaction::from_bytes(&trailing, tx.id()).is_err());
        assert!(WireTransaction::from_bytes(&bytes[..bytes.len() - 1], tx.id()).is_err());
        assert!(WireTransaction::from_bytes(&bytes, &"00".repeat(32)).is_err());
    }
    println!("{} pinned transactions use node bytes_to_sign; proof/extension changes and canonical output references checked",rows.len());
}

#[test]
fn legacy_box_missing_reference_cannot_be_promoted() {
    let f = fixtures();
    let b = boxes(&f).remove(0);
    let legacy: ergo_sandbox::ScenarioBox = serde_json::from_value(
        json!({"value":10_000_000,"ergoTree":f["candidate"]["ergoTree"],"boxId":b.id().unwrap()}),
    )
    .unwrap();
    assert!(legacy
        .canonical_box()
        .unwrap_err()
        .contains("creation reference"));
    for value in [
        json!({}),
        json!({"transactionId":"11".repeat(32)}),
        json!({"index":0}),
    ] {
        assert!(serde_json::from_value::<CreationReference>(value).is_err());
    }
    let mut missing = serde_json::to_value(material(&b)).unwrap();
    missing.as_object_mut().unwrap().remove("bytes");
    assert!(serde_json::from_value::<BoxMaterial>(missing).is_err());
    let ref_new = CreationReference {
        transaction_id: "aa".repeat(32),
        index: 2,
    };
    let new = WireBox::hypothetical(&spec(&f), &ref_new).unwrap();
    assert_ne!(new.id().unwrap(), legacy.box_id.unwrap());
    assert_eq!(
        serde_json::to_value(new.record()).unwrap()["origin"],
        "hypothetical"
    );
}

#[test]
fn invalid_tree_never_becomes_empty_bytes() {
    let f = fixtures();
    let bs = boxes(&f);
    let reference = CreationReference {
        transaction_id: "11".repeat(32),
        index: 0,
    };
    for tree in ["", "zz", "10", "000101", "10010101d1730000"] {
        let mut invalid = spec(&f);
        invalid.ergo_tree = tree.into();
        assert!(invalid.build().is_err(), "{tree}");
        assert!(
            WireBox::hypothetical(&invalid, &reference).is_err(),
            "{tree}"
        );
        assert!(
            WireTransaction::build(vec![], vec![], &[invalid]).is_err(),
            "{tree}"
        );
        let mut bad = material(&bs[0]);
        bad.ergo_tree = tree.into();
        assert!(WireBox::recorded(bad, "fixture:bad-tree".into(), None).is_err());
    }
    for field in ["value", "ergoTree", "creationHeight", "tokens", "registers"] {
        let mut missing = f["candidate"].clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<CandidateSpec>(missing).is_err());
    }
    for registers in ["", "01", "0000"] {
        let mut invalid = spec(&f);
        invalid.registers = registers.into();
        assert!(invalid.build().is_err());
    }
    let mut bad = material(&bs[0]);
    bad.bytes.push_str("00");
    assert!(WireBox::recorded(bad, "fixture:trailing".into(), None).is_err());
    let mut bad = material(&bs[0]);
    bad.bytes.truncate(bad.bytes.len() - 2);
    assert!(WireBox::recorded(bad, "fixture:truncated".into(), None).is_err());
}
