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
#[tokio::test]
async fn verify_http_outcomes_and_compile_shape() {
    let base = spawn().await;
    let client = reqwest::Client::new();
    let req = json!({"source":"sigmaProp(HEIGHT > $h)","params":{"h":{"type":"Int","value":100}}, "network":"testnet", "treeVersion":3});
    let compiled: Value = client
        .post(format!("{base}/api/v1/compile"))
        .json(&req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    for (target, source, outcome) in [
        (compiled["p2s"].clone(), "sigmaProp(HEIGHT > $h)", "exact"),
        (
            compiled["treeHex"].clone(),
            "sigmaProp(HEIGHT > 101)",
            "template",
        ),
        (
            compiled["treeHex"].clone(),
            "sigmaProp(HEIGHT < $h)",
            "no_match",
        ),
    ] {
        let mut req = req.clone();
        req["source"] = json!(source);
        req["target"] = target;
        let res = client
            .post(format!("{base}/api/v1/verify"))
            .json(&req)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let r: Value = res.json().await.unwrap();
        assert_eq!(r["outcome"], outcome);
        assert_eq!(r["limitation"], ergo_sandbox::identity::LIMITATION);
        assert_eq!(r["nodeValidated"], false);
    }
    for req in [
        json!({"source":"", "target":"00"}),
        json!({"source":"sigmaProp(true)","target":"bad"}),
        json!({"source":"sigmaProp($x)","target":"0008d3"}),
        json!({"source":"sigmaProp(true)","target":"0008d3","network":"moon"}),
    ] {
        let res = client
            .post(format!("{base}/api/v1/verify"))
            .json(&req)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 400);
        assert!(res.json::<Value>().await.unwrap()["error"]["message"].is_string());
    }
    let res = client
        .post(format!("{base}/api/v1/verify"))
        .header("content-type", "application/json")
        .body("x".repeat(ergo_web::app::MAX_BODY_BYTES + 1))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 413);
}
