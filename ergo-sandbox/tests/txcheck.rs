//! Transaction validation: run every input's script in the real context
//! (SELF at its index, all inputs, all outputs, data inputs, the extension)
//! and check value/token conservation. The question right before signing.

use ergo_sandbox::txcheck::{check, TxCheck};

fn run(json: &str) -> TxCheck {
    let req: ergo_sandbox::txcheck::TxRequest = serde_json::from_str(json).expect("request parses");
    check(&req).expect("check runs")
}

const TRUE_TREE: &str = "10010101d17300"; // sigmaProp(true)

#[test]
fn a_transaction_whose_scripts_pass_and_balances_is_valid() {
    let r = run(&format!(
        r#"{{
          "height": 1000,
          "tx": {{
            "inputs": [ {{ "boxId": "{a}" }}, {{ "boxId": "{b}" }} ],
            "dataInputs": [],
            "outputs": [ {{ "value": 1500, "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1000 }} ]
          }},
          "boxes": [
            {{ "boxId": "{a}", "value": 1000, "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }},
            {{ "boxId": "{b}", "value": 500,  "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }}
          ]
        }}"#,
        a = "aa".repeat(32),
        b = "bb".repeat(32),
        t = TRUE_TREE
    ));
    assert_eq!(r.inputs.len(), 2);
    assert!(r.inputs.iter().all(|i| i.verdict == "pass"), "{r:?}");
    assert!(r.valid, "{r:?}");
    assert!(r.problems.is_empty(), "{:?}", r.problems);
    assert_eq!(r.erg_in, 1500);
    assert_eq!(r.erg_out, 1500);
}

#[test]
fn an_input_needing_a_signature_is_reported_and_does_not_invalidate() {
    // P2PK input: needsProof — expected for an unsigned transaction.
    let pk_tree = "0008cd028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197";
    let r = run(&format!(
        r#"{{ "height": 1000,
          "tx": {{ "inputs": [ {{ "boxId": "{a}" }} ], "dataInputs": [],
                  "outputs": [ {{ "value": 100, "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1000 }} ] }},
          "boxes": [ {{ "boxId": "{a}", "value": 100, "ergoTree": "{pk}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }} ] }}"#,
        a = "aa".repeat(32),
        t = TRUE_TREE,
        pk = pk_tree
    ));
    assert_eq!(r.inputs[0].verdict, "needsProof");
    assert!(r.inputs[0]
        .reduced_to
        .as_deref()
        .unwrap()
        .contains("ProveDlog"));
    assert!(r.valid);
    assert_eq!(r.signatures_needed, 1);
}

#[test]
fn a_failing_script_invalidates_and_names_the_input() {
    // sigmaProp(HEIGHT > 2000) at height 1000.
    let r = run(&format!(
        r#"{{ "height": 1000,
          "tx": {{ "inputs": [ {{ "boxId": "{a}" }} ], "dataInputs": [],
                  "outputs": [ {{ "value": 100, "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1000 }} ] }},
          "boxes": [ {{ "boxId": "{a}", "value": 100, "ergoTree": "100104a01fd191a37300", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }} ] }}"#,
        a = "aa".repeat(32),
        t = TRUE_TREE
    ));
    assert_eq!(r.inputs[0].verdict, "fail", "{r:?}");
    assert!(!r.valid);
    assert!(
        r.problems.iter().any(|p| p.contains("input 0")),
        "{:?}",
        r.problems
    );
}

#[test]
fn value_and_token_conservation_are_checked() {
    let tok = "cc".repeat(32);
    let r = run(&format!(
        r#"{{ "height": 1000,
          "tx": {{ "inputs": [ {{ "boxId": "{a}" }} ], "dataInputs": [],
                  "outputs": [ {{ "value": 150, "ergoTree": "{t}", "assets": [ {{ "tokenId": "{tok}", "amount": 5 }} ], "additionalRegisters": {{}}, "creationHeight": 1000 }} ] }},
          "boxes": [ {{ "boxId": "{a}", "value": 100, "ergoTree": "{t}", "assets": [ {{ "tokenId": "{tok}", "amount": 3 }} ], "additionalRegisters": {{}}, "creationHeight": 1 }} ] }}"#,
        a = "aa".repeat(32),
        t = TRUE_TREE,
        tok = tok
    ));
    assert!(!r.valid);
    assert!(
        r.problems
            .iter()
            .any(|p| p.contains("ERG") && p.contains("150") && p.contains("100")),
        "{:?}",
        r.problems
    );
    assert!(
        r.problems
            .iter()
            .any(|p| p.contains("token") && p.contains("cc")),
        "{:?}",
        r.problems
    );
}

#[test]
fn minting_one_new_token_with_the_first_input_id_is_allowed() {
    let a = "aa".repeat(32);
    let r = run(&format!(
        r#"{{ "height": 1000,
          "tx": {{ "inputs": [ {{ "boxId": "{a}" }} ], "dataInputs": [],
                  "outputs": [ {{ "value": 100, "ergoTree": "{t}", "assets": [ {{ "tokenId": "{a}", "amount": 1000 }} ], "additionalRegisters": {{}}, "creationHeight": 1000 }} ] }},
          "boxes": [ {{ "boxId": "{a}", "value": 100, "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }} ] }}"#,
        a = a,
        t = TRUE_TREE
    ));
    assert!(r.valid, "{:?}", r.problems);
}

#[test]
fn the_extension_reaches_the_script_as_context_variables() {
    // sigmaProp(getVar[Int](0).get == 5); extension {"0": "040a"} (Int 5).
    let r = run(&format!(
        r#"{{ "height": 1000,
          "tx": {{ "inputs": [ {{ "boxId": "{a}", "extension": {{ "0": "040a" }} }} ], "dataInputs": [],
                  "outputs": [ {{ "value": 100, "ergoTree": "{t}", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1000 }} ] }},
          "boxes": [ {{ "boxId": "{a}", "value": 100, "ergoTree": "1001040ad193e4e300047300", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }} ] }}"#,
        a = "aa".repeat(32),
        t = TRUE_TREE
    ));
    assert_eq!(r.inputs[0].verdict, "pass", "{r:?}");
}

#[test]
fn a_missing_input_box_is_a_problem_not_a_panic() {
    let r = run(&format!(
        r#"{{ "height": 1000,
          "tx": {{ "inputs": [ {{ "boxId": "{a}" }} ], "dataInputs": [], "outputs": [] }},
          "boxes": [] }}"#,
        a = "aa".repeat(32)
    ));
    assert!(!r.valid);
    assert_eq!(r.inputs[0].verdict, "missing");
}
