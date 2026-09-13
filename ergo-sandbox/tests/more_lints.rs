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
            "unconstrained-outputs" => audit::lints::unconstrained_outputs(&lifted.node),
            "successor-field-drift" => audit::lints::successor_field_drift(&lifted.node),
            "trivial-sigma-branch" => audit::lints::trivial_sigma_branch(&lifted.node),
            "unauthenticated-code-execution" => {
                audit::lints::unauthenticated_code_execution(&lifted.node)
            }
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

#[test]
fn output_tail_needs_an_upper_bound_sum_or_universal_constraint() {
    for guard in [
        "OUTPUTS.size > 0",
        "OUTPUTS.size >= 1",
        "OUTPUTS.size != 0",
        "OUTPUTS.exists { (b: Box) => b.value >= SELF.value }",
        "OUTPUTS.forall { (b: Box) => true }",
        "OUTPUTS.map { (b: Box) => b.value }.size > 0",
        "OUTPUTS.fold(0L, { (s: Long, b: Box) => s + 1L }) == 2L",
        "OUTPUTS.size <= OUTPUTS.size + 1",
    ] {
        let src = format!("sigmaProp(OUTPUTS(0).value > 0L && OUTPUTS(1).value > 0L && ({guard}))");
        assert_eq!(source(&src, "unconstrained-outputs").len(), 1, "{guard}");
    }
    for guard in [
        "OUTPUTS.size == 2", "2 >= OUTPUTS.size", "OUTPUTS.size < 3",
        "OUTPUTS.forall { (b: Box) => b.propositionBytes == SELF.propositionBytes }",
        "OUTPUTS.slice(1, OUTPUTS.size).forall { (b: Box) => b.value <= 1000000L }",
        "OUTPUTS.fold(0L, { (s: Long, b: Box) => s + b.value }) == SELF.value",
        "OUTPUTS.map { (b: Box) => b.value }.fold(0L, { (a: Long, b: Long) => a + b }) <= SELF.value",
    ] {
        let src = format!("{{ val first = OUTPUTS(0); sigmaProp(first.value > 0L && ({guard})) }}");
        assert!(source(&src, "unconstrained-outputs").is_empty(), "{guard}");
    }
    assert!(source("sigmaProp(SELF.value > 0L)", "unconstrained-outputs").is_empty());
}

#[test]
fn successor_drift_requires_the_same_field_and_successor() {
    let base = "out.propositionBytes == SELF.propositionBytes && out.value >= SELF.value && out.tokens == SELF.tokens && SELF.R4[Long].get > 0L";
    for check in [
        "out.R4[Long].isDefined",
        "out.R5[Long].get == SELF.R4[Long].get",
        "OUTPUTS(1).R4[Long].get == SELF.R4[Long].get",
        "out.R4[Long].get == 1L",
        "out.R4[Long].isDefined == SELF.R4[Long].isDefined",
    ] {
        let src = format!("{{ val out = OUTPUTS(0); sigmaProp({base} && {check}) }}");
        let f = source(&src, "successor-field-drift");
        assert_eq!(f.len(), 1, "{check}: {f:?}");
        assert!(f[0].message.contains("R4[Long]"));
        assert!(!f[0].message.contains("ERG value"));
    }
    for check in [
        "out.R4[Long].get == SELF.R4[Long].get",
        "out.R4[Long].get == SELF.R4[Long].get + 1L",
        "out.R4[Long] == SELF.R4[Long]",
    ] {
        let src = format!("{{ val out = OUTPUTS(0); sigmaProp({base} && {check}) }}");
        assert!(source(&src, "successor-field-drift").is_empty(), "{check}");
    }
}

#[test]
fn successor_drift_follows_nft_identity_and_self_amount_reads() {
    let base =
        "out.tokens(0) == SELF.tokens(0) && out.value >= SELF.value && SELF.tokens(1)._2 > 0L";
    for check in ["HEIGHT > 0", "OUTPUTS(1).tokens(1)._2 == SELF.tokens(1)._2"] {
        let src = format!("{{ val out = OUTPUTS(0); sigmaProp({base} && {check}) }}");
        let f = source(&src, "successor-field-drift");
        assert_eq!(f.len(), 1, "{check}");
        assert!(f[0].message.contains("tokens(1)._2"));
    }
    let src =
        format!("{{ val out = OUTPUTS(0); sigmaProp({base} && out.tokens(1) == SELF.tokens(1)) }}");
    assert!(source(&src, "successor-field-drift").is_empty());
    assert!(source(
        "sigmaProp(INPUTS(0).propositionBytes == SELF.propositionBytes)",
        "successor-field-drift"
    )
    .is_empty());
}

#[test]
fn sigma_disjuncts_preserve_conjunctive_authority_and_result_scope() {
    for src in [
        "sigmaProp(SELF.R4[Boolean].get || getVar[Boolean](1).get)",
        "sigmaProp(anyOf(Coll(SELF.R4[Boolean].get, OUTPUTS(0).R4[Long].get > 1L)))",
        "{ val branch = getVar[Boolean](1).get; sigmaProp(SELF.R4[Boolean].get || branch) }",
        "sigmaProp(SELF.R4[Boolean].get || true)",
        "SELF.R4[SigmaProp].get || sigmaProp(true)",
        "SELF.R4[SigmaProp].get || getVar[SigmaProp](1).get",
        "atLeast(0, Coll(SELF.R4[SigmaProp].get))",
    ] {
        assert_eq!(source(src, "trivial-sigma-branch").len(), 1, "{src}");
    }
    for src in [
        "sigmaProp(getVar[Int](1).get > 0)",
        "sigmaProp(SELF.R4[Boolean].get && (true || getVar[Boolean](1).get))",
        "SELF.R4[SigmaProp].get || (proveDlog(decodePoint(fromBase16(\"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798\"))) && sigmaProp(getVar[Boolean](1).get))",
        "sigmaProp(SELF.R4[Boolean].get || (false && getVar[Boolean](1).get))",
        "{ val unused = getVar[Boolean](1).get || true; sigmaProp(SELF.R4[Boolean].get) }",
        "sigmaProp(if (getVar[Boolean](1).get) true else SELF.R4[Boolean].get)",
        "sigmaProp(SELF.R4[Boolean].get || SELF.R5[Boolean].get)",
        "atLeast(1, Coll(SELF.R4[SigmaProp].get))",
    ] { assert!(source(src, "trivial-sigma-branch").is_empty(), "{src}"); }
}

#[test]
fn sigma_register_provenance_uses_immutable_box_identity() {
    for guard in [
        "INPUTS(0).tokens(0)._1 == SELF.tokens(0)._1",
        "INPUTS(0).id == SELF.R5[Coll[Byte]].get",
    ] {
        let src = format!("{{ val bound = {guard}; sigmaProp((bound && SELF.R4[Boolean].get) || INPUTS(0).R4[Boolean].get) }}");
        assert!(source(&src, "trivial-sigma-branch").is_empty(), "{guard}");
    }
    let src = "sigmaProp((INPUTS(0).tokens(0)._1 == getVar[Coll[Byte]](2).get && SELF.R4[Boolean].get) || INPUTS(0).R4[Boolean].get)";
    assert_eq!(source(src, "trivial-sigma-branch").len(), 1);
}

#[test]
fn code_authentication_follows_exact_bytes_aliases_and_context_ids() {
    for guard in [
        "getVar[Coll[Byte]](2).get == fromBase16(\"92a304c801\")",
        "getVar[Coll[Byte]](1).get != fromBase16(\"92a304c801\")",
        "getVar[Coll[Byte]](1).get == OUTPUTS(0).R4[Coll[Byte]].get",
        "CONTEXT.getVarFromInput[Coll[Byte]](1, 1).get == fromBase16(\"92a304c801\")",
    ] {
        assert_eq!(
            source(
                &format!("sigmaProp({guard} && executeFromVar[Boolean](1))"),
                "unauthenticated-code-execution"
            )
            .len(),
            1,
            "{guard}"
        );
    }
    for guard in [
        "bytes == fromBase16(\"92a304c801\")",
        "SELF.R4[Coll[Byte]].get == bytes",
        "blake2b256(bytes) == SELF.R4[Coll[Byte]].get",
        "sha256(bytes) == fromBase16(\"aabb\")",
    ] {
        let src = format!("{{ val bytes = getVar[Coll[Byte]](1).get; sigmaProp({guard} && executeFromVar[Boolean](1)) }}");
        assert!(
            source(&src, "unauthenticated-code-execution").is_empty(),
            "{guard}"
        );
    }
}

/// Branch-local syntax deliberately does not establish enforcement.
#[test]
fn new_absence_lints_do_not_claim_path_proofs() {
    assert!(source(
        "sigmaProp(OUTPUTS(0).value > 0L && (OUTPUTS.size == 1 || HEIGHT > 0))",
        "unconstrained-outputs"
    )
    .is_empty());
    assert!(source("sigmaProp(OUTPUTS(0).propositionBytes == SELF.propositionBytes && (OUTPUTS(0).value >= SELF.value || HEIGHT > 0))", "successor-field-drift").is_empty());
    assert!(source("sigmaProp(!(getVar[Coll[Byte]](1).get == fromBase16(\"92a304c801\")) && executeFromVar[Boolean](1))", "unauthenticated-code-execution").is_empty());
}

/// Some deserialize forms are not emitted by this pinned compiler. Exercise
/// their actual NodeKind shapes without claiming compiler/lift coverage.
#[test]
fn code_hooks_cover_global_and_method_deserialisation_ast_forms() {
    use ergo_sandbox::{Node, NodeKind};
    let n = |kind| Node { id: 1, kind };
    let bytes = n(NodeKind::Method(
        Box::new(n(NodeKind::GetVar(1, "Coll[Byte]".into()))),
        "get".into(),
        vec![],
    ));
    let literal = n(NodeKind::Const("fromBase16(\"92a304c801\")".into()));
    for name in ["deserialize[Boolean]", "deserializeTo[Boolean]"] {
        for method in [false, true] {
            let hook = n(if method {
                NodeKind::Method(Box::new(bytes.clone()), name.into(), vec![])
            } else {
                NodeKind::Global(name.into(), vec![bytes.clone()])
            });
            assert_eq!(audit::lints::unauthenticated_code_execution(&hook).len(), 1);
            let check = n(NodeKind::Infix(
                "==",
                Box::new(bytes.clone()),
                Box::new(literal.clone()),
            ));
            let root = n(NodeKind::Infix("&&", Box::new(check), Box::new(hook)));
            assert!(audit::lints::unauthenticated_code_execution(&root).is_empty());
        }
    }
    assert!(
        audit::lints::unauthenticated_code_execution(&n(NodeKind::Raw(
            "DeserializeRegister".into()
        )))
        .is_empty()
    );
}
