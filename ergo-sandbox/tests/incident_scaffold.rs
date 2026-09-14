use ergo_sandbox::incident::{scaffold, PLACEHOLDER};
use ergo_sandbox::map::fixture::Fixture;
use ergo_sandbox::map::source::TxBoxes;
use ergo_sandbox::testsuite::Suite;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

const TX: &str = "5371373d346aade57f684ead23f386e3441ffb2cb7a860babe96e3c5048b2725";
fn committed(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(format!("examples/incidents/use-lp-drain.{name}.test.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

// The committed USE suites lack boxId; their inputs also lack creationHeight. Supply conspicuously
// synthetic metadata only for those fields; equality below removes exactly
// those two fields. Everything else, including raw register hex, is original.
fn fixture() -> Fixture {
    let original = committed("deployed-swap");
    let case = &original["scenarios"][0];
    let convert = |key: &str| -> Vec<Value> {
        case[key]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let mut b = b.clone();
                b["boxId"] = json!(format!(
                    "{:064x}",
                    i + if key == "inputs" { 1 } else { 101 }
                ));
                b.as_object_mut()
                    .unwrap()
                    .entry("creationHeight")
                    .or_insert(json!(123));
                if let Some(regs) = b.as_object_mut().unwrap().remove("registers") {
                    b["additionalRegisters"] = regs
                        .as_object()
                        .unwrap()
                        .iter()
                        .map(|(k, v)| (k.clone(), v["value"].clone()))
                        .collect::<serde_json::Map<_, _>>()
                        .into();
                }
                b
            })
            .collect()
    };
    let tx: TxBoxes = serde_json::from_value(json!({"inputs": convert("inputs"),
        "outputs": convert("outputs"), "dataInputs": [], "inclusionHeight": case["height"]}))
    .unwrap();
    let mut source = Fixture::new("fixture", None, 1999999); // deliberately not spending height
    source.transactions.insert(TX.into(), tx);
    // Exercise the archive's actual JSON path, not just an in-memory stub.
    Fixture::from_json(&source.to_json().unwrap()).unwrap()
}
fn use_projection(boxes: &Value, original: &Value) -> Value {
    let mut boxes = boxes.clone();
    for (b, original) in boxes
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(original.as_array().unwrap())
    {
        for field in ["boxId", "creationHeight"] {
            if original.get(field).is_none() {
                b.as_object_mut().unwrap().remove(field);
            }
        }
    }
    boxes
}

#[test]
fn incident_scaffold_reproduces_use_boxes() {
    let draft = scaffold(&fixture(), TX, "mainnet").unwrap();
    assert_eq!(
        draft.suites.len(),
        3,
        "includes the decoy's distinct script"
    );
    for name in ["deployed-swap", "deployed-pool"] {
        let original = committed(name);
        let suite = &draft
            .suites
            .iter()
            .find(|s| s.document["tree"] == original["tree"])
            .unwrap()
            .document;
        assert_eq!(suite["scenarios"].as_array().unwrap().len(), 1);
        let case = &suite["scenarios"][0];
        assert_eq!(case["selfIndex"], original["scenarios"][0]["selfIndex"]);
        assert_eq!(case["height"], 1868204);
        for key in ["inputs", "outputs"] {
            let projected = use_projection(&case[key], &original["scenarios"][0][key]);
            // Serialized JSON byte equality preserves array/token order, exact
            // integers, field set, and every hex character (including case).
            assert_eq!(
                serde_json::to_vec(&projected).unwrap(),
                serde_json::to_vec(&original["scenarios"][0][key]).unwrap(),
                "{name} {key}"
            );
            assert_eq!(projected, committed("fixed-swap")["scenarios"][0][key]);
        }
        assert_eq!(case["dataInputs"], json!([]));
    }
}

#[test]
fn incident_scaffold_never_fills_expectations() {
    let mut source = fixture();
    for known_height in [true, false] {
        if !known_height {
            source.transactions.get_mut(TX).unwrap().inclusion_height = None;
        }
        let draft = scaffold(&source, TX, "testnet").unwrap();
        let out = std::env::temp_dir().join(format!(
            "forge-b9-incident-{}-{known_height}",
            std::process::id()
        ));
        draft.write_to(&out).unwrap();
        assert!(
            draft.write_to(&out).is_err(),
            "never overwrite authored files"
        );
        for suite in &draft.suites {
            assert_eq!(suite.document["network"], "testnet");
            for c in suite.document["scenarios"].as_array().unwrap() {
                assert_eq!(c["expect"], PLACEHOLDER);
                if !known_height {
                    assert_eq!(c["height"], PLACEHOLDER);
                }
            }
            assert!(serde_json::from_value::<Suite>(suite.document.clone()).is_err());
            let result = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
                .arg("test")
                .arg(out.join(&suite.directory).join("contract.test.json"))
                .output()
                .unwrap();
            assert!(!result.status.success());
            assert!(String::from_utf8_lossy(&result.stderr).contains(PLACEHOLDER));
        }
        assert_eq!(draft.triage["inputIndex"], Value::Null);
        assert_eq!(draft.triage["drain"]["inputs"], json!([]));
        assert!(draft.readme.contains("nodeValidated: false"));
        if !known_height {
            assert!(draft.readme.contains("MISSING SPENDING HEIGHT"));
        }
        std::fs::remove_dir_all(out).unwrap();
    }
}

#[test]
fn repeated_scripts_and_data_inputs_keep_original_positions_and_hex() {
    let mut source = fixture();
    let tx = source.transactions.get_mut(TX).unwrap();
    let mut repeated = tx.inputs[1].clone();
    repeated.box_id = "AB".repeat(32);
    repeated.ergo_tree = repeated.ergo_tree.to_uppercase();
    tx.inputs.push(repeated.clone());
    repeated.registers.insert("R4".into(), "0e02ABcd".into());
    tx.data_inputs = vec![repeated];
    let draft = scaffold(&source, TX, "mainnet").unwrap();
    assert_eq!(draft.suites.len(), 3);
    let cases = draft.suites[1].document["scenarios"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0]["selfIndex"], 1);
    assert_eq!(cases[1]["selfIndex"], 3);
    for case in cases {
        assert_eq!(case["inputs"][3]["boxId"], "AB".repeat(32));
        assert_eq!(
            case["dataInputs"][0]["registers"]["R4"]["value"],
            "0e02ABcd"
        );
        assert_eq!(case["dataInputs"][0]["creationHeight"], 123);
    }
    assert!(scaffold(&source, "../bad", "mainnet").is_err());
    assert!(scaffold(&source, TX, "unknown").is_err());
}

#[test]
fn incident_cli_rejects_incomplete_options_before_network_access() {
    for args in [
        vec![TX],
        vec![TX, "--explorer"],
        vec![TX, "--wat", "value"],
        vec![TX, "--explorer", "file:///tmp/no"],
        vec![
            TX,
            "--explorer",
            "http://unused.invalid",
            "--network",
            "invalid",
        ],
        vec!["bad-tx", "--explorer", "http://unused.invalid"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
            .arg("incident")
            .args(args)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("request failed"));
    }
}
