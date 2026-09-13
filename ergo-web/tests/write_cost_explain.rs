//! W04 gates use the production router over real HTTP.
use ergo_web::app::{router_with, AppConfig};
use serde_json::{json, Value};
async fn spawn(enabled: bool) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            router_with(AppConfig {
                cost_trace: enabled,
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}")
}
async fn request(base: &str, route: &str, payload: Value) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{base}/api/v1/{route}"))
        .json(&payload)
        .send()
        .await
        .unwrap()
}
async fn post(base: &str, route: &str, payload: Value) -> Value {
    let r = request(base, route, payload).await;
    let status = r.status();
    let body = r.json::<Value>().await.unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["nodeValidated"], false);
    body
}
#[tokio::test]
async fn cost_trace_route_is_behind_feature_flag() {
    assert_eq!(
        AppConfig::default().cost_trace,
        cfg!(feature = "cost-trace")
    );
    for enabled in [false, true] {
        let base = spawn(enabled).await;
        let cfg: Value = reqwest::get(format!("{base}/api/v1/config"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let present = enabled && cfg!(feature = "cost-trace");
        assert_eq!(cfg["costTrace"], present);
        let req = json!({"source":"sigmaProp(HEIGHT > 1)","height":10});
        assert_eq!(
            request(&base, "cost-spans", req.clone()).await.status(),
            if present { 200 } else { 404 }
        );
        let body = post(&base, "eval", req).await;
        assert_eq!(body.get("costSpans").is_some(), present);
    }
}
#[tokio::test]
async fn hot_spots_map_to_source_spans() {
    let base = spawn(true).await;
    let source =
        "// λ\nsigmaProp(OUTPUTS.exists { (o: Box) => o.value >= SELF.value } && HEIGHT > 100)";
    let payload = json!({"source":source,"height":200,"selfBox":{"value":1000},"outputs":[{"value":2000,"ergoTree":"10010101"}]});
    let response = post(&base, "eval", payload.clone()).await;
    assert_eq!(response["mapStatus"], "aligned");
    #[cfg(feature = "cost-trace")]
    {
        use ergo_ser::opcode::{node_opcode, preorder};
        let sc: ergo_sandbox::Scenario = serde_json::from_value(payload).unwrap();
        let out = ergo_sandbox::eval_scenario(&sc).unwrap();
        let positions =
            ergo_sandbox::source_positions::SourcePositions::for_run(&sc, &out).unwrap();
        let spans = &response["costSpans"];
        let total = spans["totalJit"].as_u64().unwrap();
        assert_eq!(total, out.cost_breakdown.last().unwrap().total);
        assert_eq!(
            total,
            ["attributedJit", "ambiguousJit", "unattributedJit"]
                .iter()
                .map(|k| spans[k].as_u64().unwrap())
                .sum::<u64>()
        );
        assert_eq!(
            total,
            spans["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["jit"].as_u64().unwrap())
                .sum::<u64>()
        );
        assert!(spans["attributedJit"].as_u64().unwrap() > 0, "{spans}");
        assert!(spans["ambiguousJit"].as_u64().unwrap() > 0, "{spans}");
        let exact = spans["attributedJit"].as_u64().unwrap() as f64 / total as f64;
        assert!((spans["exactShare"].as_f64().unwrap() - exact).abs() < 1e-12);
        for row in spans["rows"].as_array().unwrap() {
            let candidates = row["candidates"].as_array().unwrap();
            if row["rule"] == "exact" {
                assert_eq!(candidates.len(), 1);
            }
            if row["rule"] == "ambiguous" {
                assert!(candidates.len() > 1);
            }
            for c in candidates {
                let id = c["irId"].as_u64().unwrap();
                if !c["span"].is_null() {
                    let off = positions.map.as_ref().unwrap().offset(id).unwrap();
                    assert_eq!(c["span"]["offset"], off);
                    assert_eq!(c["span"]["kind"], "start");
                    assert!(c["span"].get("end").is_none());
                    let (line, col) = ergo_compiler::span::line_col(source, off);
                    assert_eq!(c["span"]["line"], line);
                    assert_eq!(c["span"]["col"], col);
                }
                if row["rule"] == "exact" {
                    let op = node_opcode(
                        preorder(&positions.tree.body)
                            .find(|(i, _)| *i == id)
                            .unwrap()
                            .1,
                    );
                    assert_eq!(
                        preorder(&positions.tree.body)
                            .filter(|(_, e)| node_opcode(e) == op)
                            .count(),
                        1
                    );
                }
            }
        }
        // Repeat opcode is ambiguous even with different runtime values / n.
        let amounts = spans["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rawLabel"] == "OP:0xC1")
            .unwrap();
        assert_eq!(amounts["rule"], "ambiguous");
        assert_eq!(amounts["candidates"].as_array().unwrap().len(), 2);
        let via_route = post(&base,"cost-spans",json!({"source":source,"height":200,"selfBox":{"value":1000},"outputs":[{"value":2000,"ergoTree":"10010101"}]})).await;
        assert_eq!(via_route["costSpans"], response["costSpans"]);
    }
}

#[tokio::test]
async fn explain_subexpression_matches_full_reduction() {
    let base = spawn(true).await;
    let source = "// current context\nsigmaProp(HEIGHT > 100)";
    for height in [50, 200] {
        let scenario = json!({"source":source,"height":height});
        let full = post(
            &base,
            "explain",
            json!({"scenario":scenario,"selection":{"offset":0,"length":source.len()}}),
        )
        .await;
        assert_eq!(full["expression"]["irId"], 0);
        assert_eq!(full["expression"]["residual"], full["reducedTo"]);
        let off = source.find("HEIGHT").unwrap();
        let sub = post(
            &base,
            "explain",
            json!({"scenario":scenario,"selection":{"offset":off,"length":6}}),
        )
        .await;
        assert!(sub["expression"].is_object(), "{sub}");
        assert!(sub["expression"]["opcode"]
            .as_str()
            .unwrap()
            .starts_with("HEIGHT"));
        for v in sub["expression"]["values"].as_array().unwrap() {
            let i = v["traceIndex"].as_u64().unwrap() as usize;
            assert_eq!(v["value"], sub["values"][i]["value"]);
            assert_eq!(sub["values"][i]["irId"], sub["expression"]["irId"]);
            assert_eq!(v["value"], format!("Int({height})"));
            assert!(full["values"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["irId"] == sub["expression"]["irId"] && f["value"] == v["value"]));
        }
        let (line, col) = ergo_compiler::span::line_col(source, off as u32);
        let point = post(
            &base,
            "explain",
            json!({"scenario":scenario,"selection":{"line":line,"col":col}}),
        )
        .await;
        assert_eq!(point["expression"], sub["expression"]);
    }
    // Root offset zero is uncited by this compiler: whole-contract still
    // denotes the root, while a selection of an unmapped name gets no neighbour.
    for source in [
        "sigmaProp(true)",
        "{ val a = HEIGHT + 1; sigmaProp(a > 100) }",
        "proveDlog(groupGenerator)",
    ] {
        let body=post(&base,"explain",json!({"scenario":{"source":source,"height":200},"selection":{"offset":0,"length":source.len()}})).await;
        assert_eq!(body["expression"]["residual"], body["reducedTo"], "{body}");
        assert!(!body["reducedTo"].is_null());
    }
    let source = "{ val a = HEIGHT + 1; sigmaProp(a > 100) }";
    for (offset, length) in [(2, 3), (6, 1), (1, 1)] {
        let body=post(&base,"explain",json!({"scenario":{"source":source,"height":200},"selection":{"offset":offset,"length":length}})).await;
        assert!(body["expression"].is_null(), "{body}");
        assert_eq!(
            body["message"],
            "no evaluated expression starts in this selection"
        );
    }
    // Selected sigma node uses its own recorded proposition, not the root's.
    let source = "{ sigmaProp(HEIGHT > 100) || proveDlog(groupGenerator) }";
    let off = source.find("sigmaProp").unwrap();
    let body = post(
        &base,
        "explain",
        json!({"scenario":{"source":source,"height":50},"selection":{"offset":off,"length":9}}),
    )
    .await;
    assert_eq!(
        body["expression"]["values"][0]["residual"], "false",
        "{body}"
    );
    assert_ne!(
        body["expression"]["values"][0]["residual"],
        body["reducedTo"]
    );
}

#[tokio::test]
async fn explain_rejects_bad_selections_and_preserves_partial_runs() {
    let base = spawn(false).await;
    for selection in [
        json!({"offset":0,"length":0}),
        json!({"offset":100,"length":1}),
        json!({"offset":1,"length":4294967295u64}),
        json!({"line":0,"col":1}),
        json!({"offset":0,"length":1,"line":1,"col":1}),
    ] {
        assert_eq!(
            request(
                &base,
                "explain",
                json!({"scenario":{"source":"sigmaProp(true)","height":1},"selection":selection})
            )
            .await
            .status(),
            400
        );
    }
    assert_eq!(
        request(
            &base,
            "explain",
            json!({"scenario":{"tree":"10010101","height":1},"selection":{"offset":0,"length":1}})
        )
        .await
        .status(),
        400
    );
    // Runtime exception keeps values already recorded in this run.
    let source = "sigmaProp(HEIGHT > SELF.R4[Int].get)";
    let body = post(
        &base,
        "explain",
        json!({"scenario":{"source":source,"height":250},"selection":{"offset":10,"length":6}}),
    )
    .await;
    assert_eq!(body["verdict"], "error");
    assert_eq!(body["expression"]["values"][0]["value"], "Int(250)");
    let source = "sigmaProp(HEIGHT > 10)";
    let body=post(&base,"explain",json!({"scenario":{"source":source,"height":250,"costLimit":0},"selection":{"offset":10,"length":6}})).await;
    assert_eq!(body["verdict"], "error");
    assert!(body["expression"].is_null());
}

#[tokio::test]
async fn explain_keeps_unicode_substitution_and_sigma_trace_limits_explicit() {
    let base = spawn(false).await;
    let source = "// 😀\r\nsigmaProp(HEIGHT > 10)";
    let offset = source.find("HEIGHT").unwrap();
    let (line, col) = ergo_compiler::span::line_col(source, offset as u32);
    let body = post(
        &base,
        "explain",
        json!({"scenario":{"source":source,"height":99},"selection":{"line":line,"col":col}}),
    )
    .await;
    assert_eq!(body["expression"]["span"]["offset"], offset);
    assert_eq!(body["expression"]["values"][0]["value"], "Int(99)");
    assert_eq!(
        request(
            &base,
            "explain",
            json!({"scenario":{"source":source,"height":99},"selection":{"offset":4,"length":1}})
        )
        .await
        .status(),
        400
    );
    let source = "{ val bytes = fromBase16(\"$data\"); sigmaProp(HEIGHT > bytes.size) }";
    let body=post(&base,"explain",json!({"scenario":{"source":source,"height":99,"params":{"data":{"type":"String","value":"abcdef"}}},"selection":{"offset":0,"length":source.len()}})).await;
    assert_eq!(body["mapStatus"], "substituted-source");
    assert!(body["expression"].is_null());
    let source = "{ proveDlog(groupGenerator) || sigmaProp(HEIGHT > 100) }";
    let body = post(
        &base,
        "explain",
        json!({"scenario":{"source":source,"height":50},"selection":{"offset":2,"length":9}}),
    )
    .await;
    assert!(
        body["expression"]["values"][0]["residual"]
            .as_str()
            .unwrap()
            .starts_with("ProveDlog("),
        "{body}"
    );
    let source="{ (proveDlog(groupGenerator) && proveDlog(decodePoint(fromBase16(\"02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5\")))) || sigmaProp(HEIGHT > 100) }";
    // Select the first start (shared by the conjunction and its first child)
    // through the actual map/values to exercise the compound recorder limit.
    let full=post(&base,"explain",json!({"scenario":{"source":source,"height":50},"selection":{"offset":0,"length":source.len()}})).await;
    assert!(
        full["values"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["value"].as_str().unwrap().ends_with("...")),
        "{full}"
    );
    assert!(!full["expression"]["residual"]
        .as_str()
        .unwrap()
        .ends_with("..."));
}
