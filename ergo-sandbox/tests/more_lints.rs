//! Height policy and data provenance are review obligations, not spend verdicts.

use std::collections::BTreeMap;

use ergo_sandbox::audit::{self, Completeness};
use ergo_sandbox::{compile_source, lift_tree, Finding, Severity, TypedValue};
use ergo_ser::address::NetworkPrefix;

fn findings(tree: &ergo_ser::ergo_tree::ErgoTree, lint: &str) -> Vec<Finding> {
    let mut modes = Vec::new();
    for testnet in [false, true] {
        let lifted = lift_tree(tree, testnet);
        let audit = audit::audit(&lifted);
        assert_eq!(audit.completeness, Completeness::Complete);
        let direct = match lint {
            "height-guards" => audit::lints::height_guards(&lifted.node),
            "trust-assumptions" => audit::lints::trust_assumptions(&lifted.node),
            _ => unreachable!(),
        };
        let f: Vec<_> = audit
            .findings
            .into_iter()
            .filter(|f| f.lint == lint)
            .collect();
        assert_eq!(f.len(), direct.len(), "lint must be registered");
        for finding in &f {
            assert!(finding.ir_id.is_some(), "{finding:?}");
            assert!(finding.message.contains("Review"));
            assert!(finding.message.contains("not evidence of exploitability"));
            assert!(finding.snippet.chars().count() <= audit::SNIPPET_MAX);
        }
        modes.push(f);
    }
    assert_eq!(
        modes[0].len(),
        modes[1].len(),
        "both address rendering networks"
    );
    modes.remove(0)
}

fn source(src: &str, lint: &str) -> Vec<Finding> {
    let compiled = compile_source(src, 3, NetworkPrefix::Testnet).expect(src);
    findings(&compiled.ergo_tree, lint)
}

fn corpus(src: &str, params: serde_json::Value, lint: &str) -> Vec<Finding> {
    let params: BTreeMap<String, TypedValue> = serde_json::from_value(params).unwrap();
    let compiled =
        ergo_sandbox::compile::compile_with_params(src, &params, 3, NetworkPrefix::Testnet)
            .expect("compile corpus");
    findings(&compiled.ergo_tree, lint)
}

#[test]
fn basic_height_lock_is_an_intentional_permissionless_release() {
    let f = corpus(
        include_str!("../../examples/contracts/basics/height-lock.es"),
        serde_json::json!({"unlockHeight": {"type": "Int", "value": 1000}}),
        "height-guards",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, Severity::Low);
    assert!(f[0].message.contains("permissionless"));
}

#[test]
fn crystalpool_deposit_height_selects_the_same_seller_key() {
    let f = source(
        include_str!("../../examples/contracts/crystalpool/deposit.es"),
        "height-guards",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, Severity::Medium);
    assert!(f[0].message.contains("identical branches"));
}

#[test]
fn real_authorised_locks_and_distinct_crystalpool_branches_are_not_flagged() {
    let f = corpus(
        include_str!("../../examples/contracts/recipes/time-lock.es"),
        serde_json::json!({
            "unlockHeight": {"type": "Int", "value": 1000},
            "owner": {"type": "SigmaProp", "value": "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"}
        }),
        "height-guards",
    );
    assert!(f.is_empty(), "{f:?}");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../examples/tests/htlc.test.json")).unwrap();
    assert!(corpus(
        include_str!("../../examples/contracts/recipes/htlc.es"),
        fixture["params"].clone(),
        "height-guards"
    )
    .is_empty());
    assert!(source(
        include_str!("../../examples/contracts/crystalpool/sell-token-for-erg.es"),
        "height-guards"
    )
    .is_empty());
}

#[test]
fn height_release_only_follows_result_alternatives() {
    for src in [
        "sigmaProp(HEIGHT >= 100)",
        "sigmaProp(100 < HEIGHT)",
        "sigmaProp(100 <= HEIGHT)",
        "sigmaProp(HEIGHT > 100 || SELF.R4[Boolean].get)",
        "sigmaProp(anyOf(Coll(HEIGHT > 100, SELF.R4[Boolean].get)))",
        "{ val release = HEIGHT > 100; sigmaProp(release || SELF.R4[Boolean].get) }",
    ] {
        let f = source(src, "height-guards");
        assert_eq!(f.len(), 1, "{src}: {f:?}");
    }
    for src in [
        "sigmaProp(HEIGHT > 100 && SELF.R4[Boolean].get)",
        "sigmaProp(HEIGHT > 100 && HEIGHT < 200)",
        "sigmaProp(HEIGHT == 100)",
        "sigmaProp(HEIGHT < 100)",
        "sigmaProp(HEIGHT > SELF.R4[Int].get)",
        "sigmaProp(HEIGHT > 2147483647)",
        "sigmaProp(HEIGHT > -1)",
        "sigmaProp(!(HEIGHT > 100))",
        "sigmaProp(if (HEIGHT > 100) SELF.R4[Boolean].get else SELF.R5[Boolean].get)",
    ] {
        assert!(source(src, "height-guards").is_empty(), "{src}");
    }
}

#[test]
fn identical_height_branches_are_local_and_do_not_imply_success() {
    let f = source(
        "sigmaProp(if (HEIGHT > SELF.R4[Int].get) SELF.R5[Boolean].get else SELF.R5[Boolean].get)",
        "height-guards",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].message.contains("can still fail"));
    assert!(source(
        "sigmaProp(if (SELF.R4[Int].get > 100) SELF.R5[Boolean].get else SELF.R5[Boolean].get)",
        "height-guards"
    )
    .is_empty());
}

#[test]
fn chaincash_note_authenticates_a_key_without_pinning_reserve_identity() {
    let f = corpus(
        include_str!("../../examples/contracts/chaincash-basis/chaincash/onchain/note.es"),
        serde_json::json!({
            "reserveContractHash": {"type": "String", "value": "11111111111111111111111111111111"},
            "receiptContractHash": {"type": "String", "value": "11111111111111111111111111111112"}
        }),
        "trust-assumptions",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, Severity::Medium);
    assert_eq!(f[0].snippet, "CONTEXT.dataInputs(0).R4[GroupElement]");
    assert!(f[0]
        .message
        .contains("key checks may intentionally suffice"));
}

#[test]
fn real_oracles_and_rosen_token_search_bind_data_provenance() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../examples/tests/oracle-price-gate.test.json"
    ))
    .unwrap();
    assert!(corpus(
        include_str!("../../examples/contracts/basics/oracle-price-gate.es"),
        fixture["params"].clone(),
        "trust-assumptions"
    )
    .is_empty());
    assert!(source(
        include_str!("../../examples/contracts/chaincash-basis/chaincash/onchain/reserve.es"),
        "trust-assumptions"
    )
    .is_empty());
    // Rosen's symbolic base64 placeholder is not a valid encoding. Substitute
    // one fixed 32-byte id, retaining its original any-token-slot predicate.
    let rosen = include_str!("../../examples/contracts/rosen-bridge/Lock.es")
        .replace("GUARD_NFT", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    assert!(source(&rosen, "trust-assumptions").is_empty());
}

fn data_input(extra: &str) -> Vec<Finding> {
    source(&format!("{{ val oracle = CONTEXT.dataInputs(0); sigmaProp(oracle.R4[Long].getOrElse(0L) > 100L && ({extra})) }}"), "trust-assumptions")
}

#[test]
fn provenance_requires_an_identity_anchor_not_presence_or_spender_data() {
    for check in [
        "oracle.R4[Long].isDefined",
        "oracle.tokens(0)._1 == getVar[Coll[Byte]](0).get",
        "oracle.tokens(0)._1 == CONTEXT.dataInputs(1).tokens(0)._1",
        "oracle.id == oracle.id",
        "oracle.propositionBytes == SELF.propositionBytes",
        "oracle.tokens(0)._1 != fromBase16(\"aabb\")",
        "CONTEXT.dataInputs(1).tokens(0)._1 == fromBase16(\"aabb\")",
    ] {
        assert_eq!(data_input(check).len(), 1, "{check}");
    }
    for check in [
        "oracle.tokens(0)._1 == fromBase16(\"aabb\")",
        "fromBase16(\"aabb\") == oracle.tokens(2)._1",
        "oracle.id == SELF.R5[Coll[Byte]].get",
        "oracle.tokens(0)._1 == SELF.tokens(1)._1",
        "oracle.tokens.exists { (t: (Coll[Byte], Long)) => t._1 == SELF.R5[Coll[Byte]].get }",
    ] {
        assert!(data_input(check).is_empty(), "{check}");
    }
}

#[test]
fn trust_findings_are_per_box_and_reads_outside_scope_are_undecided() {
    let f = source("sigmaProp(CONTEXT.dataInputs(0).R4[Long].get > CONTEXT.dataInputs(0).R5[Long].get && CONTEXT.dataInputs(1).R4[Long].get > 0L)", "trust-assumptions");
    assert_eq!(f.len(), 2, "{f:?}");
    assert_ne!(f[0].node_id, f[1].node_id);
    for src in [
        "sigmaProp(CONTEXT.dataInputs(0).R4[Long].isDefined)",
        "sigmaProp(SELF.R4[Long].get > 0L)",
        "sigmaProp(INPUTS(0).R4[Long].get > 0L)",
        "sigmaProp(getVar[Long](0).get > 0L)",
        "sigmaProp(CONTEXT.dataInputs(HEIGHT % 2).R4[Long].get > 0L)",
    ] {
        assert!(source(src, "trust-assumptions").is_empty(), "{src}");
    }
}

/// A whole-tree identity comparison is evidence, not proof of enforcement.
#[test]
fn branch_specific_and_negated_identity_comparisons_are_not_decided() {
    assert!(data_input("oracle.tokens(0)._1 == fromBase16(\"aabb\") || HEIGHT > 100").is_empty());
    assert!(data_input("!(oracle.tokens(0)._1 == fromBase16(\"aabb\"))").is_empty());
}
