//! S01 gates run through a real HTTP server, using S00 and P05 fixtures.
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

fn fixture(path: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}

async fn post(base: &str, request: Value) -> Value {
    let response = reqwest::Client::new()
        .post(format!("{base}/api/v1/checklist"))
        .json(&request)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(status, 200, "{body}");
    body
}

fn row<'a>(body: &'a Value, id: &str) -> &'a Value {
    body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap()
}

fn assert_rows(body: &Value) {
    let catalogue = fixture("docs/security/vectors.json");
    let rows = body["rows"].as_array().unwrap();
    assert_eq!(rows.len(), catalogue["vectors"].as_array().unwrap().len());
    for (r, v) in rows.iter().zip(catalogue["vectors"].as_array().unwrap()) {
        for field in ["id", "class", "title"] {
            assert_eq!(r[field], v[field]);
        }
        assert!(matches!(
            r["provenance"].as_str(),
            Some("static" | "scenario" | "preflight" | "node-validated" | "unchecked")
        ));
        assert!(!r["answer"].as_str().unwrap().is_empty());
        for finding in r["findings"].as_array().unwrap() {
            assert_eq!(finding["provenance"], "static");
            assert!(v["instrument"].as_array().unwrap().contains(&json!(format!(
                "lint:{}",
                finding["lint"].as_str().unwrap()
            ))));
            assert!(finding["anchor"]["nodeId"].is_u64());
            assert!(finding["anchor"]["irId"].is_u64() || finding["anchor"]["irId"].is_null());
            assert!(!finding["snippet"].as_str().unwrap().is_empty());
        }
        if r["provenance"] == "static" {
            assert!(!r["findings"].as_array().unwrap().is_empty());
        }
        if r["provenance"] == "unchecked" {
            assert!(r["findings"].as_array().unwrap().is_empty());
        }
    }
    assert_eq!(body["method"], "static-analysis");
    assert_eq!(body["nodeValidated"], false);
}

#[tokio::test]
async fn checklist_defaults_to_unchecked() {
    let base = spawn().await;
    // Guarded-get control from the existing HTTP/lint fixture: no lint hit.
    let body = post(
        &base,
        json!({"input":"1001040ad801d601c6a70404d1ede6720191e472017300"}),
    )
    .await;
    assert_rows(&body);
    assert!(body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["provenance"] == "unchecked"));
    assert!(body["artifacts"].as_array().unwrap().is_empty());
    // S00 control removes the zero-threshold observation, without asserting safety.
    let control = post(
        &base,
        json!({"source":include_str!("../../examples/contracts/vectors/zero-threshold/fixed.es")}),
    )
    .await;
    assert_eq!(row(&control, "zero-threshold")["provenance"], "unchecked");
}

#[tokio::test]
async fn checklist_answer_carries_provenance() {
    let base = spawn().await;
    let static_hit = post(&base, json!({"source":include_str!("../../examples/contracts/vectors/zero-threshold/vulnerable.es")})).await;
    assert_rows(&static_hit);
    assert_eq!(row(&static_hit, "zero-threshold")["provenance"], "static");
    assert_eq!(
        row(&static_hit, "rounded-payment")["provenance"],
        "unchecked"
    );

    let suite = fixture("examples/contracts/vectors/stale-oracle-epoch/contract.test.json");
    let scenario = post(&base, json!({"source":suite["source"],"scenario":{"vectorIds":["stale-oracle-epoch"],"scenario":suite["scenarios"][0]}})).await;
    assert_rows(&scenario);
    assert_eq!(
        row(&scenario, "stale-oracle-epoch")["provenance"],
        "scenario"
    );
    assert_eq!(scenario["artifacts"][0]["result"]["verdict"], "pass");
    assert_eq!(scenario["artifacts"][0]["nodeValidated"], false);
    for r in scenario["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["id"] != "stale-oracle-epoch")
    {
        assert_ne!(r["provenance"], "scenario");
    }

    let bundle = fixture("ergo-sandbox/tests/fixtures/evidence/claim-vectors/use-incident.fixture");
    let target = fixture("examples/incidents/use-lp-drain.deployed-swap.test.json")["tree"].clone();
    let evidence = post(&base, json!({"input":target,"evidence":{"vectorIds":["positional-reserve-binding"],"bundle":bundle}})).await;
    assert_rows(&evidence);
    assert_eq!(
        row(&evidence, "positional-reserve-binding")["provenance"],
        "node-validated"
    );
    assert_eq!(
        evidence["artifacts"][0]["result"]["status"],
        "confirmed-violation"
    );
    assert_eq!(evidence["artifacts"][0]["nodeValidated"], true);
    assert!(!row(&evidence, "positional-reserve-binding")["findings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn checklist_never_emits_a_score() {
    fn no_ranking(value: &Value) {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    let key = key.to_lowercase();
                    assert!(
                        ![
                            "score",
                            "percentage",
                            "percent",
                            "grade",
                            "passedcount",
                            "countofpassed",
                            "ranking",
                            "coverage"
                        ]
                        .iter()
                        .any(|word| key.contains(word)),
                        "{key}"
                    );
                    no_ranking(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    no_ranking(value);
                }
            }
            _ => (),
        }
    }
    let base = spawn().await;
    for input in [
        "1001040ad191e4c6a704047300",
        "1001040ad801d601c6a70404d1ede6720191e472017300",
    ] {
        let body = post(&base, json!({"input":input})).await;
        assert_rows(&body);
        no_ranking(&body);
    }
}

#[tokio::test]
async fn checklist_rejects_mismatched_or_forged_artifacts() {
    let base = spawn().await;
    for request in [
        json!({"input":"1001040ad191e4c6a704047300","scenario":{"vectorIds":["missing-self-register"],"scenario":{"height":100,"source":"sigmaProp(false)"}}}),
        json!({"source":"sigmaProp(false)","scenario":{"vectorIds":["deployment-constant-drift"],"scenario":{"height":100}}}),
        json!({"source":"sigmaProp(false)","scenario":{"vectorIds":["absent"],"scenario":{"height":100}}}),
        json!({"source":"sigmaProp(false)","evidence":{"vectorIds":["missing-self-register"],"bundle":{"nodeValidated":true,"status":"confirmed-violation"}}}),
    ] {
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/checklist"))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "{}", response.text().await.unwrap());
    }
    let mut bundle =
        fixture("ergo-sandbox/tests/fixtures/evidence/claim-vectors/use-incident.fixture");
    bundle["execution"]["headers"] = json!({"status":"missing","reason":"test omits headers"});
    let body = post(&base, json!({"input":fixture("examples/incidents/use-lp-drain.deployed-swap.test.json")["tree"],"evidence":{"vectorIds":["positional-reserve-binding"],"bundle":bundle}})).await;
    assert_eq!(body["artifacts"][0]["provenance"], "unchecked");
    assert!(body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["provenance"] != "node-validated"));
}

#[tokio::test]
async fn checklist_preflight_is_only_a_supplied_transaction_check() {
    let base = spawn().await;
    let input = "1001040ad801d601c6a70404d1ede6720191e472017300";
    let request = json!({"height":100,"boxes":[{"boxId":"ab".repeat(32),"ergoTree":input,"value":1000000,"creationHeight":1}],"tx":{"inputs":[{"boxId":"ab".repeat(32)}],"outputs":[{"ergoTree":input,"value":1000000,"creationHeight":100}]}});
    let body = post(&base, json!({"input":input,"preflight":{"vectorIds":["missing-self-register"],"request":request}})).await;
    assert_rows(&body);
    assert_eq!(
        row(&body, "missing-self-register")["provenance"],
        "preflight"
    );
    assert_eq!(body["artifacts"][0]["nodeValidated"], false);
    assert_eq!(body["artifacts"][0]["result"]["preflightPassed"], false);
}

#[tokio::test]
async fn checklist_uses_inspect_network_resolution_and_limits() {
    let base = spawn().await;
    let source = "sigmaProp(false)";
    let mainnet = post(&base, json!({"source":source})).await;
    let testnet = post(
        &base,
        json!({"input":mainnet["treeHex"],"network":"testnet"}),
    )
    .await;
    assert_ne!(mainnet["address"], testnet["address"]);
    let from_address = post(
        &base,
        json!({"input":testnet["address"],"network":"testnet"}),
    )
    .await;
    assert_eq!(from_address["treeHex"], mainnet["treeHex"]);
    for (request, expected) in [
        (json!({"source":source,"network":"Testnet"}), 400),
        (json!({"input":"garbage"}), 400),
        (json!({"input":mainnet["treeHex"],"source":source}), 400),
        (json!({"source":"a".repeat(64*1024+1)}), 413),
        (
            json!({"input":"a".repeat(ergo_web::app::MAX_BODY_BYTES+1)}),
            413,
        ),
    ] {
        let response = reqwest::Client::new()
            .post(format!("{base}/api/v1/checklist"))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert!(response.json::<Value>().await.unwrap()["error"].is_object());
    }
}
