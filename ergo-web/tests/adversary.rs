#[path = "../../ergo-sandbox/tests/engine_support/request.rs"]
mod engine_support;
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

fn request() -> Value {
    let id = format!("{:0>64}", "b0");
    json!({
        "height": 1000,
        "network": "mainnet",
        "boxes": [{"boxId": id, "value": 1000000, "ergoTree": "10010101d17300", "tokens": [], "registers": {}}],
        "tx": {"inputs": [{"boxId": id}], "dataInputs": [], "outputs": [{"value": 1000000, "ergoTree": "10010101d17300"}]},
        "options": {"seed": "http-test", "maxDepth": 1, "maxProbes": 2, "maxOpsPerStep": 1}
    })
}

#[tokio::test]
async fn adversary_route_is_budgeted_and_synthetic() {
    let base = spawn().await;
    let response = reqwest::Client::new()
        .post(format!("{base}/api/v2/adversary"))
        .json(&request())
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["nodeValidated"], false);
    assert_eq!(body["method"], "adversary-search");
    assert_eq!(body["caps"]["maxDepth"], 1);
    assert!(body["fingerprint"].as_str().is_some());
}
