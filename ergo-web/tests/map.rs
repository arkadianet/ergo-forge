//! The reader's map API: offline reproducibility, typed edges and honest limits.
use ergo_web::app::{router_with, AppConfig};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../ui/examples/reader-map.json")).unwrap()
}
fn request() -> Value {
    json!({"input": "11".repeat(32), "kind": "boxId", "fixture": fixture()})
}
async fn spawn(explorer: Option<String>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            router_with(AppConfig {
                explorer_url: explorer,
                explorer_network: "mainnet".into(),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}")
}
async fn post(base: &str, req: &Value) -> (u16, Value) {
    let res = reqwest::Client::new()
        .post(format!("{base}/api/v1/map"))
        .json(req)
        .send()
        .await
        .unwrap();
    (res.status().as_u16(), res.json().await.unwrap())
}

#[tokio::test]
async fn offline_map_reaches_related_source_and_preserves_typed_edges() {
    // A recording must work even when the configured explorer is unreachable.
    let base = spawn(Some("http://127.0.0.1:1".into())).await;
    let (status, result) = post(&base, &request()).await;
    assert_eq!(status, 200, "{result}");
    assert_eq!(result["seed"]["kind"], "boxId");
    assert_eq!(result["source"]["recorded"], true);
    assert_eq!(result["source"]["height"], 1200);
    assert_eq!(result["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(result["nodes"][1]["depth"], 1);
    assert_eq!(result["nodes"][1]["value"], "1000000");
    let edges = result["edges"].as_array().unwrap();
    assert!(edges
        .iter()
        .any(|e| e["binding"] == "nft" && e["singleton"] == true && e["to"] == "22".repeat(32)));
    assert!(edges.iter().any(|e| e["binding"] == "data-input"));
    assert!(result["truncated"].is_null());
    // Every node can actually be opened in the reader, including offline.
    for n in result["nodes"].as_array().unwrap() {
        let res = reqwest::Client::new()
            .post(format!("{base}/api/v1/inspect"))
            .json(&json!({"input": n["treeHex"]}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
    }
    assert_eq!(
        post(&base, &request()).await.1,
        result,
        "stable DTO for a recorded chain state"
    );
}

#[tokio::test]
async fn all_seed_kinds_and_exact_box_frontier() {
    let base = spawn(None).await;
    let mut f = fixture();
    let first = f["boxes"]["11".repeat(32)].clone();
    let second = f["boxes"]["22".repeat(32)].clone();
    let address = ergo_ser::address::encode_p2s(
        ergo_ser::address::NetworkPrefix::Mainnet,
        &hex::decode(first["ergoTree"].as_str().unwrap()).unwrap(),
    );
    f["boxesByAddress"] =
        json!({address.clone(): {"items": [first.clone(), second.clone()], "total": 2}});
    f["transactions"] = json!({"bb".repeat(32): {"inputs": [first], "outputs": [second]}});
    for (kind, input, count) in [
        ("boxId", "11".repeat(32), 1),
        ("address", address, 2),
        ("tokenId", "aa".repeat(32), 1),
        ("transactionId", "bb".repeat(32), 2),
    ] {
        let (status, result) = post(
            &base,
            &json!({"kind":kind,"input":input,"fixture":f,"maxDepth":0}),
        )
        .await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["nodes"].as_array().unwrap().len(), count, "{kind}");
        assert_eq!(result["truncated"]["depth"], count);
    }
}

#[tokio::test]
async fn caps_and_unresolved_references_are_not_silently_omitted() {
    let base = spawn(None).await;
    let mut req = request();
    req["maxNodes"] = json!(1);
    let (status, result) = post(&base, &req).await;
    assert_eq!(status, 200, "{result}");
    assert_eq!(result["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(result["truncated"]["nodes"], 1);
    assert!(result["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["unresolved"].is_string()));
}

#[tokio::test]
async fn invalid_inputs_and_recording_gaps_are_json_errors() {
    let base = spawn(None).await;
    for change in [
        json!({"kind":"guess"}),
        json!({"input":"not-an-id"}),
        json!({"maxDepth":5}),
        json!({"maxNodes":0}),
        json!({"maxNodes":65}),
        json!({"network":"typo"}),
        json!({"kind":"address","input":"deadbeef"}),
        json!({"fixture":{"formatVersion":99,"height":1,"kind":"fixture"}}),
    ] {
        let mut req = request();
        req.as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        let (status, body) = post(&base, &req).await;
        assert_eq!(status, 400, "{req}: {body}");
        assert_eq!(body["error"]["code"], "invalid_input");
    }
    let mut req = request();
    req["fixture"]["tokens"] = json!({});
    let (status, body) = post(&base, &req).await;
    assert_eq!(status, 400);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("no recorded answer"));
}

#[tokio::test]
async fn live_map_requires_configuration_and_matching_network() {
    let mut req = request();
    req.as_object_mut().unwrap().remove("fixture");
    let (status, body) = post(&spawn(None).await, &req).await;
    assert_eq!(status, 501);
    assert_eq!(body["error"]["code"], "not_configured");
    req["network"] = json!("testnet");
    let (status, body) = post(&spawn(Some("http://127.0.0.1:1".into())).await, &req).await;
    assert_eq!(status, 400);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("mainnet"));
}

#[tokio::test]
async fn live_map_uses_configured_explorer_and_reports_missing_seed() {
    use axum::{routing::get, Json, Router};
    let b = fixture()["boxes"]["22".repeat(32)].clone();
    let app = Router::new()
        .route("/api/v1/networkState", get(|| async { Json(json!({"height":1200})) }))
        .route("/api/v1/boxes/{id}", get(move |axum::extract::Path(id): axum::extract::Path<String>| {
            let b = b.clone(); async move {
                if id == "22".repeat(32) { Ok(Json(json!({"boxId":id,"ergoTree":b["ergoTree"],"value":1000000,"assets":[]}))) }
                else { Err(axum::http::StatusCode::NOT_FOUND) }
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let base = spawn(Some(format!("http://{address}"))).await;
    let (status, result) = post(&base, &json!({"input":"22".repeat(32),"kind":"boxId"})).await;
    assert_eq!(status, 200, "{result}");
    assert_eq!(result["source"]["recorded"], false);
    assert_eq!(result["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(
        post(&base, &json!({"input":"33".repeat(32),"kind":"boxId"}))
            .await
            .0,
        404
    );
}
