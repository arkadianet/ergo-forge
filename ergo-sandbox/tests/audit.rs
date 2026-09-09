//! Audit layer: framework behaviour and lint decisions.

use ergo_sandbox::audit::{self, Completeness};
use ergo_sandbox::{compile_source, lift_tree};
use ergo_ser::address::NetworkPrefix;

/// Compile a source string and lift the resulting tree.
fn lifted(src: &str) -> ergo_sandbox::Lifted {
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).expect("parse");
    lift_tree(&tree, true)
}

#[test]
fn a_fully_lifted_tree_audits_as_complete() {
    let a = audit::audit(&lifted("sigmaProp(HEIGHT > 100)"));
    assert_eq!(a.completeness, Completeness::Complete);
}

#[test]
fn children_covers_every_node_of_a_nested_tree() {
    let l = lifted("sigmaProp(if (HEIGHT > 100) OUTPUTS.size > 1 else INPUTS.size == 1)");
    fn count(n: &ergo_sandbox::Node) -> usize {
        1 + audit::children(n).into_iter().map(count).sum::<usize>()
    }
    assert!(count(&l.node) > 5, "traversal reached too few nodes");
}

/// Lint ids present in an audit of `src`.
fn lints_of(src: &str) -> Vec<&'static str> {
    audit::audit(&lifted(src))
        .findings
        .iter()
        .map(|f| f.lint)
        .collect()
}

#[test]
fn bare_register_get_is_flagged() {
    assert_eq!(
        lints_of("sigmaProp(SELF.R4[Int].get > 5)"),
        vec!["unchecked-get"]
    );
}

#[test]
fn is_defined_conjunction_guards_the_get() {
    assert!(lints_of("sigmaProp(SELF.R4[Int].isDefined && SELF.R4[Int].get > 5)").is_empty());
}

#[test]
fn is_defined_conditional_guards_the_get() {
    assert!(
        lints_of("sigmaProp(if (SELF.R4[Int].isDefined) SELF.R4[Int].get > 5 else false)")
            .is_empty()
    );
}

#[test]
fn get_or_else_is_never_flagged() {
    assert!(lints_of("sigmaProp(OUTPUTS(0).R4[Long].getOrElse(0L) > 5L)").is_empty());
}

#[test]
fn two_unguarded_gets_produce_two_findings_with_distinct_nodes() {
    let a = audit::audit(&lifted(
        "sigmaProp(SELF.R4[Int].get > 5 && SELF.R5[Int].get > 6)",
    ));
    assert_eq!(a.findings.len(), 2, "{:?}", a.findings);
    assert_ne!(a.findings[0].node_id, a.findings[1].node_id);
}

#[test]
fn findings_carry_a_readable_snippet() {
    let a = audit::audit(&lifted("sigmaProp(SELF.R4[Int].get > 5)"));
    assert!(
        a.findings[0].snippet.contains("get"),
        "snippet: {}",
        a.findings[0].snippet
    );
    assert_eq!(a.findings[0].severity, ergo_sandbox::Severity::High);
}

/// Guard collection is conjunctive-only: an `isDefined` under `||` proves
/// nothing (`x.isDefined || y.isDefined` can hold with `x` empty), so the
/// `get` must still be flagged.
#[test]
fn is_defined_under_or_does_not_guard_the_get() {
    assert_eq!(
        lints_of(
            "sigmaProp((SELF.R4[Int].isDefined || SELF.R5[Int].isDefined) && SELF.R4[Int].get > 5)"
        ),
        vec!["unchecked-get"]
    );
}

/// `!x.isDefined` asserts the OPPOSITE — walking into it must not guard.
#[test]
fn negated_is_defined_does_not_guard_the_get() {
    assert_eq!(
        lints_of("sigmaProp(!(SELF.R4[Int].isDefined) && SELF.R4[Int].get > 5)"),
        vec!["unchecked-get"]
    );
}

/// The conjunctive descent that replaced the arbitrary walk still guards
/// through nested `&&` chains.
#[test]
fn is_defined_nested_in_conjunction_still_guards() {
    assert!(lints_of(
        "sigmaProp((HEIGHT > 100 && SELF.R4[Int].isDefined) && SELF.R4[Int].get > 5)"
    )
    .is_empty());
}

// ── severity tiers: who controls the receiver ──────────────────────────────

fn findings_of(src: &str) -> Vec<ergo_sandbox::Finding> {
    audit::audit(&lifted(src)).findings
}

/// A context variable is supplied by the spender with the proof; a missing
/// one fails the spend but cannot lock the box — the spender just supplies it.
#[test]
fn a_get_on_a_context_variable_is_low() {
    let f = findings_of("sigmaProp(getVar[Int](0).get > 5)");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].lint, "unchecked-get");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::Low);
    assert!(
        f[0].message.contains("context variable 0"),
        "{}",
        f[0].message
    );
}

/// Inside a lambda over a collection the receiver is an element the spender
/// chose (inputs, outputs, data inputs) — fragile, not a lock.
#[test]
fn a_get_on_a_lambda_element_is_medium() {
    let f = findings_of("sigmaProp(OUTPUTS.exists { (b: Box) => b.R4[Int].get > 0 })");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::Medium);
    assert!(f[0].message.contains("element"), "{}", f[0].message);
}

/// A guard inside the lambda body still clears the element read.
#[test]
fn a_guarded_lambda_element_get_is_not_flagged() {
    assert!(findings_of(
        "sigmaProp(OUTPUTS.exists { (b: Box) => b.R4[Int].isDefined && b.R4[Int].get > 0 })"
    )
    .is_empty());
}

/// SELF inside a lambda body is still SELF — the tier follows the receiver's
/// root, not the lexical position.
#[test]
fn a_self_get_inside_a_lambda_stays_high() {
    let f = findings_of("sigmaProp(OUTPUTS.exists { (b: Box) => b.value > SELF.R4[Long].get })");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::High);
}

/// A get whose receiver is indexed off a lambda element is still the element's.
#[test]
fn a_get_through_an_element_property_chain_is_medium() {
    let f = findings_of(
        "sigmaProp(INPUTS.forall { (b: Box) => b.tokens(0)._1 == SELF.id && b.R5[Long].get > 0L })",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::Medium);
}

// ── guards held in a val ───────────────────────────────────────────────────
//
// A single-use `val` is inlined by the compiler, so only multi-use guards
// survive to the tree (as `val v2 = v.isDefined; … v2 && … && v2`). The
// lint must follow the binding.

#[test]
fn a_guard_bound_to_a_val_clears_the_get_under_and() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined; sigmaProp(ok && SELF.R4[Int].get > 5 && ok) }"
    )
    .is_empty());
}

#[test]
fn a_guard_bound_to_a_val_clears_the_get_in_the_then_branch() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined && HEIGHT > 10; sigmaProp(if (ok) SELF.R4[Int].get > 5 else ok) }"
    )
    .is_empty());
}

#[test]
fn a_val_guard_under_or_still_proves_nothing() {
    assert_eq!(
        findings_of(
            "{ val ok = SELF.R4[Int].isDefined || SELF.R5[Int].isDefined; sigmaProp(ok && SELF.R4[Int].get > 5 && ok) }"
        )
        .len(),
        1
    );
}

#[test]
fn a_val_guard_does_not_clear_a_get_outside_its_scope() {
    assert_eq!(
        findings_of(
            "{ val ok = SELF.R4[Int].isDefined; sigmaProp(SELF.R4[Int].get > 5 && ok && ok) }"
        )
        .len(),
        1
    );
}

#[test]
fn a_val_guard_composed_of_two_guards_clears_both_gets() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined && SELF.R5[Long].isDefined; sigmaProp(ok && SELF.R4[Int].get > 5 && SELF.R5[Long].get > 1L && ok) }"
    )
    .is_empty());
}

// ── guards through || and negation ────────────────────────────────────────
//
// `!x.isDefined || x.get > 5` runs the right operand only when x is defined;
// `if (!x.isDefined) a else x.get` reaches the else branch the same way.

#[test]
fn a_negated_guard_under_or_clears_the_right_operand() {
    assert!(findings_of("sigmaProp(!(SELF.R4[Int].isDefined) || SELF.R4[Int].get > 5)").is_empty());
}

#[test]
fn a_negated_guard_condition_clears_the_else_branch() {
    assert!(findings_of(
        "sigmaProp(if (!(SELF.R4[Int].isDefined)) false else SELF.R4[Int].get > 5)"
    )
    .is_empty());
}

#[test]
fn a_negated_guard_condition_does_not_clear_the_then_branch() {
    assert_eq!(
        findings_of("sigmaProp(if (!(SELF.R4[Int].isDefined)) SELF.R4[Int].get > 5 else false)")
            .len(),
        1
    );
}

#[test]
fn an_or_chain_of_negated_guards_clears_the_tail() {
    assert!(findings_of(
        "sigmaProp(!(SELF.R4[Int].isDefined) || !(SELF.R5[Long].isDefined) || SELF.R4[Int].get > 5 && SELF.R5[Long].get > 1L)"
    )
    .is_empty());
}

#[test]
fn a_positive_guard_under_or_still_proves_nothing_for_the_right_operand() {
    assert_eq!(
        findings_of("sigmaProp(SELF.R4[Int].isDefined || SELF.R4[Int].get > 5)").len(),
        1
    );
}

#[test]
fn a_negated_val_guard_under_or_clears_the_right_operand() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined; sigmaProp(!ok || SELF.R4[Int].get > 5 && ok) }"
    )
    .is_empty());
}

// ----- findings cite IR nodes -----

#[test]
fn a_finding_carries_the_ir_id_of_its_node() {
    let a = audit::audit(&lifted("sigmaProp(SELF.R4[Int].get > 5)"));
    assert_eq!(a.findings.len(), 1);
    let f = &a.findings[0];
    assert!(f.ir_id.is_some(), "{f:?}");
    // The `.get` is IR node 2: D1(0) → GT(1) → OptionGet(2) → ExtractRegisterAs(3) → SELF(4).
    assert_eq!(f.ir_id, Some(2));
}

// ── unbound-box-reserves ──────────────────────────────────────────────────
//
// The class behind the 2026-09-08 USE/Dexy LP drain: a script does its value
// maths on a box it picked by *position*, with nothing saying which box that
// index must hold.

/// Lint ids reported for a deployed ErgoTree, given as wire hex. Lifted with
/// the mainnet prefix — these fixtures are mainnet boxes, and it is what
/// `ergo-es audit --mainnet` uses.
fn lints_of_tree(tree_hex: &str) -> (Vec<&'static str>, Completeness) {
    let bytes = hex::decode(tree_hex).expect("hex");
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).expect("parse");
    let a = audit::audit(&lift_tree(&tree, false));
    (a.findings.iter().map(|f| f.lint).collect(), a.completeness)
}

/// The USE LP **swap** contract as deployed (box holding swap NFT
/// `ef461517…`). Reads `INPUTS(0).value` and `INPUTS(0).tokens(i)._2` with no
/// NFT binding — the drained contract.
const USE_LP_SWAP_TREE: &str = "100f04000400040404040402040204020500050005ca0f05d00f05ca0f05ca0f05d00f05ca0fd809d601b2a4730000d602db63087201d603b2a5730100d604db63087203d605c17201d60699c172037205d6078cb2720273020002d608998cb27204730300027207d609b2a5730400d1eded93998cb27202730500028cb27204730600027307959172067308929c9c7e7207067e7206067e7309069c7ef07208069a9c7e7205067e730a067e9c7206730b06929c9c7e7205067e7208067e730c069c7ef07206069a9c7e7207067e730d067e9c7208730e06eded93c27209c2a792c17209c1a793db63087209db6308a7";

/// The USE LP **extract** contract (extract NFT `bc685d6a…`). Binds the LP by
/// NFT `4ecaa1aa…` before touching its reserves.
const USE_LP_EXTRACT_TREE: &str = "102a040004000e20f77b3cac4f77a31aeffaf716070345b3b04330bbba02e27671015129fb74e883010104020400040404040402040204000402040005d00f05c8010400040004020402040a04d0050e204ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d4104000402040204000e206a2b821b5727e85beb5e78b4efb9f0250d59cd48481d2ded2c23e91ba1d07c6605000e20fec586b8d7b92b336a5fea060556cbb4ced15d5334dcb7ca9f9a7bb6ca866c42040405ca0105000e2057af5c7446d419e98e2e6fbd4bce9029cd589f8094686c457902feb472f194ec04a00b0404040404000e2078c24bdf41283f45208664cd8eb78e2ffa7fbb29f26ebb43e6b31a46b3b975ae05808095e789c604010005c20105c401d803d601b2a4730000d602db63087201d6038cb2720273010001d19593720373027303d813d604b2a5730400d605db63087204d606db6308a7d607b2a5730500d608db63087207d609b27208730600d60a8c720901d60bb27202730700d60cb27206730800d60d998cb27205730900028c720c02d60e8c720902d60fc17207d610db6501fed611b27210730a00d612b27210730b00d6138cb2db63087212730c0001d614e4c672120704d6159de4c672110405730dd6169c9d720f720e730eedededededededed938cb27205730f00018cb2720673100001938cb27205731100018cb272067312000193c27204c2a792c17204c1a7928cc772040199a373138f8cc7a70199a37314ededededededed9372037315938cb2720873160001720393b27208731700b2720273180093720a8c720b0193720a8c720c0193720e998c720b02720d93720fc1720193c27207c27201938cb2db6308721173190001731aecededed8f720d731b937213731c9199a37214731d919c7215731e7216edededed91720d731f93721373209199a3721473219591b172107322d801d617b27210732300ed938cb2db6308721773240001732590c1721773267327ed8f9c721573287216919c721573297216";

/// The USE **bank** contract (bank NFT `78c24bdf…`). Every box it looks at is
/// bound by NFT or is SELF's successor.
const USE_BANK_TREE: &str = "100e04020400040004000400040204020e2040db16e1ed50b16077b19102390f36b41ca35c64af87426d04af3b93408590510e20c79bef6fe21c788546beab08c963999d5ef74151a9b7fd6c1843f626eea0ecf5040404000e20dbf655f0f6101cb03316e931a689412126fefbfb7c78bd9869ad6a1a58c1b4240e20a2482fca4ca774ef9d3896977e3677b031597c6e312b0c10d47157bb0d6ed69f0e20f77b3cac4f77a31aeffaf716070345b3b04330bbba02e27671015129fb74e883d804d601b2a5730000d602db63087201d603db6308a7d6048cb2db6308b2a473010073020001d1ecededed93b27202730300b2720373040093c27201c2a7938cb27202730500018cb2720373060001ececec93720473079372047308938cb2db6308b2a4730900730a0001730b937204730c937204730d";

/// Swap shape: reserves of INPUTS(0)/OUTPUTS(0) drive the maths, no NFT.
#[test]
fn positional_reserves_without_an_nft_are_flagged() {
    let f = findings_of(
        "{ val lp = INPUTS(0); val out = OUTPUTS(0);          sigmaProp(out.value - lp.value >= 0L && OUTPUTS(1).propositionBytes == SELF.propositionBytes) }",
    );
    let keys: Vec<&str> = f
        .iter()
        .filter(|x| x.lint == "unbound-box-reserves")
        .map(|x| x.message.as_str())
        .collect();
    assert_eq!(keys.len(), 2, "{f:?}");
    assert!(keys.iter().any(|m| m.contains("INPUTS(0)")), "{keys:?}");
    assert!(keys.iter().any(|m| m.contains("OUTPUTS(0)")), "{keys:?}");
    assert!(f
        .iter()
        .filter(|x| x.lint == "unbound-box-reserves")
        .all(|x| x.severity == ergo_sandbox::Severity::High));
}

/// The same tree with the LP pinned by its NFT is clean.
#[test]
fn an_nft_bound_box_is_not_flagged() {
    assert!(lints_of(
        "{ val lp = INPUTS(0); val out = OUTPUTS(0);          sigmaProp(lp.tokens(0)._1 == fromBase16(\"aabb\") && out.tokens(0)._1 == fromBase16(\"aabb\")          && out.value - lp.value >= 0L) }"
    )
    .is_empty());
}

/// A self-validating pool: reserves on SELF and on the output that carries
/// SELF's own script. Nothing here is substitutable.
#[test]
fn a_self_successor_pool_is_not_flagged() {
    assert!(lints_of(
        "{ val succ = OUTPUTS(0);          sigmaProp(succ.propositionBytes == SELF.propositionBytes && succ.value >= SELF.value) }"
    )
    .is_empty());
}

/// SigmaUSD-style: reserve maths on SELF alone. SELF is never positional.
#[test]
fn reserves_read_only_from_self_are_not_flagged() {
    assert!(lints_of("sigmaProp(SELF.value * 2L > SELF.tokens(0)._2 + 1L)").is_empty());
}

/// A box that is merely passed along — its script compared, no reserve read —
/// is not the vulnerable shape.
#[test]
fn a_positional_box_without_reserve_math_is_not_flagged() {
    assert!(lints_of(
        "sigmaProp(INPUTS(1).propositionBytes == SELF.propositionBytes && INPUTS(2).tokens(0)._1 == fromBase16(\"aabb\"))"
    )
    .is_empty());
}

/// Binding a box against another *positional* box's NFT slot binds nothing:
/// the spender places both.
#[test]
fn binding_to_another_positional_box_does_not_count() {
    let f = findings_of(
        "sigmaProp(INPUTS(0).tokens(0)._1 == INPUTS(1).tokens(0)._1 && INPUTS(0).value > 1L)",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].lint, "unbound-box-reserves");
}

/// A data input read positionally is the same hazard.
#[test]
fn an_unbound_data_input_reserve_read_is_flagged() {
    let f = findings_of("sigmaProp(CONTEXT.dataInputs(0).value > 1000000L)");
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].message.contains("CONTEXT.dataInputs(0)"),
        "{}",
        f[0].message
    );
}

/// **Regression, real chain.** The drained USE LP swap contract must flag.
#[test]
fn the_deployed_use_lp_swap_contract_is_flagged() {
    let (lints, completeness) = lints_of_tree(USE_LP_SWAP_TREE);
    assert_eq!(completeness, Completeness::Complete);
    assert!(
        lints.contains(&"unbound-box-reserves"),
        "the drained swap contract must flag: {lints:?}"
    );
}

/// **Regression, real chain.** The extract contract binds the LP by NFT.
#[test]
fn the_deployed_use_lp_extract_contract_is_clean() {
    let (lints, completeness) = lints_of_tree(USE_LP_EXTRACT_TREE);
    assert_eq!(completeness, Completeness::Complete);
    assert!(
        !lints.contains(&"unbound-box-reserves"),
        "extract binds the LP by NFT 4ecaa1aa…: {lints:?}"
    );
}

/// **Regression, real chain.** The bank contract binds every box it reads.
#[test]
fn the_deployed_use_bank_contract_is_clean() {
    let (lints, completeness) = lints_of_tree(USE_BANK_TREE);
    assert_eq!(completeness, Completeness::Complete);
    assert!(
        !lints.contains(&"unbound-box-reserves"),
        "the bank binds every box by NFT: {lints:?}"
    );
}

/// **Incident-corpus linkage.** The patched swap shipped in
/// `examples/incidents/fixed/use-lp-swap.es` is the same maths as the deployed
/// swap (which `the_deployed_use_lp_swap_contract_is_flagged` proves the lint
/// flags) with the one missing check restored: `INPUTS(0).tokens(0)._1 ==
/// $lpNft`. The lint must clear it — this ties the replay corpus to the audit
/// layer, so a redeployment of the drained shape cannot pass review.
#[test]
fn the_incident_corpus_fixed_swap_is_clean() {
    use std::collections::BTreeMap;
    // The LP singleton NFT the fixed swap binds the pool by.
    let mut params = BTreeMap::new();
    params.insert(
        "lpNft".to_string(),
        ergo_sandbox::TypedValue {
            r#type: "Coll[Byte]".to_string(),
            value: serde_json::json!(
                "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41"
            ),
        },
    );
    let src = include_str!("../../examples/incidents/fixed/use-lp-swap.es");
    let bytes = ergo_sandbox::compile::compile_with_params(src, &params, 3, NetworkPrefix::Mainnet)
        .expect("compile fixed swap")
        .tree_bytes;
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).expect("parse");
    let a = audit::audit(&lift_tree(&tree, false));
    let lints: Vec<&str> = a.findings.iter().map(|f| f.lint).collect();
    assert_eq!(a.completeness, Completeness::Complete);
    assert!(
        !lints.contains(&"unbound-box-reserves"),
        "the fixed swap binds INPUTS(0) by the LP NFT: {lints:?}"
    );
}

/// Binding by the whole `(id, amount)` pair pins `tokens(0)` just as an `._1`
/// equality does — the oracle-pool refresh shape.
#[test]
fn binding_by_the_whole_token_pair_counts() {
    assert!(lints_of(
        "{ val lp = INPUTS(0); val out = OUTPUTS(0); \
         sigmaProp(lp.tokens(0)._1 == fromBase16(\"aabb\") && out.tokens(0) == lp.tokens(0) \
         && out.value - lp.value >= 0L) }"
    )
    .is_empty());
}

/// So does an equality on the whole `tokens` collection.
#[test]
fn binding_by_the_whole_tokens_collection_counts() {
    assert!(lints_of(
        "{ val out = OUTPUTS(0); sigmaProp(out.tokens == SELF.tokens && out.value > SELF.value) }"
    )
    .is_empty());
}

#[test]
fn guarded_successor_alias_survives_compilation_and_is_not_reported() {
    let tree = lifted(
        r#"{
        val successor = OUTPUTS(0)
        val wellFormed = OUTPUTS(0).R4[Long].isDefined && OUTPUTS(0).R5[Long].isDefined
        if (!wellFormed) sigmaProp(false) else sigmaProp(
            successor.R4[Long].get > 0L && successor.R5[Long].get > 0L &&
            successor.value == SELF.value)
    }"#,
    );
    let source = ergo_sandbox::decompile::print(&tree.node);
    assert!(
        source.contains("val "),
        "alias must survive compilation: {source}"
    );
    let findings = audit::audit(&tree).findings;
    assert!(
        findings.iter().all(|f| f.lint != "unchecked-get"),
        "{findings:?}"
    );
}

#[test]
fn unguarded_successor_alias_is_still_reported_after_compilation() {
    let findings = findings_of(
        r#"{
        val successor = OUTPUTS(0)
        sigmaProp(OUTPUTS(0).R4[Long].isDefined &&
            successor.R4[Long].get > 0L && successor.R5[Long].get > 0L &&
            successor.value == SELF.value)
    }"#,
    );
    let gets: Vec<_> = findings
        .iter()
        .filter(|f| f.lint == "unchecked-get")
        .collect();
    assert_eq!(gets.len(), 1, "{findings:?}");
    assert!(gets[0].snippet.contains("R5[Long]"));
    assert_eq!(gets[0].severity, ergo_sandbox::Severity::High);
}
