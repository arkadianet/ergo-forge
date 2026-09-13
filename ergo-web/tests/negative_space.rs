//! Negative-space lines are projections of actual lint findings, through HTTP.
use serde_json::{json, Value};

async fn inspect(source: &str) -> Value {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, ergo_web::app::router_with(Default::default()))
            .await
            .unwrap();
    });
    let client = reqwest::Client::new();
    let compiled = client
        .post(format!("http://{addr}/api/v1/compile"))
        .json(&json!({"source":source}))
        .send()
        .await
        .unwrap();
    assert_eq!(compiled.status(), 200);
    let compiled: Value = compiled.json().await.unwrap();
    let response = client
        .post(format!("http://{addr}/api/v1/inspect"))
        .json(&json!({"input":compiled["treeHex"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}

fn sources() -> [&'static str; 4] {
    [
        "sigmaProp(INPUTS(0).value > SELF.value)",
        "sigmaProp(OUTPUTS(0).value >= SELF.value)",
        "sigmaProp(CONTEXT.dataInputs(0).R4[Long].get > 100L)",
        include_str!("../../examples/contracts/vectors/successor-register-drift/vulnerable.es"),
    ]
}

#[tokio::test]
async fn negative_space_lines_have_anchors() {
    for source in sources() {
        let response = inspect(source).await;
        let bytes = hex::decode(response["treeHex"].as_str().unwrap()).unwrap();
        let report = ergo_sandbox::decompile::with_large_stack(move || {
            let tree = ergo_sandbox::inspect::parse_tree(&bytes).unwrap();
            ergo_sandbox::audit::audit(&ergo_sandbox::lift_tree(&tree, false))
        });
        let lines = response["negativeSpace"].as_array().unwrap();
        assert!(!lines.is_empty(), "{source}");
        for line in lines {
            let node = line["anchor"]["nodeId"].as_u64().expect("node anchor");
            let original = report
                .findings
                .iter()
                .find(|f| f.node_id == node && f.lint == line["lint"])
                .expect("an actual lint finding");
            assert_eq!(line["text"], original.message);
            assert_eq!(line["snippet"], original.snippet);
            assert_eq!(line["anchor"]["irId"], json!(original.ir_id));
            assert!(!original.snippet.is_empty());
        }
    }
}

#[tokio::test]
async fn negative_space_is_labelled_static() {
    for source in sources() {
        let response = inspect(source).await;
        assert_eq!(response["method"], "static-analysis");
        assert_eq!(response["nodeValidated"], false);
        let lines = response["negativeSpace"].as_array().unwrap();
        assert!(!lines.is_empty());
        for line in lines {
            assert_eq!(line["provenance"], "static");
        }
    }
    let clean = inspect("sigmaProp(false)").await;
    assert!(clean["negativeSpace"].as_array().unwrap().is_empty());
}
