//! Contract test suites: a contract plus named scenarios with expected
//! verdicts, run as one unit.

use ergo_sandbox::testsuite::{run, Suite};

fn suite(json: &str) -> Suite {
    serde_json::from_str(json).expect("suite parses")
}

#[test]
fn a_suite_runs_every_scenario_against_the_compiled_contract() {
    let r = run(&suite(
        r#"{
          "source": "sigmaProp(HEIGHT > $unlockHeight)",
          "params": { "unlockHeight": { "type": "Int", "value": 1000 } },
          "scenarios": [
            { "name": "locked before", "expect": "fail", "height": 999 },
            { "name": "unlocked after", "expect": "pass", "height": 1001 },
            { "name": "wrong expectation", "expect": "pass", "height": 5 }
          ]
        }"#,
    ))
    .unwrap();
    assert_eq!(r.cases.len(), 3);
    assert_eq!(r.passed, 2);
    assert_eq!(r.failed, 1);
    assert!(r.cases[0].passed && r.cases[1].passed && !r.cases[2].passed);
    assert_eq!(r.cases[2].expected, "pass");
    assert_eq!(r.cases[2].actual, "fail");
    assert!(r.cases[1].cost > 0);
    assert!(r.tree_hex.starts_with("10"));
}

#[test]
fn a_suite_accepts_a_tree_instead_of_source() {
    let r = run(&suite(
        r#"{ "tree": "100104c801d191a37300",
             "scenarios": [ { "name": "at 200", "expect": "pass", "height": 200 } ] }"#,
    ))
    .unwrap();
    assert_eq!(r.passed, 1);
}

#[test]
fn a_thrown_script_is_an_error_verdict_the_case_can_expect() {
    let r = run(&suite(
        r#"{ "source": "sigmaProp(SELF.R4[Int].get > 0)",
             "scenarios": [
               { "name": "no register", "expect": "error", "height": 1 },
               { "name": "with register", "expect": "pass", "height": 1,
                 "selfBox": { "registers": { "R4": { "type": "Int", "value": 5 } } } }
             ] }"#,
    ))
    .unwrap();
    assert_eq!(r.failed, 0, "{:?}", r.cases);
    assert!(r.cases[0].error.as_deref().unwrap_or("").contains("None"));
}

#[test]
fn a_bad_scenario_fails_its_case_without_aborting_the_suite() {
    let r = run(&suite(
        r#"{ "source": "sigmaProp(true)",
             "scenarios": [
               { "name": "bad register", "expect": "pass", "height": 1,
                 "selfBox": { "registers": { "R4": { "type": "Int", "value": "nope" } } } },
               { "name": "fine", "expect": "pass", "height": 1 }
             ] }"#,
    ))
    .unwrap();
    assert_eq!(r.cases.len(), 2);
    assert!(!r.cases[0].passed && r.cases[0].error.is_some());
    assert!(r.cases[1].passed);
}

#[test]
fn a_contract_that_does_not_compile_fails_the_suite() {
    assert!(run(&suite(
        r#"{ "source": "sigmaProp(HEIGHT >", "scenarios": [ { "name": "x", "expect": "pass", "height": 1 } ] }"#
    ))
    .is_err());
}

#[test]
fn a_scenario_may_not_name_its_own_contract() {
    let err = run(&suite(
        r#"{ "source": "sigmaProp(true)",
             "scenarios": [ { "name": "x", "expect": "pass", "height": 1, "source": "sigmaProp(false)" } ] }"#
    ))
    .unwrap_err();
    assert!(err.to_string().contains("scenario"), "{err}");
}

#[test]
fn an_unknown_expectation_is_rejected_at_parse_time() {
    assert!(serde_json::from_str::<Suite>(
        r#"{ "source": "sigmaProp(true)", "scenarios": [ { "name": "x", "expect": "maybe", "height": 1 } ] }"#
    )
    .is_err());
}

#[test]
fn an_unknown_network_name_is_rejected() {
    let err = run(&suite(
        r#"{ "source": "sigmaProp(true)", "network": "testnet ", "scenarios": [] }"#,
    ))
    .unwrap_err();
    assert!(err.to_string().contains("network"), "{err}");
    assert!(run(&suite(
        r#"{ "source": "sigmaProp(true)", "network": "testnet", "scenarios": [] }"#
    ))
    .is_ok());
}

/// `"$self"` as an `ergoTree` in any scenario box stands for the contract
/// under test — its compiled tree is not known when the suite is written.
#[test]
fn self_placeholder_in_a_box_tree_means_the_contract_under_test() {
    let r = run(&suite(
        r#"{ "source": "sigmaProp(OUTPUTS(0).propositionBytes == SELF.propositionBytes)",
             "scenarios": [
               { "name": "same script", "expect": "pass", "height": 1,
                 "outputs": [ { "value": 1, "ergoTree": "$self" } ] },
               { "name": "other script", "expect": "fail", "height": 1,
                 "outputs": [ { "value": 1, "ergoTree": "10010101d17300" } ] }
             ] }"#,
    ))
    .unwrap();
    assert_eq!(r.failed, 0, "{:?}", r.cases);
}

/// `expectResidual` / `expectResidualExcludes`: substrings the reduced
/// proposition must (not) contain — how a suite tells "the receiver may
/// claim" from "only the funder may cancel" when both reduce to needsProof.
#[test]
fn residual_expectations_distinguish_who_may_spend() {
    let r = run(&suite(
        r#"{ "source": "$a || ($b && sigmaProp(HEIGHT > 100))",
             "params": { "a": { "type": "SigmaProp", "value": "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798" },
                         "b": { "type": "SigmaProp", "value": "028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197" } },
             "scenarios": [
               { "name": "both keys after the height", "expect": "needsProof", "expectResidual": "028333", "height": 200 },
               { "name": "only a before the height", "expect": "needsProof", "expectResidualExcludes": "028333", "height": 50 },
               { "name": "wrong expectation", "expect": "needsProof", "expectResidual": "028333", "height": 50 }
             ] }"#,
    ))
    .unwrap();
    assert!(r.cases[0].passed, "{:?}", r.cases[0]);
    assert!(r.cases[1].passed, "{:?}", r.cases[1]);
    assert!(!r.cases[2].passed, "{:?}", r.cases[2]);
    assert!(
        r.cases[2]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("residual"),
        "{:?}",
        r.cases[2]
    );
}

/// `contextVars` keys are strings in JSON ("0"); through the flattened
/// case they must still parse (serde's flatten cannot convert integer keys).
#[test]
fn context_vars_with_string_keys_parse_inside_a_suite_case() {
    let r = run(&suite(
        r#"{ "source": "sigmaProp(getVar[Int](0).get == 5)",
             "scenarios": [ { "name": "var 0 = 5", "expect": "pass", "height": 1,
                              "contextVars": { "0": { "type": "Int", "value": 5 } } } ] }"#,
    ))
    .unwrap();
    assert_eq!(r.failed, 0, "{:?}", r.cases);
}
