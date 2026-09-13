//! Catalogue integrity and authored counterexamples; no deployment verdicts.
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use ergo_sandbox::testsuite::{run, Suite};
use serde_json::Value;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn catalogue() -> Value {
    read_json(&root().join("docs/security/vectors.json"))
}

fn example_path(path: &str) -> PathBuf {
    let relative = Path::new(path);
    assert!(relative.starts_with("examples"), "{path}");
    assert!(
        relative
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "{path}"
    );
    let full = root().join(relative).canonicalize().unwrap();
    assert!(full.starts_with(root().join("examples").canonicalize().unwrap()));
    assert!(full.is_file(), "{path}");
    full
}

fn kebab(value: &str) -> bool {
    value.split('-').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}

fn verify_pair(row: &Value, paths: &mut BTreeSet<PathBuf>) {
    let Some(pair) = row.get("suites") else {
        return;
    };
    let mut cases = Vec::new();
    for (role, source_name) in [("vulnerable", "vulnerable.es"), ("fixed", "fixed.es")] {
        let path = example_path(pair[role].as_str().unwrap());
        assert!(
            paths.insert(path.clone()),
            "duplicate suite {}",
            path.display()
        );
        let mut value = read_json(&path);
        assert_eq!(
            value["source"].as_str().unwrap(),
            std::fs::read_to_string(path.parent().unwrap().join(source_name)).unwrap()
        );
        let suite: Suite = serde_json::from_value(value.clone()).unwrap();
        let result = run(&suite).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(result.failed, 0, "{}: {:?}", path.display(), result.cases);
        let scenarios = value["scenarios"].as_array_mut().unwrap();
        assert!(scenarios.len() >= 2);
        // Explicit counterexample and ordinary-spend expectations are independent
        // of the reducer's results. Both input indices appear for composition.
        let last = scenarios.len() - 1;
        for (i, case) in scenarios.iter_mut().enumerate() {
            let expected = if role == "vulnerable" || i == last {
                "pass"
            } else {
                "fail"
            };
            assert_eq!(case["expect"], expected, "{}", path.display());
            case.as_object_mut().unwrap().remove("expect");
        }
        cases.push(value["scenarios"].clone());
    }
    assert_eq!(cases[0], cases[1], "{}: pair contexts differ", row["id"]);
}

fn suite_paths(directory: &Path, found: &mut BTreeSet<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            suite_paths(&path, found);
        } else if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".test.json")
        {
            found.insert(path.canonicalize().unwrap());
        }
    }
}

#[test]
fn every_vector_has_examples_or_is_marked_manual() {
    let data = catalogue();
    assert_eq!(data["schemaVersion"], 1);
    let classes: BTreeSet<_> = data["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(classes.len(), 12);
    assert_eq!(data["classes"].as_array().unwrap().len(), 12);
    let mut represented = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut tested_suites = BTreeSet::new();
    for row in data["vectors"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        assert!(kebab(id) && ids.insert(id), "invalid/duplicate id {id}");
        let class = row["class"].as_str().unwrap();
        assert!(classes.contains(class), "{id}: unknown class");
        represented.insert(class);
        for field in ["title", "mechanism", "notes"] {
            assert!(
                !row[field].as_str().unwrap().trim().is_empty(),
                "{id}: {field}"
            );
        }
        assert!(
            (2..=5).contains(&row["mechanism"].as_str().unwrap().split(". ").count()),
            "{id}: mechanism needs two to five sentences"
        );
        // All initial rows, including manual ones, carry a positive and control.
        for role in ["exhibits", "control"] {
            let examples = row["examples"][role].as_array().unwrap();
            assert!(!examples.is_empty(), "{id}: missing {role}");
            for example in examples {
                assert!(!example["notes"].as_str().unwrap().trim().is_empty());
                let path = example_path(example["path"].as_str().unwrap());
                if let Some(pointer) = example.get("pointer") {
                    let doc = read_json(&path);
                    assert!(
                        !doc.pointer(pointer.as_str().unwrap())
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .is_empty(),
                        "{id}: absent contract at pointer"
                    );
                } else {
                    assert_eq!(
                        path.extension().unwrap(),
                        "es",
                        "{id}: JSON examples must identify their contract with a pointer"
                    );
                }
                if path.starts_with(root().join("examples/contracts/vectors")) {
                    assert!(
                        row.get("suites").is_some(),
                        "{id}: authored pair needs suites"
                    );
                }
            }
        }
        if row["instrument"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i == "manual")
        {
            assert!(
                row["notes"].as_str().unwrap().contains("reviewer"),
                "{id}: describe the manual check"
            );
        }
        verify_pair(row, &mut tested_suites);
    }
    assert_eq!(represented, classes);
    let mut all_suites = BTreeSet::new();
    suite_paths(&root().join("examples/contracts/vectors"), &mut all_suites);
    assert_eq!(
        tested_suites, all_suites,
        "every authored suite must be catalogued and executed"
    );
    let rendered = Command::new("python3")
        .args(["scripts/render_vectors.py", "--check"])
        .current_dir(root())
        .output()
        .expect("Python renderer available");
    assert!(
        rendered.status.success(),
        "{}{}",
        String::from_utf8_lossy(&rendered.stdout),
        String::from_utf8_lossy(&rendered.stderr)
    );
}

#[test]
fn every_named_instrument_exists() {
    // Read the production registry rather than keeping a second list of lint IDs.
    let registry = include_str!("../src/audit/mod.rs")
        .split("const LINTS:")
        .nth(1)
        .unwrap()
        .split("= &[")
        .nth(1)
        .unwrap()
        .split("];")
        .next()
        .unwrap();
    let mut lint_ids = BTreeSet::new();
    for line in registry.lines() {
        if let Some(name) = line.trim().strip_prefix("lints::") {
            let name = name.trim_end_matches(',');
            let source = std::fs::read_to_string(
                root().join(format!("ergo-sandbox/src/audit/lints/{name}.rs")),
            )
            .unwrap();
            let id = source
                .split("lint: \"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            lint_ids.insert(id.to_owned());
        }
    }
    assert!(!lint_ids.is_empty());
    for row in catalogue()["vectors"].as_array().unwrap() {
        let instruments = row["instrument"].as_array().unwrap();
        assert!(
            !instruments.is_empty(),
            "{}: no instrument or manual marker",
            row["id"]
        );
        let mut seen = BTreeSet::new();
        for instrument in instruments {
            let name = instrument.as_str().unwrap();
            assert!(seen.insert(name), "duplicate instrument {name}");
            if let Some(id) = name.strip_prefix("lint:") {
                assert!(lint_ids.contains(id), "{}: unknown lint {id}", row["id"]);
            } else {
                assert!(
                    matches!(
                        name,
                        "hunt" | "drain" | "scenario" | "node-validated" | "manual"
                    ),
                    "{}: unknown instrument {name}",
                    row["id"]
                );
            }
        }
    }
}
