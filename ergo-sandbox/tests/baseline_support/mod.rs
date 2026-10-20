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
        if path == "Cargo.lock" {
            // Node 55229e00 moves most of the node's dependency graph (num-bigint 0.5 and the
            // crates it pulls in), too many entries to rewrite one by one here. The lockfile must
            // be byte for byte the one reviewed in docs/reports/node-pin-55229e00.md.
            use sha2::{Digest, Sha256};
            let reviewed = std::fs::read_to_string(
                workspace.join("docs/reports/node-pin-55229e00/Cargo.lock.sha256"),
            )
            .unwrap();
            assert_eq!(
                format!("{}  Cargo.lock\n", hex::encode(Sha256::digest(&current))),
                reviewed,
                "unreviewed engine migration edit: {path}"
            );
            return historical;
        }
        if path.ends_with("Cargo.toml") {
            let rev = ergo_sandbox::evidence::case::engine_revision();
            expected = expected.replace("9468043396e5daa2828211bcff4234bc70fae4f0", rev);
            if path == "Cargo.toml" {
                expected = expected
                    .replace("version    = \"0.3.0\"", "version    = \"0.5.0\"")
                    // Node 55229e00 moves to num-bigint 0.5.
                    .replace("num-bigint = \"0.4\"", "num-bigint = \"0.5\"");
            } else if path == "ergo-sandbox/Cargo.toml" {
                // Emission constants (docs/reports/emission-constants.md): `evidence::emission`
                // names the node's `MonetaryParams`, so `ergo-chain-spec` moves from the
                // dev-dependencies to the dependencies. Same line, same revision. Cargo.lock lists
                // both kinds together and does not change.
                let line = format!(
                    "ergo-chain-spec = {{ git = \"https://github.com/arkadianet/ergo\", rev = \"{rev}\" }}\n"
                );
                assert_eq!(expected.matches(&line).count(), 1);
                expected = expected.replace(&line, "").replace(
                    "ergo-validation = { workspace = true }\n",
                    &format!("ergo-validation = {{ workspace = true }}\n{line}"),
                );
            } else if path == "Cargo.lock" {
                expected = expected
                    .split("[[package]]")
                    .map(|block| {
                        if block.contains("git+https://github.com/arkadianet/ergo?") {
                            let block = block.replace("version = \"0.6.0\"", "version = \"0.8.0\"");
                            if block.starts_with("\nname = \"ergo-state\"\n") {
                                // Node a203cc02: ergo-state now depends on serde_json.
                                block.replace(" \"serde\",\n", " \"serde\",\n \"serde_json\",\n")
                            } else {
                                block
                            }
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
                expected = expected.replace(
                    "[[package]]\nname = \"rustls\"\nversion = \"0.23.43\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"0283386ce02abc0151e1761d08802dfe86c173b0b494af5cbc086574e453da06\"",
                    "[[package]]\nname = \"rustls\"\nversion = \"0.23.45\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"0d41d731c7d2f962d1ccc364cec258de3c0e93b38c2fb3ba97ac74513048d634\"",
                );
            }
        } else {
            // Only the node API edits are permitted: the explicit activation argument (X01)
            // and the validation settings node a203cc02 adds to the protocol parameters.
            expected = expected.replace(
                "pub fn node(&self) -> Result<Option<ReemissionRuleInputs>, String> {",
                "pub fn node(\n        &self,\n        activated_script_version: u8,\n    ) -> Result<Option<ReemissionRuleInputs>, String> {")
                .replace("ergo_ser::ergo_tree::check_tree_version_supported(&parsed).map_err(err)?;",
                    "ergo_ser::ergo_tree::check_tree_version_supported(\n                    &parsed,\n                    activated_script_version,\n                )\n                .map_err(err)?;")
                .replace("required(&request.network_rules, \"network rules\")\n        .map_err(input)?\n        .node()",
                    "required(&request.network_rules, \"network rules\")\n        .map_err(input)?\n        .node(context.activated_script_version)")
                .replace("            storage_period: self.storage_period,\n",
                    "            storage_period: self.storage_period,\n            validation_settings: Default::default(),\n");
            // Node 55229e00: the protocol parameters carry the block version (from which the node
            // takes the activated script version) and its rule state, and the re-emission inputs
            // say whether to check rule 123 and with which emission constants (none here).
            expected = expected
                .replace("    pub fn node(&self) -> ProtocolParams {\n        ProtocolParams {\n",
                    "    /// The node's parameters at `block_version`, which the node derives transaction rules and the\n    /// activated script version from (`activated script version = block version - 1`).\n    pub fn node(&self, block_version: u8) -> ProtocolParams {\n        ProtocolParams {\n            missing_core_parameters: 0,\n            disabled_rules: Default::default(),\n            block_rule_inputs: None,\n            block_version,\n")
                .replace("                Ok(Some(ReemissionRuleInputs {\n",
                    "                // The node applies rule 123 only when asked (`check_rules`); these rules are given\n                // to be checked, as before. Emission-box allocation needs the emission constants,\n                // which a network-rule description does not carry, so it is not checked.\n                Ok(Some(ReemissionRuleInputs {\n                    check_rules: true,\n                    emission: None,\n")
                .replace("    let params = required(&request.parameters, \"parameters\")\n        .map_err(input)?\n        .node();\n", "")
                .replace("        .map_err(input)?;\n    let rules = required(&request.network_rules, \"network rules\")",
                    "        .map_err(input)?;\n    let params = required(&request.parameters, \"parameters\")\n        .map_err(input)?\n        .node(context.activated_script_version + 1);\n    let rules = required(&request.network_rules, \"network rules\")");
            // Emission constants (docs/reports/emission-constants.md): an optional `emission`
            // premise inside the enabled network rules, passed to the node as
            // `ReemissionRuleInputs::emission`. Requests without it are unchanged.
            expected = expected
                .replace("use super::{wire::WireBox, EvidenceCase, Origin, Premise};",
                    "use super::{emission::EmissionConstants, wire::WireBox, EvidenceCase, Origin, Premise};")
                .replace("        pay_to_reemission_tree: String,\n    },",
                    "        pay_to_reemission_tree: String,\n        /// The emission constants that let the node check the emission box's allocation. Absent:\n        /// that check does not run.\n        #[serde(default, skip_serializing_if = \"Option::is_none\")]\n        emission: Option<EmissionConstants>,\n    },")
                .replace("                pay_to_reemission_tree,\n            } if !description.is_empty() => {",
                    "                pay_to_reemission_tree,\n                emission,\n            } if !description.is_empty() => {")
                .replace("Emission-box allocation needs the emission constants,\n                // which a network-rule description does not carry, so it is not checked.\n                Ok(Some(ReemissionRuleInputs {\n                    check_rules: true,\n                    emission: None,\n",
                    "Emission-box allocation needs the emission constants:\n                // without `emission` it is not checked.\n                Ok(Some(ReemissionRuleInputs {\n                    check_rules: true,\n                    emission: emission\n                        .as_ref()\n                        .map(|e| e.node(activated_script_version))\n                        .transpose()?,\n");
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
