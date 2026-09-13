use ergo_sandbox::{
    identity,
    verify::{verify, Outcome},
    TypedValue,
};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn tree(source: &str) -> String {
    hex::encode(
        ergo_sandbox::compile_source(source, 3, NetworkPrefix::Mainnet)
            .unwrap()
            .tree_bytes,
    )
}

#[test]
fn verify_exact_template_and_mismatch_are_distinct() {
    let source = "sigmaProp(HEIGHT > 100 && SELF.value > 200L)";
    for (target, expected) in [
        (tree(source), Outcome::Exact),
        (
            tree("sigmaProp(HEIGHT > 101 && SELF.value > 201L)"),
            Outcome::Template,
        ),
        // Shares almost every node; an operator change is still a different program.
        (
            tree("sigmaProp(HEIGHT < 100 && SELF.value > 200L)"),
            Outcome::NoMatch,
        ),
    ] {
        let r = verify(&target, source, &BTreeMap::new(), 3, NetworkPrefix::Mainnet).unwrap();
        assert_eq!(r.outcome, expected);
        assert_eq!(r.limitation, identity::LIMITATION);
        assert_eq!(serde_json::to_value(r).unwrap()["nodeValidated"], false);
    }
    let out = ergo_sandbox::compile_source(source, 3, NetworkPrefix::Testnet).unwrap();
    assert_eq!(
        verify(
            &out.p2s_address,
            source,
            &BTreeMap::new(),
            3,
            NetworkPrefix::Testnet
        )
        .unwrap()
        .outcome,
        Outcome::Exact
    );
    assert!(verify(
        &out.p2s_address,
        source,
        &BTreeMap::new(),
        3,
        NetworkPrefix::Mainnet
    )
    .is_err());
    for target in ["", "zzzz", "00", "deadbeef", "box:123"] {
        assert!(verify(target, source, &BTreeMap::new(), 3, NetworkPrefix::Mainnet).is_err());
    }
    assert_eq!(
        [
            Outcome::Exact.exit_code(),
            Outcome::Template.exit_code(),
            Outcome::NoMatch.exit_code()
        ],
        [0, 3, 4]
    );
}

#[test]
fn verify_lists_differing_constants() {
    let source = "sigmaProp(HEIGHT > $h && SELF.value > $v)";
    let params: BTreeMap<String, TypedValue> = serde_json::from_value(
        json!({"h":{"type":"Int","value":100},"v":{"type":"Long","value":200}}),
    )
    .unwrap();
    let target = tree("sigmaProp(HEIGHT > 101 && SELF.value > 202L)");
    let r = verify(&target, source, &params, 3, NetworkPrefix::Mainnet).unwrap();
    assert_eq!(r.outcome, Outcome::Template);
    assert_eq!(r.constant_differences.len(), 2);
    let direct = identity::match_trees(
        &hex::decode(&r.compiled_tree_hex).unwrap(),
        &hex::decode(target).unwrap(),
    )
    .unwrap();
    let diffs = serde_json::to_value(r.constant_differences).unwrap();
    assert_eq!(
        diffs,
        serde_json::to_value(direct.constant_differences).unwrap()
    );
    assert_eq!(diffs[0]["left"]["type"], "Int");
    assert_eq!(diffs[0]["right"]["value"], "Int(101)");
    assert_eq!(diffs[1]["left"]["type"], "Long");
    assert_eq!(diffs[1]["right"]["value"], "Long(202)");
    assert_ne!(diffs[0]["path"], diffs[1]["path"]);
}

#[test]
fn ignored_serialization_differences_are_not_exact_or_template() {
    use ergo_primitives::writer::VlqWriter;
    let source = "sigmaProp(HEIGHT > 100)";
    let mut out = ergo_sandbox::compile_source(source, 3, NetworkPrefix::Mainnet).unwrap();
    out.ergo_tree.has_size = true;
    let mut writer = VlqWriter::new();
    ergo_ser::ergo_tree::write_ergo_tree(&mut writer, &out.ergo_tree).unwrap();
    let bytes = writer.result();
    assert_ne!(bytes, out.tree_bytes);
    assert_eq!(
        identity::match_trees(&out.tree_bytes, &bytes)
            .unwrap()
            .verdict,
        identity::MatchVerdict::SameProgram
    );
    assert_eq!(
        verify(
            &hex::encode(bytes),
            source,
            &BTreeMap::new(),
            3,
            NetworkPrefix::Mainnet
        )
        .unwrap()
        .outcome,
        Outcome::NoMatch
    );
}

#[test]
fn verify_cli_codes_json_text_and_errors() {
    let path = std::env::temp_dir().join(format!("forge-verify-{}.es", std::process::id()));
    std::fs::write(&path, "sigmaProp(HEIGHT > 100)").unwrap();
    for (source, code, outcome) in [
        ("sigmaProp(HEIGHT > 100)", 0, "exact"),
        ("sigmaProp(HEIGHT > 200)", 3, "template"),
        ("sigmaProp(HEIGHT < 100)", 4, "no_match"),
    ] {
        for json in [false, true] {
            let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_ergo-es"));
            cmd.args(["verify", &tree(source), "--source", path.to_str().unwrap()]);
            if json {
                cmd.arg("--json");
            }
            let out = cmd.output().unwrap();
            assert_eq!(
                out.status.code(),
                Some(code),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            if json {
                let r: Value = serde_json::from_slice(&out.stdout).unwrap();
                assert_eq!(r["outcome"], outcome);
                assert_eq!(r["limitation"], identity::LIMITATION);
            } else {
                let text = String::from_utf8(out.stdout).unwrap();
                assert!(text.starts_with(outcome));
                assert!(text.contains(identity::LIMITATION));
            }
        }
    }
    for flags in [
        vec![],
        vec!["--source"],
        vec!["--source", path.to_str().unwrap(), "--bogus"],
        vec!["--source", path.to_str().unwrap(), "--network", "moon"],
    ] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_ergo-es"))
            .args(["verify", "00"])
            .args(flags)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
    }
    std::fs::remove_file(path).unwrap();
}
