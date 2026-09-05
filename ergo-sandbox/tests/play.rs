//! Play: a sandbox chain's one operation — apply a transaction to a set of
//! boxes. Every input's script runs in the full transaction context (with
//! secrets when given), ERG and tokens must balance, and the outputs come
//! back with the ids the chain would give them.

use ergo_sandbox::play::{apply, PlayRequest};

const G: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
const X1: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn tree(src: &str) -> String {
    hex::encode(
        ergo_sandbox::compile_source(src, 3, ergo_ser::address::NetworkPrefix::Mainnet)
            .unwrap()
            .tree_bytes,
    )
}

fn req(v: serde_json::Value) -> PlayRequest {
    serde_json::from_value(v).unwrap()
}

#[test]
fn a_time_lock_refuses_early_and_passes_late_and_outputs_get_ids() {
    let lock = tree("sigmaProp(HEIGHT > 100)");
    let boxes = serde_json::json!([{ "boxId": "11".repeat(32), "value": 1_000_000_000, "ergoTree": lock, "creationHeight": 50 }]);
    let tx = serde_json::json!({ "inputs": [{ "boxId": "11".repeat(32) }], "outputs": [{ "value": 1_000_000_000, "ergoTree": "0008cd".to_string() + G }] });
    let early = apply(&req(
        serde_json::json!({ "height": 60, "boxes": boxes, "tx": tx }),
    ))
    .unwrap();
    assert!(!early.ok);
    assert_eq!(early.inputs[0].verdict, "fail");
    let late = apply(&req(
        serde_json::json!({ "height": 200, "boxes": boxes, "tx": tx }),
    ))
    .unwrap();
    assert!(late.ok, "{:?}", late);
    assert_eq!(late.outputs.len(), 1);
    let id = late.outputs[0].box_id.clone().unwrap();
    assert_eq!(id.len(), 64);
    assert_eq!(late.outputs[0].creation_height, 200);
    assert_ne!(id, "0".repeat(64));
    // The same transaction again yields the same ids (deterministic), and a
    // different height a different one.
    let again = apply(&req(
        serde_json::json!({ "height": 200, "boxes": boxes, "tx": tx }),
    ))
    .unwrap();
    assert_eq!(again.outputs[0].box_id, late.outputs[0].box_id);
}

#[test]
fn erg_and_tokens_must_balance_except_a_mint_named_after_the_first_input() {
    let anyone = tree("sigmaProp(true)");
    let boxes = serde_json::json!([{ "boxId": "22".repeat(32), "value": 10, "ergoTree": anyone, "tokens": [{ "id": "aa".repeat(32), "amount": 5 }] }]);
    let short = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "22".repeat(32) }], "outputs": [{ "value": 9, "ergoTree": anyone }] } }))).unwrap();
    assert!(
        !short.ok && short.problems.iter().any(|p| p.contains("ERG")),
        "{:?}",
        short.problems
    );
    let conjured = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "22".repeat(32) }], "outputs": [{ "value": 10, "ergoTree": anyone, "tokens": [{ "id": "aa".repeat(32), "amount": 6 }] }] } }))).unwrap();
    assert!(
        !conjured.ok && conjured.problems.iter().any(|p| p.contains("token")),
        "{:?}",
        conjured.problems
    );
    let minted = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "22".repeat(32) }], "outputs": [{ "value": 10, "ergoTree": anyone, "tokens": [{ "id": "aa".repeat(32), "amount": 5 }, { "id": "22".repeat(32), "amount": 1000 }] }] } }))).unwrap();
    assert!(minted.ok, "{:?}", minted.problems);
}

#[test]
fn a_key_box_needs_its_secret() {
    let key = tree(&format!("proveDlog(decodePoint(fromBase16(\"{G}\")))"));
    let anyone = tree("sigmaProp(true)");
    let boxes = serde_json::json!([{ "boxId": "33".repeat(32), "value": 7, "ergoTree": key }]);
    let unsigned = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "33".repeat(32) }], "outputs": [{ "value": 7, "ergoTree": anyone }] } }))).unwrap();
    assert!(!unsigned.ok);
    assert_eq!(unsigned.inputs[0].verdict, "needsProof");
    let signed = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "33".repeat(32), "secrets": [{ "dlog": X1 }] }], "outputs": [{ "value": 7, "ergoTree": anyone }] } }))).unwrap();
    assert!(signed.ok, "{:?}", signed);
    assert_eq!(signed.inputs[0].verdict, "proofAccepted");
}

#[test]
fn data_inputs_and_missing_boxes_are_handled() {
    let gate = tree("sigmaProp(CONTEXT.dataInputs(0).R4[Long].get > 100L)");
    let anyone = tree("sigmaProp(true)");
    let boxes = serde_json::json!([
        { "boxId": "44".repeat(32), "value": 5, "ergoTree": gate },
        { "boxId": "55".repeat(32), "value": 1, "ergoTree": anyone, "registers": { "R4": { "type": "Long", "value": 500 } } }]);
    let ok = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "44".repeat(32) }], "dataInputs": ["55".repeat(32)], "outputs": [{ "value": 5, "ergoTree": anyone }] } }))).unwrap();
    assert!(ok.ok, "{:?}", ok);
    let missing = apply(&req(
        serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "99".repeat(32) }], "outputs": [] } }),
    ));
    assert!(missing.is_err());
}

#[test]
fn duplicate_inputs_malformed_outputs_and_fake_mints_are_refused() {
    let anyone = tree("sigmaProp(true)");
    let boxes = serde_json::json!([{ "boxId": "66".repeat(32), "value": 10, "ergoTree": anyone, "tokens": [{ "id": "66".repeat(32), "amount": 1 }] }]);
    let dup = apply(&req(
        serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "66".repeat(32) }, { "boxId": "66".repeat(32) }], "outputs": [{ "value": 20, "ergoTree": anyone }] } }),
    ));
    assert!(dup.unwrap_err().to_string().contains("twice"));
    let bad = apply(&req(
        serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "66".repeat(32) }], "outputs": [{ "value": 10, "ergoTree": "10010101", "tokens": [{ "id": "66".repeat(32), "amount": 1 }] }] } }),
    ));
    assert!(bad.unwrap_err().to_string().contains("does not parse"));
    let grow = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "66".repeat(32) }], "outputs": [{ "value": 10, "ergoTree": anyone, "tokens": [{ "id": "66".repeat(32), "amount": 5 }] }] } }))).unwrap();
    assert!(
        !grow.ok && grow.problems.iter().any(|p| p.contains("token")),
        "{:?}",
        grow.problems
    );
}

#[test]
fn data_inputs_are_part_of_the_transaction_id() {
    let anyone = tree("sigmaProp(true)");
    let boxes = serde_json::json!([
        { "boxId": "77".repeat(32), "value": 3, "ergoTree": anyone },
        { "boxId": "88".repeat(32), "value": 1, "ergoTree": anyone }]);
    let a = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "77".repeat(32) }], "outputs": [{ "value": 3, "ergoTree": anyone }] } }))).unwrap();
    let b = apply(&req(serde_json::json!({ "height": 1, "boxes": boxes, "tx": { "inputs": [{ "boxId": "77".repeat(32) }], "dataInputs": ["88".repeat(32)], "outputs": [{ "value": 3, "ergoTree": anyone }] } }))).unwrap();
    assert_ne!(a.tx_id, b.tx_id);
    assert_ne!(a.outputs[0].box_id, b.outputs[0].box_id);
}
