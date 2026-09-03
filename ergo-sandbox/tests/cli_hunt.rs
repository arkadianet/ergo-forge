//! `ergo-es hunt` — the CLI face of the spend hunt.

use std::process::Command;

fn ergo_es(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(args)
        .output()
        .expect("run ergo-es");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tree_hex(src: &str) -> String {
    hex::encode(
        ergo_sandbox::compile_source(src, 3, ergo_ser::address::NetworkPrefix::Testnet)
            .unwrap()
            .tree_bytes,
    )
}

#[test]
fn hunt_reports_a_trivially_true_tree_as_spendable_by_anyone() {
    let (ok, out, err) = ergo_es(&["hunt", &tree_hex("sigmaProp(true)")]);
    assert!(ok, "stderr: {err}");
    assert!(out.contains("spendable by anyone"), "stdout: {out}");
    // One line per probe with height, shape, and verdict.
    assert_eq!(out.matches("PASS").count(), 6, "stdout: {out}");
}

#[test]
fn hunt_reports_the_residual_key_for_p2pk() {
    let tree = tree_hex("PK(\"3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt\")");
    let (ok, out, _) = ergo_es(&["hunt", &tree]);
    assert!(ok);
    assert!(out.contains("requires proof"), "stdout: {out}");
    assert!(out.contains("ProveDlog"), "stdout: {out}");
}

#[test]
fn hunt_honours_a_caller_height() {
    let tree = tree_hex("sigmaProp(HEIGHT > 3000000)");
    let (ok, out, _) = ergo_es(&["hunt", &tree, "--height", "3000001"]);
    assert!(ok);
    assert!(out.contains("spendable by anyone"), "stdout: {out}");
    assert!(out.contains("3000001"), "stdout: {out}");
}

#[test]
fn hunt_warns_when_self_is_synthetic_and_probes_error() {
    let tree = tree_hex("sigmaProp(SELF.R4[Int].get > 0)");
    let (ok, out, _) = ergo_es(&["hunt", &tree]);
    assert!(ok);
    assert!(out.contains("not under probes"), "stdout: {out}");
    assert!(out.contains("synthetic"), "stdout: {out}");
}

#[test]
fn hunt_mainnet_does_not_read_a_flag_value_as_the_corpus_limit() {
    // With no node checkout as a sibling this errors either way; the check
    // is that the error is about the corpus, never a limit parse of "123".
    let (_, out, err) = ergo_es(&["hunt", "--mainnet", "--height", "123"]);
    let all = format!("{out}{err}");
    assert!(
        !all.contains("hunted: 123") && !all.contains("invalid limit"),
        "{all}"
    );
}

// ----- ergo-es test -----

#[test]
fn test_command_prints_a_table_and_exits_nonzero_on_a_failing_case() {
    let dir = std::env::temp_dir().join(format!("ergo-es-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("contract.test.json");
    std::fs::write(
        &path,
        r#"{ "source": "sigmaProp(HEIGHT > 100)",
             "scenarios": [
               { "name": "before", "expect": "fail", "height": 50 },
               { "name": "after", "expect": "pass", "height": 150 },
               { "name": "wrong", "expect": "pass", "height": 50 }
             ] }"#,
    )
    .unwrap();
    let (ok, out, _) = ergo_es(&["test", path.to_str().unwrap()]);
    assert!(!ok, "a failing case must exit non-zero\n{out}");
    assert!(
        out.contains("before") && out.contains("after") && out.contains("wrong"),
        "{out}"
    );
    assert!(
        out.contains("2 passed") && out.contains("1 failed"),
        "{out}"
    );
    std::fs::write(
        &path,
        r#"{ "source": "sigmaProp(HEIGHT > 100)",
             "scenarios": [ { "name": "after", "expect": "pass", "height": 150 } ] }"#,
    )
    .unwrap();
    let (ok, out, _) = ergo_es(&["test", path.to_str().unwrap()]);
    assert!(ok, "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- ergo-es validate-tx -----

#[test]
fn validate_tx_command_reports_each_input_and_exits_nonzero_when_invalid() {
    let dir = std::env::temp_dir().join(format!("ergo-es-vtx-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tx.json");
    let a = "aa".repeat(32);
    std::fs::write(&path, format!(r#"{{ "height": 1000,
        "tx": {{ "inputs": [ {{ "boxId": "{a}" }} ], "dataInputs": [],
                 "outputs": [ {{ "value": 999, "ergoTree": "10010101d17300", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1000 }} ] }},
        "boxes": [ {{ "boxId": "{a}", "value": 100, "ergoTree": "10010101d17300", "assets": [], "additionalRegisters": {{}}, "creationHeight": 1 }} ] }}"#)).unwrap();
    let (ok, out, _) = ergo_es(&["validate-tx", path.to_str().unwrap()]);
    assert!(!ok, "ERG is not conserved: must exit non-zero\n{out}");
    assert!(
        out.contains("input 0") && out.contains("pass") && out.contains("ERG not conserved"),
        "{out}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
