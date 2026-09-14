//! Independent authored scenarios: expectations predate compilation/reduction.
use ergo_sandbox::compile::{compile_with_params, scan_params, template_doc};
use ergo_sandbox::testsuite::{run, Expect, Suite};
use serde_json::Value;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap()
}
fn records() -> Vec<Value> {
    serde_json::from_str::<Value>(&read("examples/mutants/recipes.json")).unwrap()["recipeMutants"]
        .as_array()
        .unwrap()
        .clone()
}
fn suite(row: &Value) -> Suite {
    serde_json::from_str(&read(row["suite"].as_str().unwrap())).unwrap()
}

#[test]
fn new_recipes_have_independent_expectations() {
    for row in records().iter().take(2) {
        let suite = suite(row);
        let source = read(row["control"].as_str().unwrap());
        assert_eq!(suite.source.as_deref(), Some(source.as_str()));
        assert_eq!(suite.network.as_deref(), Some("mainnet"));
        assert!(
            suite.tree_version.is_none(),
            "exercise workbench default version 3"
        );
        let doc = template_doc(&source).expect("Build recipe walkthrough");
        assert!(doc.description.contains("Walkthrough:"));
        assert!(!scan_params(&source).is_empty());
        let required = if row["id"] == "pool-bound-swap" {
            vec![
                ("honest swap", Expect::Pass),
                ("decoy at pool position", Expect::Fail),
                ("reserve product falls", Expect::Fail),
                ("extra output", Expect::Fail),
            ]
        } else {
            vec![
                ("honest preservation", Expect::Pass),
                ("successor drops value floor", Expect::Fail),
                ("successor below configured floor", Expect::Fail),
                ("successor changes R4", Expect::Fail),
                ("successor drops R5", Expect::Fail),
                ("decoy successor NFT", Expect::Fail),
            ]
        };
        for (name, expected) in required {
            assert_eq!(
                suite
                    .scenarios
                    .iter()
                    .find(|c| c.name == name)
                    .expect(name)
                    .expect,
                expected
            );
        }
        let result = run(&suite).expect("compile and reduce independent suite");
        assert_eq!(result.failed, 0, "{}: {:?}", row["id"], result.cases);
        assert_eq!(
            serde_json::to_value(result).unwrap()["nodeValidated"],
            false
        );
    }
}

#[test]
fn new_recipe_mutants_are_caught() {
    for row in records() {
        let mut suite = suite(&row);
        let source = read(row["control"].as_str().unwrap());
        let find = row["diff"]["find"].as_str().unwrap();
        assert_eq!(source.matches(find).count(), 1);
        let mutant = source.replacen(find, row["diff"]["replace"].as_str().unwrap(), 1);
        if let Some(path) = row["mutant"].as_str() {
            assert_eq!(read(path), mutant);
        }
        for (src, is_mutant) in [(&source, false), (&mutant, true)] {
            suite.source = Some(src.clone());
            let result = run(&suite).unwrap();
            let binding = result
                .cases
                .iter()
                .find(|c| c.name == row["bindingCase"])
                .unwrap();
            assert_eq!(binding.expected, "fail");
            assert_eq!(binding.actual, if is_mutant { "pass" } else { "fail" });
            assert_eq!(binding.passed, !is_mutant);
            // A thrown exception is not evidence that this mutation was caught.
            assert_eq!(
                result.failed,
                usize::from(is_mutant),
                "{}: {:?}",
                row["id"],
                result.cases
            );
            let compiled = compile_with_params(
                src,
                &suite.params,
                3,
                ergo_ser::address::NetworkPrefix::Mainnet,
            )
            .unwrap();
            for inline in [false, true] {
                let audit = ergo_sandbox::audit::audit(&ergo_sandbox::lift_tree(
                    &compiled.ergo_tree,
                    inline,
                ));
                assert_eq!(
                    audit.completeness,
                    ergo_sandbox::audit::Completeness::Complete
                );
                if is_mutant {
                    let findings: Vec<_> = audit
                        .findings
                        .iter()
                        .filter(|f| f.lint == row["lint"])
                        .collect();
                    assert_eq!(findings.len(), 1, "{}: {:?}", row["id"], audit.findings);
                    let f = serde_json::to_value(findings[0]).unwrap();
                    assert_eq!(f["nodeValidated"], false);
                    assert_eq!(f["severityMeaning"], "review-priority");
                    assert!(findings[0].ir_id.is_some());
                } else {
                    assert!(
                        audit.findings.is_empty(),
                        "{}: {:?}",
                        row["id"],
                        audit.findings
                    );
                }
            }
        }
    }
}
