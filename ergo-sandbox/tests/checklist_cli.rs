//! CLI checks use the same S00 source/control fixtures as the HTTP surface.
use serde_json::{json, Value};
use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn checklist_cli_lists_sources_addresses_and_supplied_scenarios() {
    let source = include_str!("../../examples/contracts/vectors/zero-threshold/vulnerable.es");
    let output = cli(&["checklist", source, "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["rows"].as_array().unwrap().len(), 15);
    assert_eq!(body["nodeValidated"], false);
    let address = body["address"].as_str().unwrap();
    let from_address = cli(&["checklist", address, "--json"]);
    assert!(from_address.status.success());
    assert_eq!(
        body,
        serde_json::from_slice::<Value>(&from_address.stdout).unwrap()
    );

    let text = cli(&["checklist", source]);
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("[static]") && text.contains("[unchecked]") && text.contains("node "));

    let path = std::env::temp_dir().join(format!(
        "forge-checklist-scenario-{}.json",
        std::process::id()
    ));
    std::fs::write(
        &path,
        json!({"vectorIds":["zero-threshold"],"scenario":{"height":200}}).to_string(),
    )
    .unwrap();
    let output = cli(&[
        "checklist",
        source,
        "--scenario",
        path.to_str().unwrap(),
        "--json",
    ]);
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    let row = body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "zero-threshold")
        .unwrap();
    assert_eq!(row["provenance"], "scenario");
    assert_eq!(row["findings"][0]["provenance"], "static");
    assert_eq!(body["artifacts"][0]["nodeValidated"], false);
    assert!(!cli(&["checklist", source, "--scenario"]).status.success());
}
