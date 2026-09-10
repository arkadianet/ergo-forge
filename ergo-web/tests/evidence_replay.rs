//! Real HTTP, CLI, and production DOM replay contract checks.
use serde_json::{json, Value};
use std::{path::PathBuf, process::Command, sync::OnceLock};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join(format!(
            "ergo-sandbox/tests/fixtures/evidence/claim-vectors/{name}.fixture"
        )))
        .unwrap(),
    )
    .unwrap()
}
fn bundles() -> Vec<Value> {
    let mut incomplete = fixture("sale-fixed-paid");
    incomplete["execution"]["headers"] =
        json!({"status":"missing","reason":"headers deliberately omitted for P08 replay"});
    incomplete["execution"]["parameters"] =
        json!({"status":"missing","reason":"parameters deliberately omitted for P08 replay"});
    vec![
        fixture("use-incident"),
        fixture("sale-fixed-paid"),
        incomplete,
    ]
}
fn cli() -> &'static PathBuf {
    static CLI: OnceLock<PathBuf> = OnceLock::new();
    CLI.get_or_init(|| {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root())
            .args(["build", "-p", "ergo-sandbox", "--bin", "ergo-es"]);
        if !cfg!(debug_assertions) {
            cmd.arg("--release");
        }
        assert!(cmd.status().unwrap().success());
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("ergo-es")
    })
}
async fn spawn(explorer_url: Option<String>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            ergo_web::app::router_with(ergo_web::app::AppConfig {
                explorer_url,
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}")
}
async fn post(base: &str, bundle: &Value) -> Value {
    let response = reqwest::Client::new()
        .post(format!("{base}/api/v2/replay"))
        .json(bundle)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    response.json().await.unwrap()
}
#[tokio::test]
async fn http_and_cli_replay_have_identical_semantic_results() {
    let base = spawn(None).await;
    for (i, bundle) in bundles().iter().enumerate() {
        let path = std::env::temp_dir().join(format!("p08-{}-{i}.json", std::process::id()));
        std::fs::write(&path, serde_json::to_vec(bundle).unwrap()).unwrap();
        let output = Command::new(cli())
            .arg("replay")
            .arg(&path)
            .arg("--json")
            .output()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(output.status.success(), i < 2);
        let expected: Value = serde_json::from_slice(&output.stdout).unwrap();
        let response = post(&base, bundle).await;
        assert_eq!(response["apiVersion"], 2);
        assert_eq!(response["result"], expected);
        assert_eq!(
            expected["status"],
            [
                "confirmed-violation",
                "accepted-nonviolating",
                "incomplete-or-invalid-premises"
            ][i]
        );
    }
    for invalid in [
        json!({"formatVersion":1,"state":"confirmed","nodeValidated":true}),
        json!({"bundle":fixture("use-incident"),"status":"confirmed-violation"}),
    ] {
        let res = reqwest::Client::new()
            .post(format!("{base}/api/v2/replay"))
            .json(&invalid)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 400);
    }
}
fn dom_test(name: &str) {
    let output = Command::new("node")
        .env("ERGO_REPLAY_CLI", cli())
        .current_dir(root())
        .args([
            "--test",
            "--test-name-pattern",
            name,
            "ui/tests/evidence-replay.test.js",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("# pass 1"));
}
#[test]
fn historical_and_hypothetical_state_labels_are_visible() {
    dom_test("historical_and_hypothetical_state_labels_are_visible");
}
#[test]
fn legacy_preflight_does_not_use_node_claim_badge() {
    dom_test("legacy_preflight_does_not_use_node_claim_badge");
}
#[tokio::test]
async fn replay_request_never_fetches_or_broadcasts() {
    // A configured explorer trap covers complete and missing-context requests.
    let trap = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = spawn(Some(format!("http://{}", trap.local_addr().unwrap()))).await;
    for bundle in bundles() {
        let response =
            tokio::time::timeout(std::time::Duration::from_secs(10), post(&base, &bundle))
                .await
                .unwrap();
        assert_eq!(response["apiVersion"], 2);
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), trap.accept())
            .await
            .is_err()
    );
}
