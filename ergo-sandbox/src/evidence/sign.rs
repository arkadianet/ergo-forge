//! One explicitly owned standard P2PK funding input, signed over node bytes_to_sign.
//! Secrets are separate arguments, never request/report fields. No property claim.
use ergo_primitives::reader::VlqReader;
use ergo_ser::{
    address::build_p2pk_tree_bytes, input::SpendingProof, transaction::read_transaction,
};
use serde_json::{json, Value};

use super::{
    validate::{validate, AcceptedExecution, ValidationFailure, ValidationRequest},
    wire::{WireBox, WireTransaction},
    EvidenceCase, Premise,
};
pub use crate::prove::OwnedDlogSecret;

#[derive(Debug)]
pub enum SigningFailure {
    Material(String),
    Validation(Box<ValidationFailure>),
}
fn material(e: impl std::fmt::Display) -> SigningFailure {
    SigningFailure::Material(e.to_string())
}

/// Generate exactly one new funding proof and return only a fully validated
/// execution. Other input proofs/extensions are untouched. Imported signed
/// requests can be passed directly to `validate` without a key.
///
/// Caller metadata is preserved, not sanitized: do not put secrets in a case.
/// This API never treats caller roles/public keys as proof of key possession.
pub fn sign_owned_p2pk(
    request: &ValidationRequest,
    funding_input: usize,
    key: &OwnedDlogSecret,
) -> Result<AcceptedExecution, SigningFailure> {
    let bytes = hex::decode(&request.transaction_bytes).map_err(material)?;
    let mut reader = VlqReader::new(&bytes);
    let mut tx = read_transaction(&mut reader).map_err(material)?;
    let wire = WireTransaction::from_node(tx.clone()).map_err(material)?;
    if !reader.is_empty() || wire.bytes() != bytes {
        return Err(material(
            "signing requires exact canonical transaction bytes",
        ));
    }
    let input = tx
        .inputs
        .get(funding_input)
        .ok_or_else(|| material("funding input index out of range"))?;
    if !input.spending_proof.proof.is_empty() {
        return Err(material(
            "funding proof already present; replay it through validate",
        ));
    }
    let records = request
        .case
        .premises()
        .boxes
        .value()
        .ok_or_else(|| material("missing funding UTXO snapshot"))?;
    let mut funding = None;
    for record in records {
        let b = WireBox::from_record(record.clone()).map_err(material)?;
        if b.node().box_id().map_err(material)? == input.box_id {
            if funding.is_some() {
                return Err(material("duplicate funding UTXO"));
            }
            funding = Some(b);
        }
    }
    let funding = funding.ok_or_else(|| material("funding UTXO not found"))?;
    let standard = build_p2pk_tree_bytes(&key.public_key()).map_err(material)?;
    if funding.node().candidate.ergo_tree_bytes() != standard {
        return Err(material(
            "funding input is not the owned key's standard P2PK tree",
        ));
    }
    let mut premises = request.case.premises().clone();
    if premises.assumptions.contains_key("ownedFundingProof") {
        return Err(material(
            "signing metadata already present; construct a new request explicitly",
        ));
    }
    let previous = premises.assumptions.get("wireTransaction").cloned();
    if let Some(p) = &previous {
        let recorded = p
            .value()
            .and_then(|v| v.get("bytes"))
            .and_then(Value::as_str)
            .ok_or_else(|| material("case wire transaction lacks bytes"))?;
        if hex::decode(recorded).map_err(material)? != bytes {
            return Err(material("request disagrees with case transaction bytes"));
        }
    }
    let proof = key.prove_message(wire.bytes_to_sign()).map_err(material)?;
    tx.inputs[funding_input].spending_proof = SpendingProof::new(
        proof,
        tx.inputs[funding_input].spending_proof.extension().clone(),
    )
    .map_err(material)?;
    let signed = WireTransaction::from_node(tx).map_err(material)?;
    if signed.bytes_to_sign() != wire.bytes_to_sign() || signed.id() != wire.id() {
        return Err(material(
            "inserting a proof changed the node signing message or transaction ID",
        ));
    }
    // Replace only the explicitly checked wire attachment; retain its original
    // provenance and bytes in the signing record rather than rewriting history.
    premises.assumptions.insert(
        "ownedFundingProof".into(),
        Premise::supplied(json!({
            "formatVersion":1,"method":"ergo_wallet::proving::sigma::prove_sigma",
            "fundingInput":funding_input,"publicKey":hex::encode(key.public_key()),
            "parentRequestFingerprint":request.fingerprint(),"previousWireTransaction":previous,
            "scope":"one owned standard P2PK funding proof; no property or historical-state claim"
        })),
    );
    premises.assumptions.insert("wireTransaction".into(), Premise::supplied(json!({
        "formatVersion":1,"method":"canonical-wire-with-owned-funding-proof","nodeValidated":false,
        "bytes":hex::encode(signed.bytes()),"bytesToSign":hex::encode(signed.bytes_to_sign()),
        "transactionId":signed.id(),
    })));
    let mut signed_request = request.clone();
    signed_request.case = EvidenceCase::new(premises).map_err(material)?;
    signed_request.transaction_bytes = hex::encode(signed.bytes());
    validate(&signed_request).map_err(|e| SigningFailure::Validation(Box::new(e)))
}
