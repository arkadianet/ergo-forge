use serde_json::{json, Value};
#[tokio::test]
async fn lock_route_uses_compile_inputs_and_exact_source_text() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, ergo_web::app::router_with(Default::default()))
            .await
            .unwrap();
    });
    let client = reqwest::Client::new();
    let req = json!({"source":"// λ\r\nsigmaProp(HEIGHT > $h)\n", "params":{"h":{"type":"Int","value":100}}, "network":"testnet", "treeVersion":3});
    let compiled: Value = client
        .post(format!("http://{addr}/api/v1/compile"))
        .json(&req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let response = client
        .post(format!("http://{addr}/api/v1/lock"))
        .json(&req)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let lock: ergo_sandbox::lockfile::Lockfile = response.json().await.unwrap();
    assert_eq!(lock.address, compiled["p2s"]);
    assert_eq!(lock.tree_hex, compiled["treeHex"]);
    let report = ergo_sandbox::lockfile::verify_lock(
        &lock,
        req["source"].as_str().unwrap(),
        &serde_json::from_value(req["params"].clone()).unwrap(),
        3,
        ergo_ser::address::NetworkPrefix::Testnet,
    )
    .unwrap();
    assert!(report.drifts.is_empty());
    assert_eq!(lock.limitation, ergo_sandbox::identity::LIMITATION);
    for req in [
        json!({"source":""}),
        json!({"source":"sigmaProp($h)"}),
        json!({"source":"sigmaProp(true)","network":"moon"}),
    ] {
        assert_eq!(
            client
                .post(format!("http://{addr}/api/v1/lock"))
                .json(&req)
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    assert_eq!(
        client
            .post(format!("http://{addr}/api/v1/lock"))
            .header("content-type", "application/json")
            .body("x".repeat(ergo_web::app::MAX_BODY_BYTES + 1))
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
}
