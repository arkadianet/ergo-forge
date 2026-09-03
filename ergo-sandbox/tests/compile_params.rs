//! Compile-time parameters: environment constants, string substitution,
//! and discovery of what a source needs.

use std::collections::BTreeMap;

use ergo_sandbox::compile::{compile_with_params, scan_params, ParamError};
use ergo_sandbox::TypedValue;
use ergo_ser::address::NetworkPrefix;

fn params(pairs: &[(&str, &str, serde_json::Value)]) -> BTreeMap<String, TypedValue> {
    pairs
        .iter()
        .map(|(n, t, v)| {
            (
                n.to_string(),
                TypedValue {
                    r#type: t.to_string(),
                    value: v.clone(),
                },
            )
        })
        .collect()
}

#[test]
fn a_dollar_identifier_resolves_through_the_environment() {
    let src = "sigmaProp(HEIGHT > $minHeight)";
    let p = params(&[("minHeight", "Int", serde_json::json!(100))]);
    let out = compile_with_params(src, &p, 3, NetworkPrefix::Mainnet).unwrap();
    let plain =
        ergo_sandbox::compile_source("sigmaProp(HEIGHT > 100)", 3, NetworkPrefix::Mainnet).unwrap();
    assert_eq!(out.tree_bytes, plain.tree_bytes);
}

#[test]
fn a_bare_identifier_resolves_through_the_environment_too() {
    let src = "sigmaProp(HEIGHT > MinHeight)";
    let p = params(&[("MinHeight", "Int", serde_json::json!(100))]);
    assert!(compile_with_params(src, &p, 3, NetworkPrefix::Mainnet).is_ok());
}

#[test]
fn a_string_parameter_is_substituted_inside_string_literals() {
    let src = "sigmaProp(SELF.tokens(0)._1 == fromBase16(\"$nft\"))";
    let nft = "aa".repeat(32);
    let p = params(&[("nft", "String", serde_json::json!(nft))]);
    let out = compile_with_params(src, &p, 3, NetworkPrefix::Mainnet).unwrap();
    let plain = ergo_sandbox::compile_source(
        &format!("sigmaProp(SELF.tokens(0)._1 == fromBase16(\"{nft}\"))"),
        3,
        NetworkPrefix::Mainnet,
    )
    .unwrap();
    assert_eq!(out.tree_bytes, plain.tree_bytes);
}

#[test]
fn a_coll_byte_parameter_is_substituted_as_hex_inside_string_literals() {
    let src = "sigmaProp(SELF.tokens(0)._1 == fromBase16(\"$nft\"))";
    let nft = "bb".repeat(32);
    let p = params(&[("nft", "Coll[Byte]", serde_json::json!(nft))]);
    assert!(compile_with_params(src, &p, 3, NetworkPrefix::Mainnet).is_ok());
}

#[test]
fn a_pubkey_parameter_becomes_a_real_prove_dlog() {
    let src = "$owner";
    let p = params(&[(
        "owner",
        "SigmaProp",
        serde_json::json!("028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197"),
    )]);
    let out = compile_with_params(src, &p, 3, NetworkPrefix::Testnet).unwrap();
    assert!(hex::encode(&out.tree_bytes).contains("028333f9f7"));
}

#[test]
fn missing_parameters_are_a_structured_error() {
    let src = "sigmaProp(HEIGHT > $minHeight && SELF.value > $minValue)";
    let p = params(&[("minHeight", "Int", serde_json::json!(1))]);
    match compile_with_params(src, &p, 3, NetworkPrefix::Mainnet) {
        Err(ParamError::Missing(names)) => assert_eq!(names, vec!["minValue".to_string()]),
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn scan_finds_names_with_hints_and_ignores_comments() {
    let src = "{\n  // $oracleNFT: Coll[Byte]\n  // $minerFee: Long\n  // not a param: $commented\n  val ok = SELF.tokens(0)._1 == $oracleNFT && OUTPUTS(0).value >= $minerFee\n  sigmaProp(ok && $flag)\n}";
    let needs = scan_params(src);
    let names: Vec<&str> = needs.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["oracleNFT", "minerFee", "flag"]);
    assert_eq!(needs[0].type_hint.as_deref(), Some("Coll[Byte]"));
    assert_eq!(needs[1].type_hint.as_deref(), Some("Long"));
    assert_eq!(needs[2].type_hint, None);
}

#[test]
fn scan_reports_each_name_once_and_skips_block_comments() {
    let src = "/* $inComment */ sigmaProp($a && $a && $b)";
    let names: Vec<String> = scan_params(src).into_iter().map(|n| n.name).collect();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn compile_errors_keep_their_offset() {
    let src = "sigmaProp(HEIGHT >";
    match compile_with_params(src, &BTreeMap::new(), 3, NetworkPrefix::Mainnet) {
        Err(ParamError::Compile(e)) => assert!(e.pos() > 0, "pos {}", e.pos()),
        other => panic!("expected Compile, got {other:?}"),
    }
}

/// Real contracts write `val bankNFT = fromBase64("$bankNFT")`: the source
/// binds the bare name itself, so the environment must not also define it.
#[test]
fn a_dollar_param_does_not_collide_with_a_val_of_the_same_name() {
    let src = "{ val nft = fromBase16(\"$nft\"); sigmaProp(SELF.tokens(0)._1 == nft) }";
    let p = params(&[("nft", "Coll[Byte]", serde_json::json!("cc".repeat(32)))]);
    assert!(compile_with_params(src, &p, 3, NetworkPrefix::Mainnet).is_ok());
}

/// Bare environment names (`PoolNFT`) carry no `$`, so the scan cannot see
/// them; the compiler's "not found in env" is turned into a Missing error.
#[test]
fn a_missing_bare_environment_name_is_reported_as_missing() {
    let src = "sigmaProp(SELF.tokens(0)._1 == PoolNFT)";
    match compile_with_params(src, &BTreeMap::new(), 3, NetworkPrefix::Mainnet) {
        Err(ParamError::Missing(names)) => assert_eq!(names, vec!["PoolNFT".to_string()]),
        other => panic!("expected Missing, got {other:?}"),
    }
}

/// Rosen Bridge style: an all-caps token as the whole string literal,
/// substituted textually by the deploy script. Discovered and substituted.
#[test]
fn an_all_caps_string_token_is_a_string_param() {
    let src = "sigmaProp(SELF.tokens(0)._1 == fromBase16(\"RWT_REPO_NFT\"))";
    let needs = scan_params(src);
    assert_eq!(needs.len(), 1);
    assert_eq!(needs[0].name, "RWT_REPO_NFT");
    assert_eq!(needs[0].type_hint.as_deref(), Some("String"));
    let p = params(&[("RWT_REPO_NFT", "String", serde_json::json!("dd".repeat(32)))]);
    assert!(compile_with_params(src, &p, 3, NetworkPrefix::Mainnet).is_ok());
}

// ----- EIP-5 templates -----

#[test]
fn an_eip5_template_compiles_with_its_parameters_applied() {
    let src = "/** Height lock.\n * @param threshold the minimum height\n */\n@contract def heightLock(threshold: Int) = sigmaProp(HEIGHT > threshold)";
    let p = params(&[("threshold", "Int", serde_json::json!(100))]);
    let out = compile_with_params(src, &p, 3, NetworkPrefix::Testnet).unwrap();
    let plain =
        ergo_sandbox::compile_source("sigmaProp(HEIGHT > 100)", 3, NetworkPrefix::Testnet).unwrap();
    // Same proposition; the template tree carries a v3 header, so compare
    // the decompiled source rather than the header bytes.
    let t1 = ergo_sandbox::inspect::parse_tree(&out.tree_bytes).unwrap();
    let t2 = ergo_sandbox::inspect::parse_tree(&plain.tree_bytes).unwrap();
    assert_eq!(
        ergo_sandbox::decompile::print(&ergo_sandbox::lift_tree(&t1, true).node),
        ergo_sandbox::decompile::print(&ergo_sandbox::lift_tree(&t2, true).node)
    );
}

#[test]
fn an_eip5_template_default_fills_a_missing_parameter() {
    let src = "/** Height lock.\n * @param threshold the minimum height\n */\n@contract def heightLock(threshold: Int = 1000) = sigmaProp(HEIGHT > threshold)";
    assert!(compile_with_params(src, &BTreeMap::new(), 3, NetworkPrefix::Testnet).is_ok());
}

#[test]
fn an_eip5_template_without_a_default_reports_the_missing_parameter() {
    let src = "/** Height lock.\n * @param threshold the minimum height\n */\n@contract def heightLock(threshold: Int) = sigmaProp(HEIGHT > threshold)";
    match compile_with_params(src, &BTreeMap::new(), 3, NetworkPrefix::Testnet) {
        Err(ParamError::Missing(names)) => assert_eq!(names, vec!["threshold".to_string()]),
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn scan_lists_eip5_parameters_with_types_and_defaults() {
    let src = "/** Mixed.\n * @param base base\n * @param delta delta\n */\n@contract def mixed(base: Long, delta: Long = 5L) = sigmaProp(base + delta > 0L)";
    let needs = scan_params(src);
    assert_eq!(needs.len(), 2, "{needs:?}");
    assert_eq!(needs[0].name, "base");
    assert_eq!(needs[0].type_hint.as_deref(), Some("Long"));
    assert_eq!(needs[0].default, None);
    assert_eq!(needs[1].name, "delta");
    assert_eq!(needs[1].default.as_deref(), Some("5"));
}
