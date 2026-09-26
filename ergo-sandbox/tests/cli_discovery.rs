use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

fn temp(name: &str, value: &Value) -> PathBuf {
    let path = std::env::temp_dir().join(format!("forge-{name}-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    path
}

fn run(args: &[String]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn the_discovery_commands_emit_bounded_json() {
    let template = run(&[
        "property-template".into(),
        root()
            .join("examples/property-templates/reserve-conservation.json")
            .display()
            .to_string(),
        "--json".into(),
    ]);
    assert_eq!(template["name"], "reserve-conservation");
    assert_eq!(template["digest"].as_str().unwrap().len(), 64);

    let record = json!({"method": "hunt", "verdict": "notUnderProbes", "probes": []});
    let record_path = temp("shadow-record", &record);
    let shadow = run(&[
        "shadow-check".into(),
        record_path.display().to_string(),
        "--json".into(),
    ]);
    std::fs::remove_file(record_path).unwrap();
    assert_eq!(shadow["nodeValidated"], false);
    assert_eq!(shadow["method"], "static-analysis");

    let id = format!("{:0>64}", "b0");
    let request = json!({
        "height": 1000,
        "boxes": [{"boxId": id, "value": 1000000, "ergoTree": "10010101d17300", "tokens": [], "registers": {}}],
        "tx": {"inputs": [{"boxId": id}], "dataInputs": [], "outputs": [{"value": 1000000, "ergoTree": "10010101d17300"}]},
        "options": {"seed": "cli-test", "maxDepth": 1, "maxProbes": 2}
    });
    let request_path = temp("adversary-request", &request);
    let adversary = run(&[
        "adversary".into(),
        request_path.display().to_string(),
        "--json".into(),
    ]);
    std::fs::remove_file(request_path).unwrap();
    assert_eq!(adversary["nodeValidated"], false);
    assert_eq!(adversary["method"], "adversary-search");
    assert!(adversary["fingerprint"].as_str().is_some());
}
