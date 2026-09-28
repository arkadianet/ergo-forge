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
fn a_carried_step_compares_against_its_unmutated_draft() {
    let compiled = ergo_sandbox::compile_source(
        "sigmaProp(SELF.tokens(0)._2 == 5L)",
        3,
        NetworkPrefix::Mainnet,
    )
    .unwrap();
    let mut value = draft();
    let tokens = json!([{"id": "aa".repeat(32), "amount": 5}]);
    value["boxes"][0]["tokens"] = tokens.clone();
    value["tx"]["outputs"][0]["tokens"] = tokens;
    value["tx"]["outputs"][0]["ergoTree"] = json!(hex::encode(compiled.tree_bytes));
    value["options"] =
        json!({"seed": "height", "maxDepth": 2, "maxProbes": 128, "maxOpsPerStep": 1});
    let request: ergo_sandbox::adversary::SearchRequest = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&request).unwrap();
    assert_eq!(report.depth_reached, 2);
    let hit = report
        .best
        .expect("a token mutation at the carried guard is found");
    assert_eq!(hit.depth, 2);
    assert!(hit
        .flipped
        .iter()
        .any(|f| f.before == "pass" && f.after == "fail"));
    // Recreate each first-depth step's carried output draft and match the
    // digest recorded for the witness's parent.
    let digest = &report.steps[hit.step - 1].draft_digest;
    let carried = report
        .steps
        .iter()
        .filter(|s| s.depth == 1 && s.ok)
        .find_map(|step| {
            let result = ergo_sandbox::attack::apply_attack(&ergo_sandbox::attack::AttackRequest {
                draft: request.draft.clone(),
                operations: step.ops.clone(),
            })
            .ok()?
            .result;
            let mut value = serde_json::to_value(&request.draft).unwrap();
            value["height"] = json!(1001);
            value["boxes"] = serde_json::to_value(&result.outputs).unwrap();
            value["tx"]["inputs"] = json!(result
                .outputs
                .iter()
                .map(|b| json!({"boxId": b.box_id}))
                .collect::<Vec<_>>());
            value["tx"]["outputs"] = json!(result
                .outputs
                .iter()
                .map(|b| {
                    let mut b = serde_json::to_value(b).unwrap();
                    b["boxId"] = Value::Null;
                    b["creationHeight"] = json!(0);
                    b
                })
                .collect::<Vec<_>>());
            let carried: ergo_sandbox::play::PlayRequest = serde_json::from_value(value).unwrap();
            let actual =
                ergo_sandbox::evidence::case::json_digest(&serde_json::to_value(&carried).unwrap());
            (actual == *digest).then_some(carried)
        })
        .expect("the witness names a reproducible carried draft");
    let replay = ergo_sandbox::attack::apply_attack(&ergo_sandbox::attack::AttackRequest {
        draft: carried,
        operations: hit.minimal_ops,
    })
    .unwrap();
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(hit.witness).unwrap()
    );
}

#[test]
fn a_signed_draft_does_not_flip_when_carrying_drops_its_proof() {
    let mut value = draft();
    let key = "0008cd0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    value["boxes"][0]["ergoTree"] = json!(key);
    value["tx"]["outputs"][0]["ergoTree"] = json!(key);
    value["tx"]["inputs"][0]["secrets"] = json!([{"dlog": format!("{:0>64}", "1")}]);
    value["options"] = json!({"seed": "signed", "maxProbes": 64});
    let request = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&request).unwrap();
    assert_eq!(
        report.verdict,
        ergo_sandbox::adversary::SearchVerdict::NoFlipUnderProbes
    );
    assert!(report.best.is_none());
    assert!(report.rejections.missing_key > 0);
    // The carried draft lost its proof but is still a parent: depth two runs.
    assert!(report.steps.iter().any(|s| s.depth == 2), "{report:?}");
}

#[test]
fn invalid_reducer_calls_spend_probes() {
    let mut value = draft();
    let material = b"ergo-forge/decoy-token/decoy-box/input-0/0";
    let id = hex::encode(ergo_primitives::digest::blake2b256(material).as_bytes());
    value["boxes"][0]["boxId"] = json!(id);
    value["tx"]["inputs"][0]["boxId"] = value["boxes"][0]["boxId"].clone();
    value["options"] =
        json!({"seed": "invalid", "maxDepth": 1, "maxProbes": 64, "maxOpsPerStep": 1});
    let req = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&req).unwrap();
    let calls = report
        .steps
        .iter()
        .filter(|s| s.status != "refused")
        .count();
    assert!(
        report.steps.iter().any(|s| s.status == "invalid"),
        "{report:?}"
    );
    assert_eq!(report.probes, 1 + calls);
}

#[test]
fn register_tamper_candidates_change_the_draft() {
    let mut value = draft();
    value["boxes"][0]["registers"] = json!({"R4": {"type": "Long", "value": 42}});
    value["tx"]["outputs"][0]["registers"] = value["boxes"][0]["registers"].clone();
    value["options"] = json!({"maxDepth": 1, "maxProbes": 64, "maxOpsPerStep": 1});
    let request: ergo_sandbox::adversary::SearchRequest = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&request).unwrap();
    let mut edits = 0;
    for step in report.steps {
        for op in step.ops {
            if let ergo_sandbox::attack::AttackOp::TamperField {
                ref field,
                ref value,
                ..
            } = op
            {
                if field == "R4" {
                    assert_eq!(value["type"], "Long");
                    assert_ne!(value["value"], 42);
                    let result =
                        ergo_sandbox::attack::apply_attack(&ergo_sandbox::attack::AttackRequest {
                            draft: request.draft.clone(),
                            operations: vec![op],
                        })
                        .unwrap();
                    assert!(result.result.problems.is_empty());
                    edits += 1;
                }
            }
        }
    }
    assert_eq!(edits, 2);
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

#[test]
fn input_token_tampers_survive_the_cap_with_six_boxes() {
    let mut value = draft();
    let boxes: Vec<Value> = (0..6)
        .map(|i| {
            let mut b = value["boxes"][0].clone();
            b["boxId"] = json!(format!("{i:064x}"));
            b["tokens"] = json!([{"id": "aa".repeat(32), "amount": 5}]);
            b
        })
        .collect();
    value["tx"]["inputs"] = json!(boxes
        .iter()
        .map(|b| json!({"boxId": b["boxId"]}))
        .collect::<Vec<_>>());
    value["tx"]["outputs"] = json!(boxes);
    value["boxes"] = value["tx"]["outputs"].clone();
    value["options"] =
        json!({"seed": "six-boxes", "maxDepth": 1, "maxProbes": 128, "maxOpsPerStep": 1});
    let request = serde_json::from_value(value).unwrap();
    let report = ergo_sandbox::adversary::search(&request).unwrap();
    assert!(report
        .steps
        .iter()
        .flat_map(|s| &s.ops)
        .any(|op| matches!(op,
            ergo_sandbox::attack::AttackOp::TamperField { target, field, .. }
            if target == "input" && field == "tokenAmount:0"
        )));
}
