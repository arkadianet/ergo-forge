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
async fn malformed_json_body_is_a_json_400() {
    let base = spawn().await;
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .header("content-type", "application/json")
        .body("{not json")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let body: serde_json::Value = r.json().await.expect("400 body must be JSON");
    assert_eq!(body["error"]["code"], "invalid_input");
}

#[tokio::test]
async fn unknown_network_is_a_400() {
    let base = spawn().await;
    for bad in ["testnet ", "Mainnet", "regtest", ""] {
        let r = reqwest::Client::new()
            .post(format!("{base}/api/v1/inspect"))
            .json(&serde_json::json!({ "input": "1001040ad191e4c6a704047300", "network": bad }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 400, "network {bad:?} was accepted");
        let body: serde_json::Value = r.json().await.unwrap();
        assert_eq!(body["error"]["code"], "invalid_input");
    }
}

#[tokio::test]
async fn testnet_network_encodes_a_testnet_address() {
    let base = spawn().await;
    let res: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({ "input": "1001040ad191e4c6a704047300", "network": "testnet" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mainnet: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({ "input": "1001040ad191e4c6a704047300" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_ne!(res["address"], mainnet["address"]);
}

#[tokio::test]
async fn oversized_body_413_is_json() {
    let base = spawn().await;
    let big = "a".repeat(64 * 1024 + 1);
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .header("content-type", "application/json")
        .body(big)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 413);
    let body: serde_json::Value = r.json().await.expect("413 body must be JSON");
    assert_eq!(body["error"]["code"], "too_large");
}

/// The stack-budget proof, measured on a REAL contract. This tree is the
/// deepest in the mainnet corpus (46 nested levels), and real contracts cost
/// far more stack per level than synthetic arithmetic: Infix-only shapes take
/// a cheap lift path, while real ones drive `lift_op_inner`, whose debug
/// frame is enormously wider. Measured requirement for THIS tree: overflows
/// a 2 MiB thread (tokio's default) and fits in 3 MiB — so the handler's
/// `spawn_blocking` + `with_large_stack` (16 MiB) is carrying real load, not
/// being cautious. With `with_large_stack` removed (keeping
/// `spawn_blocking`), this test aborts the process — verified.
#[tokio::test]
async fn a_deeply_nested_contract_does_not_kill_the_server() {
    // Real mainnet tree, 46 levels deep — the corpus's worst case.
    const DEEP_TREE_HEX: &str = "1071040c0400040004000e20e5abaf1f0a9442123104cdf4d2d56ddd8065803e842bc6d433e712601133a9bc042a040c010004000406040004000402040004040400040204020402040404040404040604060406040804080408040a040a040a040c040c040c040e040e040e041004100410041204120412041404140414041604160416041804180418041a041a041a041c041c041c041e041e041e0420042004200422042204220424042404240426042604260428042804280400042a04000402040604000402040404060408040a040c040e04100412041404160418041a041c041e0420042204240426042804000e20f7f008ad8fcaad4490d8e78ab6d3f11efe7213a13f7b243795818b155e1acc92040004000402040204020402040404020408d84cd60199a37300d602b2a4730100d603b5a4d9010363d801d605db6308720395ed91b172057302938cb27205730300017304d801d606c672030611ededede6720692b1e472067305928cc7720301997201730693e4c672030504e4c6720204047307d604b17203d605e4e3011ad606b4720373087309d607e4c6b27206730a000611d608b27207730b00d609e4c6b27206730c000611d60ab27209730d00d60be4c6b27206730e000611d60cb2720b730f00d60db27207731000d60eb27209731100d60fb2720b731200d610b27207731300d611b27209731400d612b2720b731500d613b27207731600d614b27209731700d615b2720b731800d616b27207731900d617b27209731a00d618b2720b731b00d619b27207731c00d61ab27209731d00d61bb2720b731e00d61cb27207731f00d61db27209732000d61eb2720b732100d61fb27207732200d620b27209732300d621b2720b732400d622b27207732500d623b27209732600d624b2720b732700d625b27207732800d626b27209732900d627b2720b732a00d628b27207732b00d629b27209732c00d62ab2720b732d00d62bb27207732e00d62cb27209732f00d62db2720b733000d62eb27207733100d62fb27209733200d630b2720b733300d631b27207733400d632b27209733500d633b2720b733600d634b27207733700d635b27209733800d636b2720b733900d637b27207733a00d638b27209733b00d639b2720b733c00d63ab27207733d00d63bb27209733e00d63cb2720b733f00d63db27207734000d63eb27209734100d63fb2720b734200d640b27207734300d641b27209734400d642b2720b734500d643b27207734600d644b27209734700d645b2720b734800d646b27207734900d647b27209734a00d648b2720b734b00d649e4c6a7041ad64adc640de4c67202056402dc0c1db47249734c734d017205e4e3020ed64bb2a5734e00d64cb2a5734f00ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02cde4c6b27203e4e30004000407d18f8cc77202017201d19272047350d1edededededededededededededededededededed937cb2720573510099999a9a7208720a720c958f7208720a958f7208720c7208720c958f720a720c720a720c95917208720a95917208720c7208720c9591720a720c720a720c937cb2720573520099999a9a720d720e720f958f720d720e958f720d720f720d720f958f720e720f720e720f9591720d720e9591720d720f720d720f9591720e720f720e720f937cb2720573530099999a9a721072117212958f72107211958f7210721272107212958f72117212721172129591721072119591721072127210721295917211721272117212937cb2720573540099999a9a721372147215958f72137214958f7213721572137215958f72147215721472159591721372149591721372157213721595917214721572147215937cb2720573550099999a9a721672177218958f72167217958f7216721872167218958f72177218721772189591721672179591721672187216721895917217721872177218937cb2720573560099999a9a7219721a721b958f7219721a958f7219721b7219721b958f721a721b721a721b95917219721a95917219721b7219721b9591721a721b721a721b937cb2720573570099999a9a721c721d721e958f721c721d958f721c721e721c721e958f721d721e721d721e9591721c721d9591721c721e721c721e9591721d721e721d721e937cb2720573580099999a9a721f72207221958f721f7220958f721f7221721f7221958f72207221722072219591721f72209591721f7221721f722195917220722172207221937cb2720573590099999a9a722272237224958f72227223958f7222722472227224958f72237224722372249591722272239591722272247222722495917223722472237224937cb27205735a0099999a9a722572267227958f72257226958f7225722772257227958f72267227722672279591722572269591722572277225722795917226722772267227937cb27205735b0099999a9a72287229722a958f72287229958f7228722a7228722a958f7229722a7229722a95917228722995917228722a7228722a95917229722a7229722a937cb27205735c0099999a9a722b722c722d958f722b722c958f722b722d722b722d958f722c722d722c722d9591722b722c9591722b722d722b722d9591722c722d722c722d937cb27205735d0099999a9a722e722f7230958f722e722f958f722e7230722e7230958f722f7230722f72309591722e722f9591722e7230722e72309591722f7230722f7230937cb27205735e0099999a9a723172327233958f72317232958f7231723372317233958f72327233723272339591723172329591723172337231723395917232723372327233937cb27205735f0099999a9a723472357236958f72347235958f7234723672347236958f72357236723572369591723472359591723472367234723695917235723672357236937cb2720573600099999a9a723772387239958f72377238958f7237723972377239958f72387239723872399591723772389591723772397237723995917238723972387239937cb2720573610099999a9a723a723b723c958f723a723b958f723a723c723a723c958f723b723c723b723c9591723a723b9591723a723c723a723c9591723b723c723b723c937cb2720573620099999a9a723d723e723f958f723d723e958f723d723f723d723f958f723e723f723e723f9591723d723e9591723d723f723d723f9591723e723f723e723f937cb2720573630099999a9a724072417242958f72407241958f7240724272407242958f72417242724172429591724072419591724072427240724295917241724272417242937cb2720573640099999a9a724372447245958f72437244958f7243724572437245958f72447245724472459591724372449591724372457243724595917244724572447245937cb2720573650099999a9a724672477248958f72467247958f7246724872467248958f72477248724772489591724672479591724672487246724895917247724872477248d1e6724ad1938cb2db63087202736600017367d193b2db6308724b736800b2db63087202736900d1938cb2db6308724b736a00018cb2db63087202736b0001d1928cb2db6308724b736c0002998cb2db63087202736d00027e9c7204736e05d193b1db6308724bb1db63087202d193e4c6724b04049ae4c672020404736fd193db6401e4c6724b0564db6401e4724ad193e4c6724b0604e4c672020604d193c2724bc27202d192c1724bc17202d1928cc7724b0199a37370d193db6308724cdb6308a7d193c2724cc2a7d192c1724cc1a7d193e4c6724c041a7249";

    let base = spawn().await;
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/inspect"))
        .json(&serde_json::json!({ "input": DEEP_TREE_HEX }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200, "status: {}", r.status());

    // The server must still be alive afterwards.
    let h = reqwest::get(format!("{base}/api/v1/health")).await.unwrap();
    assert_eq!(h.status(), 200, "server died on a deep contract");
}

// ── /api/v1/hunt ────────────────────────────────────────────────────────────

#[tokio::test]
async fn hunt_reports_a_trivially_true_tree_as_spendable_by_anyone() {
    let base = spawn().await;
    // sigmaProp(true), compiled by the pinned compiler.
    let tree = hex::encode(
        ergo_sandbox::compile_source(
            "sigmaProp(true)",
            3,
            ergo_ser::address::NetworkPrefix::Mainnet,
        )
        .unwrap()
        .tree_bytes,
    );
    let res: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/hunt"))
        .json(&serde_json::json!({ "input": tree }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(res["verdict"], "spendableByAnyone", "{res}");
    assert_eq!(res["probes"].as_array().unwrap().len(), 6);
    assert_eq!(res["selfSynthetic"], true);
}

#[tokio::test]
async fn hunt_accepts_a_self_box_and_a_height() {
    let base = spawn().await;
    let res: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/hunt"))
        .json(&serde_json::json!({
            "input": "1001040ad191e4c6a704047300",
            "height": 1_234_567,
            "selfBox": { "value": 1000000, "registers": { "R4": { "type": "Int", "value": 9 } } }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(res["verdict"], "spendableByAnyone", "{res}");
    assert_eq!(res["selfSynthetic"], false);
    assert!(res["probes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["height"] == 1_234_567));
}

#[tokio::test]
async fn hunt_rejects_a_bad_self_box_as_invalid_input() {
    let base = spawn().await;
    let r = reqwest::Client::new()
        .post(format!("{base}/api/v1/hunt"))
        .json(&serde_json::json!({
            "input": "1001040ad191e4c6a704047300",
            "selfBox": { "registers": { "R4": { "type": "Int", "value": "nope" } } }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_input");
}

// ── /api/v1/eval ────────────────────────────────────────────────────────────

async fn post_eval(base: &str, body: serde_json::Value) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{base}/api/v1/eval"))
        .json(&body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn eval_runs_a_source_scenario_to_a_verdict() {
    let base = spawn().await;
    let res: serde_json::Value = post_eval(
        &base,
        serde_json::json!({ "source": "sigmaProp(HEIGHT > 100)", "height": 200 }),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(res["verdict"], "pass", "{res}");
    assert!(res["cost"].as_u64().unwrap() > 0);
    assert!(res["costLimit"].as_u64().unwrap() > 0);
    assert_eq!(res["reducedTo"], "true");
    assert!(res["treeHex"].as_str().unwrap().starts_with("10"));
    assert!(res["address"].as_str().is_some());
}

#[tokio::test]
async fn eval_reports_a_failing_scenario_as_a_verdict_not_an_error() {
    let base = spawn().await;
    let res: serde_json::Value = post_eval(
        &base,
        serde_json::json!({ "source": "sigmaProp(HEIGHT > 100)", "height": 50 }),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(res["verdict"], "fail", "{res}");
}

#[tokio::test]
async fn eval_keeps_a_runtime_exception_as_an_error_verdict() {
    let base = spawn().await;
    let r = post_eval(
        &base,
        serde_json::json!({ "source": "sigmaProp(SELF.R4[Int].get > 0)", "height": 50 }),
    )
    .await;
    assert_eq!(r.status(), 200);
    let res: serde_json::Value = r.json().await.unwrap();
    assert_eq!(res["verdict"], "error", "{res}");
    assert!(res["error"].as_str().unwrap().contains("None"), "{res}");
}

#[tokio::test]
async fn eval_returns_the_trace_and_a_p2pk_residual() {
    let base = spawn().await;
    let res: serde_json::Value = post_eval(
        &base,
        serde_json::json!({
            "source": "PK(\"3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt\") && sigmaProp(HEIGHT > 1)",
            "network": "testnet",
            "height": 5
        }),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(res["verdict"], "needsProof", "{res}");
    assert!(res["reducedTo"].as_str().unwrap().contains("ProveDlog"));
    assert!(res["trace"].is_array());
}

#[tokio::test]
async fn eval_compile_errors_are_invalid_input_with_the_reason() {
    let base = spawn().await;
    let r = post_eval(
        &base,
        serde_json::json!({ "source": "sigmaProp(HEIGHT >", "height": 5 }),
    )
    .await;
    assert_eq!(r.status(), 400);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_input");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("compile"),
        "{body}"
    );
}

#[tokio::test]
async fn eval_rejects_a_scenario_with_neither_tree_nor_source() {
    let base = spawn().await;
    let r = post_eval(&base, serde_json::json!({ "height": 5 })).await;
    assert_eq!(r.status(), 400);
}

/// One wire convention across the API: every field is camelCase.
#[tokio::test]
async fn inspect_fields_are_camel_case_like_the_other_endpoints() {
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
    assert!(res.get("treeHex").is_some(), "{res}");
    assert!(res.get("rawPlaceholders").is_some(), "{res}");
    assert!(res.get("tree_hex").is_none(), "{res}");
    assert!(res["findings"][0].get("nodeId").is_some(), "{res}");
}

#[tokio::test]
async fn hunt_accepts_data_inputs() {
    let base = spawn().await;
    let tree = hex::encode(
        ergo_sandbox::compile_source(
            "sigmaProp(CONTEXT.dataInputs(0).R4[Long].get > 100L)",
            3,
            ergo_ser::address::NetworkPrefix::Mainnet,
        )
        .unwrap()
        .tree_bytes,
    );
    let res: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/v1/hunt"))
        .json(&serde_json::json!({
            "input": tree,
            "dataInputs": [{ "value": 1, "ergoTree": "10010101",
                             "registers": { "R4": { "type": "Long", "value": 500 } } }]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(res["verdict"], "spendableByAnyone", "{res}");
}
