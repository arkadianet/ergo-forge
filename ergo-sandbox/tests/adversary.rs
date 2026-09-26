use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};

fn draft() -> Value {
    let box_id = format!("{:0>64}", "b0");
    json!({
        "height": 1000,
        "network": "mainnet",
        "boxes": [{
            "boxId": box_id.clone(),
            "value": 1000000,
            "ergoTree": "10010101d17300",
            "tokens": [],
            "registers": {}
        }],
        "tx": {
            "inputs": [{"boxId": box_id}],
            "dataInputs": [],
            "outputs": [{"value": 1000000, "ergoTree": "10010101d17300"}]
        }
    })
}

fn request(options: Value) -> ergo_sandbox::adversary::SearchRequest {
    let mut value = draft();
    value["options"] = options;
    serde_json::from_value(value).unwrap()
}

#[test]
fn the_same_seed_and_draft_produce_the_same_trace_fingerprint() {
    let req = request(json!({"seed": "fixture-a", "maxDepth": 2, "maxProbes": 8}));
    let first = ergo_sandbox::adversary::search(&req).unwrap();
    let second = ergo_sandbox::adversary::search(&req).unwrap();
    assert_eq!(first.fingerprint, second.fingerprint);
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );
}

#[test]
fn a_probe_cap_is_visible_and_never_becomes_a_safety_claim() {
    let req = request(json!({"seed": "capped", "maxDepth": 1, "maxProbes": 1, "maxOpsPerStep": 1}));
    let report = ergo_sandbox::adversary::search(&req).unwrap();
    assert_eq!(report.probes, 1);
    assert!(report.truncated);
    assert_eq!(
        report.verdict,
        ergo_sandbox::adversary::SearchVerdict::NoFlipUnderProbes
    );
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("cap") || n.contains("probe")));
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("\"safe\""));
    assert!(!text.contains("confirmed-violation"));
}

#[test]
fn misplaced_search_knobs_are_refused() {
    let mut value = draft();
    value["seed"] = json!("top-level");
    let error = serde_json::from_value::<ergo_sandbox::adversary::SearchRequest>(value)
        .expect_err("top-level knobs must not be silently ignored");
    assert!(error.to_string().contains("options"));
}

#[test]
fn a_carried_step_compares_against_its_parent_by_lineage() {
    let compiled =
        ergo_sandbox::compile_source("sigmaProp(HEIGHT == 1000L)", 3, NetworkPrefix::Mainnet)
            .unwrap();
    let tree = hex::encode(&compiled.tree_bytes);
    let box_id = format!("{:0>64}", "b0");
    let value = json!({
        "height": 1000,
        "network": "mainnet",
        "boxes": [{"boxId": box_id, "value": 1000000, "ergoTree": tree, "tokens": [], "registers": {}}],
        "tx": {"inputs": [{"boxId": box_id}], "dataInputs": [], "outputs": [{"value": 1000000, "ergoTree": tree}]},
        "options": {"seed": "height", "maxDepth": 2, "maxProbes": 32, "maxOpsPerStep": 2}
    });
    let request: ergo_sandbox::adversary::SearchRequest = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&request).unwrap();
    assert_eq!(report.depth_reached, 2);
    assert_eq!(
        report.verdict,
        ergo_sandbox::adversary::SearchVerdict::Flipped
    );
    let hit = report.best.expect("a carried height change is found");
    assert_eq!(hit.depth, 2);
    assert!(hit
        .flipped
        .iter()
        .any(|flip| flip.before == "pass" && flip.after == "fail"));
}

#[test]
fn a_large_reorder_axis_reports_a_lower_bound_without_panicking() {
    let boxes: Vec<Value> = (0..35)
        .map(|i| {
            json!({
                "boxId": format!("{:0>64}", format!("b{i}")),
                "value": 1,
                "ergoTree": "10010101d17300",
                "tokens": [],
                "registers": {}
            })
        })
        .collect();
    let inputs: Vec<Value> = boxes.iter().map(|b| json!({"boxId": b["boxId"]})).collect();
    let mut value = json!({
        "height": 1000,
        "boxes": boxes,
        "tx": {"inputs": inputs, "dataInputs": [], "outputs": [{"value": 35, "ergoTree": "10010101d17300"}]},
        "options": {"maxDepth": 1, "maxProbes": 2}
    });
    value["options"]["seed"] = json!("large-axis");
    let request: ergo_sandbox::adversary::SearchRequest = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&request).unwrap();
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("reportable range") || note.contains("2^128")));
}

#[test]
fn conservation_does_not_wrap_a_large_erg_sum() {
    let boxes: Vec<Value> = (0..3)
        .map(|i| {
            json!({
                "boxId": format!("{:0>64}", format!("b{i}")),
                "value": 9_000_000_000_000_000_000i64,
                "ergoTree": "10010101d17300",
                "tokens": [],
                "registers": {}
            })
        })
        .collect();
    let inputs: Vec<Value> = boxes.iter().map(|b| json!({"boxId": b["boxId"]})).collect();
    let request: ergo_sandbox::play::PlayRequest = serde_json::from_value(json!({
        "height": 1000,
        "boxes": boxes,
        "tx": {
            "inputs": inputs,
            "dataInputs": [],
            "outputs": [{"value": 9_000_000_000_000_000_000i64, "ergoTree": "10010101d17300"}]
        }
    }))
    .unwrap();
    let result = ergo_sandbox::play::apply(&request).unwrap();
    assert!(result
        .problems
        .iter()
        .any(|p| p.contains("ERG not conserved")));
    assert!(!result.ok);
}

#[test]
fn adversary_reports_are_synthetic_and_never_node_validated() {
    let req = request(json!({"seed": "synthetic", "maxDepth": 1, "maxProbes": 4}));
    let report = ergo_sandbox::adversary::search(&req).unwrap();
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["nodeValidated"], json!(false));
    assert_eq!(value["method"], json!("adversary-search"));
    assert!(value["caps"]["maxDepth"].as_u64().unwrap() <= 4);
}
