//! The bank-vault corpus fixture
//! (`examples/incidents/use-bank-vault.json`): the deployed vault tree is a
//! checkable decode, not a claim. The phase-2 flagship rests on the whitelist
//! shape, so the fixture asserts it mechanically — from the deployed wire
//! bytes, by decompilation and byte-identical re-serialization.
//!
//! This fixture caught a real error: the phase-2 design doc hand-decoded the
//! whitelist as `OUTPUTS(0).tokens(2)._1`; the deployed tree checks
//! `INPUTS(0).tokens(0)._1` and requires a faithful continuation at
//! `OUTPUTS(1)`. See the fixture's `decode.provenance`.

use ergo_sandbox::decompile::decompile_bytes;
use ergo_sandbox::inspect::tree_report;

const CORPUS: &str = include_str!("../../examples/incidents/use-bank-vault.json");
const MAP_FIXTURE: &str = include_str!("fixtures/map/use-lp.json");

#[test]
fn the_corpus_vault_is_the_mapped_vault() {
    let corpus: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus parses");
    let map: serde_json::Value = serde_json::from_str(MAP_FIXTURE).expect("map fixture parses");
    let vault = &corpus["vault"];
    // Same box, byte-for-byte, from two independent recorded sources.
    let mapped = &map["boxesByToken"][vault["tokens"][0]["id"].as_str().unwrap()]["items"][0];
    assert_eq!(
        vault, mapped,
        "the corpus vault must be the recorded map box"
    );
    assert_eq!(
        vault["value"].as_u64(),
        Some(292_615_109_709_675),
        "292,615.109709675 ERG — the funded vault"
    );
}

#[test]
fn the_deployed_tree_re_serializes_byte_identical() {
    let corpus: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus parses");
    let tree_hex = corpus["vault"]["ergoTree"].as_str().unwrap();
    assert_eq!(tree_hex, corpus["deployedTree"].as_str().unwrap());
    let bytes = hex::decode(tree_hex).expect("tree is hex");
    let report = tree_report(&bytes).expect("tree parses");
    assert!(
        report.contains("re-serializes byte-identical"),
        "the fixture must carry the canonical deployed bytes"
    );
}

/// The whitelist shape, asserted against the decompiled proposition: the
/// admin-NFT read is INPUT-side at token index 0 (`INPUTS(0).tokens(0)._1`),
/// the dead `dbf655…` branch reads `INPUTS(2).tokens(0)._1`, and the
/// faithful-continuation guards apply to `OUTPUTS(1)`.
#[test]
fn the_whitelist_is_input_side_index_zero_with_a_continuation_guard() {
    let corpus: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus parses");
    let bytes = hex::decode(corpus["vault"]["ergoTree"].as_str().unwrap()).expect("tree is hex");
    let source = decompile_bytes(&bytes).expect("tree decompiles");

    // The whitelist read: INPUTS(0), tokens(0).
    assert!(
        source.contains("val v4 = INPUTS(0).tokens(0)._1"),
        "the whitelisted token must be read from INPUTS(0), tokens(0): {source}"
    );
    // Every whitelist comparison is against that one input-side read.
    for (name, id) in [
        (
            "useFreeMint",
            "40db16e1ed50b16077b19102390f36b41ca35c64af87426d04af3b9340859051",
        ),
        (
            "useArbitrageMint",
            "c79bef6fe21c788546beab08c963999d5ef74151a9b7fd6c1843f626eea0ecf5",
        ),
        (
            "usePayout",
            "a2482fca4ca774ef9d3896977e3677b031597c6e312b0c10d47157bb0d6ed69f",
        ),
        (
            "useUpdateNft",
            "f77b3cac4f77a31aeffaf716070345b3b04330bbba02e27671015129fb74e883",
        ),
    ] {
        assert!(
            source.contains(&format!("v4 == fromBase16(\"{id}\")")),
            "{name} must be whitelisted against the INPUT-side read: {source}"
        );
    }
    // The dead branch: INPUTS(2), tokens(0).
    assert!(
        source.contains("INPUTS(2).tokens(0)._1 == fromBase16(\"dbf655f0f6101cb03316e931a689412126fefbfb7c78bd9869ad6a1a58c1b424\")"),
        "the dead branch must read INPUTS(2), tokens(0): {source}"
    );
    // No token index 2 appears anywhere (the design doc's hand-decode).
    assert!(
        !source.contains(".tokens(2)"),
        "no tokens(2) check exists in the deployed tree: {source}"
    );

    // The continuation guards bind OUTPUTS(1) — the second output.
    assert!(
        source.contains("val v = OUTPUTS(1)"),
        "the continuation is OUTPUTS(1): {source}"
    );
    assert!(
        source.contains("v.propositionBytes == SELF.propositionBytes"),
        "the continuation must preserve the script bytes: {source}"
    );
    assert!(
        source.contains("v2(0) == v3(0)"),
        "the continuation must preserve tokens(0) (id AND amount): {source}"
    );
    assert!(
        source.contains("v2(1)._1 == v3(1)._1"),
        "the continuation must preserve tokens(1)'s id — the amount is NOT pinned: {source}"
    );

    // The three script-authorizer routes require the continuation guards;
    // the useUpdateNft route stands alone (no continuation required) — which
    // is exactly why spending the admin's P2PK box is the guarded route.
    let guarded = "v2(0) == v3(0) && v.propositionBytes == SELF.propositionBytes && v2(1)._1 == v3(1)._1 && (";
    assert!(
        source.contains(&format!("sigmaProp({guarded}")),
        "the script-authorizer routes must sit behind the continuation guards: {source}"
    );
    assert!(
        source.contains(")) || v4 == fromBase16(\"f77b3cac4f77a31aeffaf716070345b3b04330bbba02e27671015129fb74e883\")"),
        "the useUpdateNft route must stand alone outside the guards: {source}"
    );
}

/// The recorded lint verdict is checkable too: `unbound-box-reserves` on the
/// deployed tree's decompiled proposition. The record carries the
/// interpretation, which the test keeps honest.
#[test]
fn the_recorded_lint_verdict_matches_a_live_run() {
    let corpus: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus parses");
    let lint = &corpus["lint"];
    assert_eq!(lint["lint"], "unbound-box-reserves");
    assert_eq!(lint["verdict"], "clean");

    // Re-run: decompile the deployed tree, recompile the proposition,
    // audit it.
    let bytes = hex::decode(corpus["vault"]["ergoTree"].as_str().unwrap()).expect("tree is hex");
    let source = decompile_bytes(&bytes).expect("tree decompiles");
    let compiled = ergo_sandbox::compile::compile_source(
        &source,
        3,
        ergo_ser::address::NetworkPrefix::Mainnet,
    )
    .expect("decompiled proposition recompiles");
    let tree = ergo_sandbox::inspect::parse_tree(&compiled.tree_bytes).expect("tree parses");
    let lifted = ergo_sandbox::decompile::lift_tree(&tree, false);
    let a = ergo_sandbox::audit::audit(&lifted);
    // `unbound-box-reserves` stays clean on this tree — the vault HAS the
    // NFT-binding shape that lint keys on. What the corpus note called the
    // gap ("CLEAN IS NOT SAFE": the successor's ERG value and the treasury
    // AMOUNT are unpinned) is now covered by `delegated-reserves`, so the
    // live audit is no longer empty. Pin both halves: the old verdict is
    // unchanged, and the gap it named is now caught by name.
    assert!(
        !a.findings.iter().any(|f| f.lint == "unbound-box-reserves"),
        "unbound-box-reserves must stay clean on the vault: {:?}",
        a.findings
    );
    let delegated: Vec<_> = a
        .findings
        .iter()
        .filter(|f| f.lint == "delegated-reserves")
        .collect();
    assert_eq!(
        delegated.len(),
        1,
        "delegated-reserves must name the vault's unpinned reserves: {:?}",
        a.findings
    );
    assert!(
        delegated[0].message.contains("ERG value") && delegated[0].message.contains("tokens(1)._2"),
        "the finding must cite BOTH unpinned assets the corpus note predicted: {:?}",
        delegated[0]
    );

    // Clean is not safe: the lint keys on reserve math and NFT-binding
    // identity. The vault binds its successor's identity (script bytes;
    // bank NFT id AND amount) but pins neither the successor's ERG value
    // nor the treasury token's amount — a dust successor is permitted by
    // the vault script alone, and every reserve guarantee is delegated to
    // the three script authorizers. The corpus record says so; this
    // assertion pins that the interpretation stays attached to the verdict.
    let note = lint["note"].as_str().unwrap();
    assert!(
        note.contains("CLEAN IS NOT SAFE") && note.contains("tokens(1) is compared by id only"),
        "the lint record must carry the unpinned-value caveat"
    );
}

/// The corpus carries the people of the whitelist: the three script
/// authorizers (the empirical routes) and the admin's P2PK box (the refused
/// route — `needsProof`, disqualified by the phase-1 gate).
#[test]
fn the_corpus_carries_the_authorizer_boxes() {
    let corpus: serde_json::Value = serde_json::from_str(CORPUS).expect("corpus parses");
    let names: Vec<&str> = corpus["authorizers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["useFreeMint", "useArbitrageMint", "usePayout"]);
    for a in corpus["authorizers"].as_array().unwrap() {
        assert!(a["boxId"].as_str().is_some(), "full box record");
        assert!(a["ergoTree"].as_str().is_some(), "full box record");
    }
    // The admin box is P2PK (tree `0008cd03…`) and carries the update NFT —
    // spending it needs a key the attacker does not hold.
    let admin = &corpus["adminBox"];
    assert!(
        admin["ergoTree"].as_str().unwrap().starts_with("0008cd03"),
        "the admin box is P2PK"
    );
    assert_eq!(
        admin["tokens"][0]["id"].as_str().unwrap(),
        "f77b3cac4f77a31aeffaf716070345b3b04330bbba02e27671015129fb74e883"
    );
}
