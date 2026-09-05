//! The method sweep: every method of every ErgoScript type, exercised once
//! against a fully specified scenario with expectations computed
//! independently (`examples/tests/gen/methods.py`). A case that errors or
//! evaluates false is a finding about the node's evaluator or compiler —
//! the first run of this sweep found `CONTEXT.headers.size` erroring, a
//! source-map assertion on constant collections, and three sandbox gaps
//! (empty box bytes, the tree header version, the SELF input's extension).

use ergo_sandbox::testsuite::{self, Suite};

#[test]
fn every_method_evaluates_as_expected() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../examples/tests/method-sweep.json")).unwrap();
    let mut failures = Vec::new();
    for c in &cases {
        let mut scenario = c["scenario"].clone();
        scenario["name"] = c["name"].clone();
        scenario["expect"] = serde_json::json!("pass");
        let suite: Suite = serde_json::from_value(serde_json::json!({
            "source": c["source"], "treeVersion": c["treeVersion"], "scenarios": [scenario]
        }))
        .unwrap();
        match testsuite::run(&suite) {
            Ok(r) if r.failed == 0 => {}
            Ok(r) => failures.push(format!(
                "{}: {:?}",
                c["name"],
                r.cases[0]
                    .error
                    .clone()
                    .unwrap_or_else(|| r.cases[0].actual.to_string())
            )),
            Err(e) => failures.push(format!("{}: {e}", c["name"])),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
