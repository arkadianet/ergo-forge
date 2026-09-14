//! In-memory HTTP against the real router: no sockets, external service, or new dependency.
use axum::http::StatusCode;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

struct MemoryListener(Option<DuplexStream>);
impl axum::serve::Listener for MemoryListener {
    type Io = DuplexStream;
    type Addr = ();
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.0.take() {
            Some(stream) => (stream, ()),
            None => std::future::pending().await,
        }
    }
    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(())
    }
}

fn request() -> Value {
    // Supplied lockfile bytes are a reference; Watch does not compile or vouch
    // for source provenance. Full compilation lock behavior is tested in I02.
    json!({"watches": [{"lockfile": {
        "schemaVersion": 1, "sourceSha256": "00".repeat(32), "params": {},
        "compilerRevision": "fixture", "nodeRevision": "fixture", "workbenchVersion": "fixture",
        "treeHex": "0008d3", "address": "fixture", "treeVersion": 0,
        "lintSweepDigest": "00".repeat(32), "limitation": ergo_sandbox::identity::LIMITATION
    }, "nfts": ["11".repeat(32)], "registers": ["R4"]}]})
}
async fn post_with(app: axum::Router, body: String) -> (StatusCode, Value) {
    let (mut client, server) = tokio::io::duplex(2 * ergo_web::app::MAX_BODY_BYTES);
    let serving = tokio::spawn(async move {
        axum::serve(MemoryListener(Some(server)), app)
            .await
            .unwrap();
    });
    let request = format!("POST /api/v1/watch HTTP/1.1\r\nHost: memory\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
    client.write_all(request.as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await.unwrap();
    serving.abort();
    let response = String::from_utf8(bytes).unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    let status =
        StatusCode::from_u16(headers.split_whitespace().nth(1).unwrap().parse().unwrap()).unwrap();
    (status, serde_json::from_str(body).unwrap())
}
async fn post(body: String) -> (StatusCode, Value) {
    post_with(ergo_web::app::router_with(Default::default()), body).await
}

#[tokio::test]
async fn watch_route_returns_unverified_without_explorer_and_keeps_no_baseline() {
    let app = ergo_web::app::router_with(Default::default());
    let req = request();
    let (status, report) = post_with(app.clone(), req.to_string()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report[0]["live"]["status"], "unverified");
    assert_eq!(
        report[0]["live"]["reason"],
        ergo_sandbox::watch::NO_EXPLORER
    );
    assert_eq!(report[0]["registers"][0]["status"], "unverified");
    assert_eq!(report[0]["chainSource"], json!({"kind":"none","url":null}));
    assert!(report[0]["height"].is_null());
    assert_eq!(report[0]["observation"], ergo_sandbox::watch::OBSERVATION);
    assert_eq!(report[0]["limitation"], ergo_sandbox::identity::LIMITATION);
    let mut baseline_req = req.clone();
    baseline_req["watches"][0]["baselineBoxes"] = json!({"11".repeat(32): {
        "boxId": "22".repeat(32), "ergoTree": "0008d3", "value": 1000000,
        "additionalRegisters": {"R4": {"serializedValue":"040e"}}
    }});
    let (status, baseline) = post_with(app.clone(), baseline_req.to_string()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(baseline[0]["registers"][0]["expected"], "040e");
    assert_eq!(baseline[0]["baselineOrigin"], "supplied");
    assert_eq!(post_with(app, req.to_string()).await.1, report);
}

#[tokio::test]
async fn watch_route_rejects_urls_unknown_fields_bad_inputs_and_oversized_body() {
    for field in ["explorer", "explorerUrl", "fixture"] {
        let mut req = request();
        req[field] = json!("https://unrequested.invalid");
        assert_eq!(post(req.to_string()).await.0, StatusCode::BAD_REQUEST);
        let mut req = request();
        req["watches"][0][field] = json!("https://unrequested.invalid");
        assert_eq!(post(req.to_string()).await.0, StatusCode::BAD_REQUEST);
    }
    for (key, value) in [
        ("nfts", json!([])),
        ("nfts", json!(["bad"])),
        ("registers", json!(["R10"])),
    ] {
        let mut req = request();
        req["watches"][0][key] = value;
        assert_eq!(post(req.to_string()).await.0, StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        post(json!({"watches":[]}).to_string()).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(post("{".into()).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        post("x".repeat(ergo_web::app::MAX_BODY_BYTES + 1)).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}
