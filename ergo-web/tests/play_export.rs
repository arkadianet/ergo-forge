//! W02 gates: real HTTP Play/export, followed by the unchanged CLI runners.
use ergo_sandbox::{eval_scenario, testsuite, Scenario};
use serde_json::{json, Value};

async fn spawn() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, ergo_web::app::router_with(Default::default()))
            .await
            .unwrap();
    });
    format!("http://{addr}")
}

async fn post(base: &str, path: &str, request: &Value) -> Value {
    let res = reqwest::Client::new()
        .post(format!("{base}/api/v1/{path}"))
        .json(request)
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body = res.json::<Value>().await.unwrap();
    assert_eq!(status, 200, "{body}");
    body
}

fn tree(source: &str) -> String {
    hex::encode(
        ergo_sandbox::compile_source(source, 3, ergo_ser::address::NetworkPrefix::Mainnet)
            .unwrap()
            .tree_bytes,
    )
}

fn request(source: &str) -> Value {
    json!({"height":200,"network":"testnet","boxes":[
        {"boxId":"aa".repeat(32),"ergoTree":tree(source),"value":7,"creationHeight":50},
        {"boxId":"bb".repeat(32),"ergoTree":tree("sigmaProp(false)"),"value":3},
        {"boxId":"cc".repeat(32),"ergoTree":tree("sigmaProp(true)"),"value":1,
         "registers":{"R4":{"type":"Long","value":500}}}
    ],"tx":{
        "inputs":[{"boxId":"bb".repeat(32)}, {"boxId":"AA".repeat(32),"contextVars":{"0":{"type":"Int","value":5}}}],
        "dataInputs":["CC".repeat(32)],
        "outputs":[{"value":10,"ergoTree":tree("sigmaProp(true)")}]
    }})
}

async fn exported(base: &str, req: &Value, kind: &str) -> Value {
    let mut payload = req.clone();
    payload["inputIndex"] = json!(1);
    payload["kind"] = json!(kind);
    post(base, "play/export", &payload).await
}

fn synthetic(doc: &Value) {
    assert_eq!(doc["nodeValidated"], false);
    assert_eq!(doc["synthetic"], true);
    assert_eq!(doc["method"], "scenario-simulation");
}

#[tokio::test]
async fn play_export_roundtrips_through_cli_test_runner() {
    let base = spawn().await;
    for (source, verdict) in [
        ("sigmaProp(SELF.id == INPUTS(1).id && INPUTS(0).value == 3L && SELF.value == 7L && HEIGHT == 200 && getVar[Int](0).get == 5 && CONTEXT.dataInputs(0).R4[Long].get == 500L && OUTPUTS(0).creationInfo._1 == HEIGHT && OUTPUTS(0).value == 10L)", "pass"),
        ("sigmaProp(HEIGHT < 100)", "fail"),
        ("sigmaProp(SELF.R4[Int].get == 1)", "error"),
        ("proveDlog(decodePoint(fromBase16(\"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798\")))", "needsProof"),
    ] {
        let req = request(source);
        let played = post(&base, "play", &req).await;
        assert_eq!(played["inputs"][1]["verdict"], verdict);
        // The other input refuses: export expects this input's verdict, not tx.ok.
        assert_eq!(played["ok"], false);
        let suite_doc = exported(&base, &req, "test").await;
        synthetic(&suite_doc);
        let suite: testsuite::Suite = serde_json::from_value(suite_doc).unwrap();
        // This is exactly the runner called by `ergo-es test`.
        let result = testsuite::run(&suite).unwrap();
        assert_eq!(result.failed, 0, "{:?}", result.cases);
        assert_eq!(result.cases[0].actual, verdict, "synthetic-drift");
        assert_eq!(result.cases[0].cost, played["inputs"][1]["cost"].as_u64().unwrap());
        let scenario_doc = exported(&base, &req, "scenario").await;
        synthetic(&scenario_doc);
        assert_eq!(scenario_doc["outputs"], played["outputs"]);
        assert_eq!(scenario_doc["selfIndex"], 1);
        assert_eq!(scenario_doc["network"], "testnet");
        assert_eq!(scenario_doc["contextVars"], req["tx"]["inputs"][1]["contextVars"]);
        assert_eq!(scenario_doc["inputs"][0]["boxId"], req["boxes"][1]["boxId"]);
        assert_eq!(scenario_doc["inputs"][1]["boxId"], req["boxes"][0]["boxId"]);
        assert_eq!(scenario_doc["dataInputs"][0]["boxId"], req["boxes"][2]["boxId"]);
        let scenario: Scenario = serde_json::from_value(scenario_doc).unwrap();
        let outcome = eval_scenario(&scenario).unwrap();
        assert_eq!(testsuite::verdict_name(outcome.verdict), verdict, "synthetic-drift");
        assert_eq!(outcome.cost, result.cases[0].cost);
    }
}

#[tokio::test]
async fn exported_case_names_only_the_suite_contract() {
    let base = spawn().await;
    let req = request("sigmaProp(HEIGHT > 100)");
    let doc = exported(&base, &req, "test").await;
    assert_eq!(doc["tree"], req["boxes"][0]["ergoTree"]);
    assert_eq!(doc["network"], "testnet");
    assert_eq!(doc["scenarios"].as_array().unwrap().len(), 1);
    for obj in [&doc, &doc["scenarios"][0]] {
        for key in ["source", "params", "secrets", "parties"] {
            assert!(obj.get(key).is_none(), "{key}: {obj}");
        }
    }
    assert!(doc["scenarios"][0].get("tree").is_none());
    assert!(doc["scenarios"][0].get("selfBox").is_none());
    let mut case = doc["scenarios"][0].clone();
    case["tree"] = doc["tree"].clone();
    case.as_object_mut().unwrap().remove("expect");
    case.as_object_mut().unwrap().remove("name");
    let mut scenario = exported(&base, &req, "scenario").await;
    for key in ["method", "provenance", "nodeValidated", "synthetic"] {
        scenario.as_object_mut().unwrap().remove(key);
    }
    assert_eq!(case, scenario);
}

#[tokio::test]
async fn signing_material_is_omitted_and_unsigned_expectation_is_named() {
    let base = spawn().await;
    for signing in ["secrets", "parties"] {
        let mut req = request("proveDlog(decodePoint(fromBase16(\"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798\")))");
        let secret = json!({"dlog":format!("{:064x}",1)});
        req["tx"]["inputs"][1][signing] = if signing == "secrets" {
            json!([secret])
        } else {
            json!([{"name":"signer","secrets":[secret]}])
        };
        let played = post(&base, "play", &req).await;
        assert_eq!(played["inputs"][1]["verdict"], "proofAccepted");
        for kind in ["test", "scenario"] {
            let doc = exported(&base, &req, kind).await;
            synthetic(&doc);
            let serialized = doc.to_string();
            assert!(!serialized.contains("\"secrets\""));
            assert!(!serialized.contains("\"parties\""));
            assert!(!serialized.contains("\"dlog\""));
            if kind == "test" {
                assert_eq!(doc["scenarios"][0]["expect"], "needsProof");
                assert!(doc["scenarios"][0]["name"]
                    .as_str()
                    .unwrap()
                    .contains("signature-less"));
                assert_eq!(
                    testsuite::run(&serde_json::from_value(doc).unwrap())
                        .unwrap()
                        .failed,
                    0
                );
            } else {
                assert_eq!(
                    testsuite::verdict_name(
                        eval_scenario(&serde_json::from_value(doc).unwrap())
                            .unwrap()
                            .verdict
                    ),
                    "needsProof"
                );
            }
        }
    }
}

#[tokio::test]
async fn export_rejects_bad_selection_kind_missing_boxes_and_oversized_bodies() {
    let base = spawn().await;
    let mut req = request("sigmaProp(true)");
    req["inputIndex"] = json!(1);
    req["kind"] = json!("test");
    for (key, value) in [
        ("inputIndex", json!(2)),
        ("inputIndex", json!(-1)),
        ("kind", json!("unknown")),
        ("boxes", json!([])),
    ] {
        let mut bad = req.clone();
        bad[key] = value;
        let res = reqwest::Client::new()
            .post(format!("{base}/api/v1/play/export"))
            .json(&bad)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 400);
        assert!(res.json::<Value>().await.unwrap()["error"].is_object());
    }
    let res = reqwest::Client::new()
        .post(format!("{base}/api/v1/play/export"))
        .header("content-type", "application/json")
        .body(" ".repeat(ergo_web::app::MAX_BODY_BYTES + 1))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 413);
    assert!(res.json::<Value>().await.unwrap()["error"].is_object());
}
