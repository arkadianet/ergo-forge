//! Preserve byte-pinned hunt baselines when S02 adds its separate namespace.

use std::path::Path;

use serde_json::Value;

pub fn historical_bytes(workspace: &Path, path: &str) -> Vec<u8> {
    let current = std::fs::read(workspace.join(path)).unwrap();
    if matches!(
        path,
        "Cargo.toml"
            | "Cargo.lock"
            | "ergo-sandbox/Cargo.toml"
            | "ergo-sandbox/src/evidence/validate.rs"
    ) {
        let historical =
            std::fs::read(workspace.join("docs/reports/batch-10/baseline").join(path)).unwrap();
        let old = String::from_utf8(historical.clone()).unwrap();
        let mut expected = old.clone();
        if path.ends_with("Cargo.toml") || path == "Cargo.lock" {
            let rev = ergo_sandbox::evidence::case::engine_revision();
            expected = expected.replace("9468043396e5daa2828211bcff4234bc70fae4f0", rev);
            if path == "Cargo.toml" {
                expected = expected.replace("version    = \"0.3.0\"", "version    = \"0.5.0\"");
            } else if path == "Cargo.lock" {
                expected = expected
                    .split("[[package]]")
                    .map(|block| {
                        if block.contains("git+https://github.com/arkadianet/ergo?") {
                            block.replace("version = \"0.6.0\"", "version = \"0.7.0\"")
                        } else if block.starts_with("\nname = \"ergo-sandbox\"\n")
                            || block.starts_with("\nname = \"ergo-web\"\n")
                        {
                            block.replace("version = \"0.3.0\"", "version = \"0.5.0\"")
                        } else {
                            block.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("[[package]]");
            }
        } else {
            // Only the new node API's explicit activation argument is permitted.
            expected = expected.replace(
                "pub fn node(&self) -> Result<Option<ReemissionRuleInputs>, String> {",
                "pub fn node(\n        &self,\n        activated_script_version: u8,\n    ) -> Result<Option<ReemissionRuleInputs>, String> {")
                .replace("ergo_ser::ergo_tree::check_tree_version_supported(&parsed).map_err(err)?;",
                    "ergo_ser::ergo_tree::check_tree_version_supported(\n                    &parsed,\n                    activated_script_version,\n                )\n                .map_err(err)?;")
                .replace("required(&request.network_rules, \"network rules\")\n        .map_err(input)?\n        .node()",
                    "required(&request.network_rules, \"network rules\")\n        .map_err(input)?\n        .node(context.activated_script_version)");
        }
        assert_eq!(
            current,
            expected.as_bytes(),
            "unreviewed engine migration edit: {path}"
        );
        return historical;
    }
    if path == "docs/roadmap-metrics.json" {
        // X01 appends its measured scoreboard and S03 its family measurement;
        // authenticate the complete historical artifact against the original
        // manifest digest below. Only these appended keys are admitted.
        let historical =
            std::fs::read(workspace.join("docs/reports/batch-10/roadmap-metrics-before.json"))
                .unwrap();
        let mut live: Value = serde_json::from_slice(&current).unwrap();
        let extension = live.as_object_mut().unwrap().remove("scoreboard").unwrap();
        assert_eq!(
            extension
                .as_object()
                .unwrap()
                .keys()
                .collect::<std::collections::BTreeSet<_>>(),
            ["S03", "X01"]
                .iter()
                .map(|s| s.to_string())
                .collect::<std::collections::BTreeSet<_>>()
                .iter()
                .collect()
        );
        assert_eq!(
            live,
            serde_json::from_slice::<Value>(&historical).unwrap(),
            "every historical scoreboard field must remain unchanged"
        );
        return historical;
    }
    if path != "examples/mutants/answer-key.json" {
        return current;
    }
    // Both M00 and D00 keep their original manifest and answer-key digest.
    // Pin the archived bytes AND require every live historical field to match;
    // only the separately gated S02 appendix is outside the hunt measurement.
    let historical =
        std::fs::read(workspace.join("examples/mutants/answer-key.pre-s02.json")).unwrap();
    let mut live: Value = serde_json::from_slice(&current).unwrap();
    assert!(live
        .as_object_mut()
        .unwrap()
        .remove("staticLintPairs")
        .is_some());
    assert_eq!(
        live,
        serde_json::from_slice::<Value>(&historical).unwrap(),
        "every historical answer-key field must remain unchanged"
    );
    historical
}
