use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn match_cli_source_params_json_text_and_errors() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let source = root.join("examples/contracts/dexy/lp/pool/swap.es");
    let fixture: Value = serde_json::from_str(include_str!(
        "../../examples/incidents/use-lp-drain.deployed-swap.test.json"
    ))
    .unwrap();
    let tree = fixture["tree"].as_str().unwrap();
    // Keep temporary files inside this worktree.
    let params = root.join(format!(".match-cli-{}.json", std::process::id()));
    std::fs::write(
        &params,
        r#"{"feeNumLp":{"type":"Long","value":3},"feeDenomLp":{"type":"Int","value":1000}}"#,
    )
    .unwrap();
    let out = run(&[
        "match",
        source.to_str().unwrap(),
        tree,
        "--params",
        params.to_str().unwrap(),
        "--json",
    ]);
    std::fs::remove_file(params).unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["verdict"], "same_program_with_differing_constants");
    assert_eq!(report["similarity"], 1.0);
    assert!(report["limitation"]
        .as_str()
        .unwrap()
        .contains("does not prove behavioural equivalence"));
    let out = run(&["match", tree, tree]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("same program\n"));
    assert!(text.contains("does not prove behavioural equivalence"));
    let out = run(&["match", tree, "0008d3", "--json"]);
    assert!(out.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["verdict"],
        "different_program"
    );
    for args in [
        vec!["match"],
        vec!["match", tree, "xyz", "--json"],
        vec!["match", tree, tree, "--params"],
        vec!["match", tree, tree, "--bogus"],
        vec!["match", tree, tree, "--params", "unused.json"],
    ] {
        let out = run(&args);
        assert!(!out.status.success(), "{args:?}");
        assert!(out.stdout.is_empty());
    }
}
