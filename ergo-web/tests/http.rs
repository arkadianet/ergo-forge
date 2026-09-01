//! Integration tests over a real server on an ephemeral port.

use std::net::SocketAddr;

/// Start the app on a random free port; returns its base URL.
async fn spawn() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, ergo_web::app::router())
            .await
            .unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn health_returns_ok() {
    let base = spawn().await;
    let body = reqwest::get(format!("{base}/api/v1/health"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("\"status\":\"ok\""), "body: {body}");
}

#[tokio::test]
async fn unguarded_get_returns_one_high_finding() {
    let base = spawn().await;
    let res: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({ "input": "1001040ad191e4c6a704047300" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(res["findings"].as_array().unwrap().len(), 1, "{res}");
    assert_eq!(res["findings"][0]["severity"], "high");
    assert_eq!(res["findings"][0]["lint"], "unchecked-get");
    assert_eq!(res["completeness"], "complete");
    assert!(res["source"].as_str().unwrap().contains("get"));
}

#[tokio::test]
async fn guarded_get_returns_no_findings() {
    let base = spawn().await;
    let res: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({
            "input": "1001040ad801d601c6a70404d1ede6720191e472017300"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(res["findings"].as_array().unwrap().len(), 0, "{res}");
}

#[tokio::test]
async fn garbage_input_is_a_400() {
    let base = spawn().await;
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({ "input": "not a contract" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn oversized_body_is_a_413() {
    let base = spawn().await;
    let big = "a".repeat(64 * 1024 + 1);
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .header("content-type", "application/json")
        .body(big)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 413, "status: {}", r.status());
}

/// The stack-budget proof: a ~60-level contract needs ~3 MiB of recursion;
/// tokio threads default to 2 MiB. The handler must run the decompile on a
/// `spawn_blocking` thread wrapped in `with_large_stack`, or this test
/// aborts the whole test process. (Removing `with_large_stack` while keeping
/// `spawn_blocking` was verified to abort — see the commit message.)
#[tokio::test]
async fn a_deeply_nested_contract_does_not_kill_the_server() {
    let mut src = String::from("1");
    for _ in 0..60 {
        src = format!("({src} + 1)");
    }
    let src = format!("sigmaProp({src} > 0)");
    // The COMPILER also recurses ~3 MiB deep at this nesting — more than the
    // 2 MiB test thread. Build the fixture on a large stack; the thing under
    // test is the server's handling of the resulting bytes.
    let bytes = ergo_sandbox::decompile::with_large_stack(move || {
        ergo_sandbox::compile_source(&src, 3, ergo_ser::address::NetworkPrefix::Mainnet)
            .expect("compile")
            .tree_bytes
    });

    let base = spawn().await;
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({ "input": hex::encode(&bytes) }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "status: {}", r.status());

    // The server must still be alive afterwards.
    let h = reqwest::get(format!("{base}/api/v1/health")).await.unwrap();
    assert_eq!(h.status(), 200, "server died on a deep contract");
}
