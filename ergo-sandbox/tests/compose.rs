//! The composer: "who can spend" + "under what conditions", as a list of
//! spending paths, assembled into readable ErgoScript with `$name`
//! parameters — and a generated test suite whose expectations come from the
//! composer's own model of the rules, so running it checks the assembly.

use std::collections::BTreeMap;

use ergo_sandbox::compose::{compose, Spec};
use ergo_sandbox::testsuite;
use ergo_sandbox::TypedValue;

const A: &str = "028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197";
const G: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

fn spec(json: &str) -> Spec {
    serde_json::from_str(json).expect("spec parses")
}
fn tv(t: &str, v: serde_json::Value) -> TypedValue {
    TypedValue {
        r#type: t.into(),
        value: v,
    }
}

#[test]
fn an_inheritance_shaped_spec_composes_to_readable_source_with_params() {
    let s = spec(
        r#"{ "paths": [
        { "name": "owner at any time", "who": { "anyOf": ["owner"] }, "conditions": [] },
        { "name": "heir after the date", "who": { "anyOf": ["heir"] }, "conditions": [ { "after": "heirHeight" } ] }
    ] }"#,
    );
    let out = compose(&s, &BTreeMap::new()).unwrap();
    assert!(out.source.contains("$owner"), "{}", out.source);
    assert!(
        out.source.contains("HEIGHT >= $heirHeight"),
        "{}",
        out.source
    );
    assert!(out.source.contains("||"), "{}", out.source);
    let names: Vec<(&str, &str)> = out
        .params
        .iter()
        .map(|p| (p.name.as_str(), p.type_hint.as_deref().unwrap()))
        .collect();
    assert_eq!(
        names,
        [
            ("owner", "SigmaProp"),
            ("heir", "SigmaProp"),
            ("heirHeight", "Int")
        ]
    );
    assert!(out.suite.is_none(), "no values, no suite");
}

#[test]
fn composed_source_compiles_and_behaves_like_the_recipe() {
    let s = spec(
        r#"{ "paths": [
        { "name": "owner", "who": { "anyOf": ["owner"] }, "conditions": [] },
        { "name": "heir later", "who": { "anyOf": ["heir"] }, "conditions": [ { "after": "heirHeight" } ] }
    ] }"#,
    );
    let mut values = BTreeMap::new();
    values.insert("owner".into(), tv("SigmaProp", serde_json::json!(A)));
    values.insert("heir".into(), tv("SigmaProp", serde_json::json!(G)));
    values.insert("heirHeight".into(), tv("Int", serde_json::json!(1000)));
    let out = compose(&s, &values).unwrap();
    let suite = out.suite.expect("values given → suite generated");
    assert!(suite.scenarios.len() >= 3, "{}", suite.scenarios.len());
    let r = testsuite::run(&suite).unwrap();
    assert_eq!(
        r.failed,
        0,
        "{:#?}",
        r.cases.iter().filter(|c| !c.passed).collect::<Vec<_>>()
    );
}

#[test]
fn payment_and_keep_conditions_allocate_outputs_in_order() {
    let s = spec(
        r#"{ "paths": [
        { "name": "buy", "who": { "anyOne": true },
          "conditions": [ { "payTo": { "key": "seller", "amount": "price" } }, { "keepHere": { "atLeast": "reserve" } } ] },
        { "name": "seller cancels", "who": { "anyOf": ["seller"] }, "conditions": [] }
    ] }"#,
    );
    let mut values = BTreeMap::new();
    values.insert("seller".into(), tv("SigmaProp", serde_json::json!(A)));
    values.insert("price".into(), tv("Long", serde_json::json!(5000)));
    values.insert("reserve".into(), tv("Long", serde_json::json!(1000)));
    let out = compose(&s, &values).unwrap();
    assert!(
        out.source.contains("OUTPUTS(0)") && out.source.contains("OUTPUTS(1)"),
        "{}",
        out.source
    );
    let r = testsuite::run(&out.suite.unwrap()).unwrap();
    assert_eq!(
        r.failed,
        0,
        "{:#?}",
        r.cases.iter().filter(|c| !c.passed).collect::<Vec<_>>()
    );
    // The "anyone" path with its conditions met passes without a key.
    assert!(r.cases.iter().any(|c| c.actual == "pass"), "{:?}", r.cases);
}

#[test]
fn k_of_n_who_and_before_condition() {
    let s = spec(
        r#"{ "paths": [
        { "name": "two of three before the deadline", "who": { "kOf": 2, "keys": ["a", "b", "c"] },
          "conditions": [ { "before": "deadline" } ] }
    ] }"#,
    );
    let mut values = BTreeMap::new();
    for k in ["a", "b", "c"] {
        values.insert(k.into(), tv("SigmaProp", serde_json::json!(A)));
    }
    values.insert("deadline".into(), tv("Int", serde_json::json!(500)));
    let out = compose(&s, &values).unwrap();
    assert!(
        out.source.contains("atLeast(2, Coll($a, $b, $c))"),
        "{}",
        out.source
    );
    let r = testsuite::run(&out.suite.unwrap()).unwrap();
    assert_eq!(
        r.failed,
        0,
        "{:#?}",
        r.cases.iter().filter(|c| !c.passed).collect::<Vec<_>>()
    );
}

#[test]
fn an_empty_spec_or_a_path_without_who_or_conditions_is_rejected() {
    assert!(compose(&spec(r#"{ "paths": [] }"#), &BTreeMap::new()).is_err());
    assert!(
        compose(
            &spec(
                r#"{ "paths": [ { "name": "x", "who": { "anyOne": true }, "conditions": [] } ] }"#
            ),
            &BTreeMap::new()
        )
        .is_err(),
        "anyone with no conditions is spendable by anyone: refused"
    );
}

// ── the wider vocabulary: what a script can actually see ──────────────────

fn compose_ok(
    spec_json: &str,
    values: BTreeMap<String, TypedValue>,
) -> ergo_sandbox::compose::Composed {
    compose(&spec(spec_json), &values).unwrap()
}
fn green(out: &ergo_sandbox::compose::Composed) {
    let suite = out.suite.clone().expect("suite");
    let r = testsuite::run(&suite).unwrap();
    assert_eq!(
        r.failed,
        0,
        "source:\n{}\nfailures: {:#?}",
        out.source,
        r.cases.iter().filter(|c| !c.passed).collect::<Vec<_>>()
    );
}
fn v(pairs: &[(&str, &str, serde_json::Value)]) -> BTreeMap<String, TypedValue> {
    pairs
        .iter()
        .map(|(n, t, x)| (n.to_string(), tv(t, x.clone())))
        .collect()
}

#[test]
fn data_input_rule_the_oracle_box_by_token_and_register() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "owner while price high", "who": { "anyOf": ["owner"] },
        "conditions": [ { "box": { "which": "dataInput", "index": 0,
                                   "token": { "id": "oracleNFT", "atLeast": "one" },
                                   "registers": [ { "reg": "R4", "type": "Long", "op": "gte", "value": "floor" } ] } } ] } ] }"#,
        v(&[
            ("owner", "SigmaProp", serde_json::json!(A)),
            (
                "oracleNFT",
                "Coll[Byte]",
                serde_json::json!("ab".repeat(32)),
            ),
            ("one", "Long", serde_json::json!(1)),
            ("floor", "Long", serde_json::json!(100)),
        ]),
    );
    assert!(
        out.source.contains("CONTEXT.dataInputs(0)"),
        "{}",
        out.source
    );
    assert!(out.source.contains("R4[Long]"), "{}", out.source);
    green(&out);
}

#[test]
fn output_rule_with_self_script_token_and_register_equal_to_height() {
    // A state box: output 0 keeps this script, keeps the token, records HEIGHT in R4.
    let out = compose_ok(
        r#"{ "paths": [ { "name": "tick", "who": { "anyOf": ["owner"] },
        "conditions": [ { "box": { "which": "output", "index": 0, "script": "self",
                                   "keepsSelfTokens": true,
                                   "valueAtLeastShare": { "percent": "pct" },
                                   "registers": [ { "reg": "R4", "type": "Int", "op": "eqHeight" } ] } } ] } ] }"#,
        v(&[
            ("owner", "SigmaProp", serde_json::json!(A)),
            ("pct", "Long", serde_json::json!(90)),
        ]),
    );
    assert!(
        out.source.contains("SELF.propositionBytes"),
        "{}",
        out.source
    );
    assert!(
        out.source.contains("R4[Int].get == HEIGHT")
            || out.source.contains("R4[Int].get == HEIGHT"),
        "{}",
        out.source
    );
    assert!(out.source.contains("SELF.tokens"), "{}", out.source);
    green(&out);
}

#[test]
fn any_and_all_output_rules_and_output_count() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "pay and no token leaks", "who": { "anyOne": true },
        "conditions": [ { "box": { "which": "output", "index": "any", "script": { "key": "seller" }, "valueAtLeast": "price" } },
                        { "box": { "which": "output", "index": "all", "noTokens": true } },
                        { "outputCount": "n" } ] } ] }"#,
        v(&[
            ("seller", "SigmaProp", serde_json::json!(A)),
            ("price", "Long", serde_json::json!(5000)),
            ("n", "Int", serde_json::json!(2)),
        ]),
    );
    assert!(out.source.contains("OUTPUTS.exists"), "{}", out.source);
    assert!(out.source.contains("OUTPUTS.forall"), "{}", out.source);
    assert!(out.source.contains("OUTPUTS.size == $n"), "{}", out.source);
    green(&out);
}

#[test]
fn time_window_box_age_and_input_count() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "aged and timed", "who": { "anyOf": ["k"] },
        "conditions": [ { "afterTime": "t0" }, { "beforeTime": "t1" }, { "boxAge": "age" }, { "inputCount": "ins" } ] } ] }"#,
        v(&[
            ("k", "SigmaProp", serde_json::json!(A)),
            ("t0", "Long", serde_json::json!(1_700_000_000_000i64)),
            ("t1", "Long", serde_json::json!(1_800_000_000_000i64)),
            ("age", "Int", serde_json::json!(100)),
            ("ins", "Int", serde_json::json!(1)),
        ]),
    );
    assert!(
        out.source.contains("CONTEXT.preHeader.timestamp"),
        "{}",
        out.source
    );
    assert!(
        out.source.contains("SELF.creationInfo._1"),
        "{}",
        out.source
    );
    assert!(out.source.contains("INPUTS.size == $ins"), "{}", out.source);
    green(&out);
}

#[test]
fn spender_values_hash_preimage_token_gate_and_paid_total() {
    let secret = hex::encode(b"hunter2");
    let h = ergo_sandbox::compose::hash_hex("blake2b256", b"hunter2").unwrap();
    let out = compose_ok(
        &format!(
            r#"{{ "witness": {{ "0": {{ "type": "Coll[Byte]", "value": "{secret}" }} }}, "paths": [
        {{ "name": "reveal", "who": {{ "anyOne": true }}, "conditions": [ {{ "hashPreimage": {{ "var": 0, "hash": "h", "algo": "blake2b256" }} }} ] }},
        {{ "name": "member pays total", "who": {{ "anyOne": true }},
          "conditions": [ {{ "tokenGated": {{ "tokenId": "member" }} }}, {{ "varEquals": {{ "index": 1, "type": "Int", "value": "code" }} }},
                          {{ "sumPaidTo": {{ "key": "treasury", "atLeast": "due" }} }} ] }} ] }}"#
        ),
        v(&[
            ("h", "Coll[Byte]", serde_json::json!(h)),
            ("member", "Coll[Byte]", serde_json::json!("cd".repeat(32))),
            ("code", "Int", serde_json::json!(42)),
            ("treasury", "SigmaProp", serde_json::json!(A)),
            ("due", "Long", serde_json::json!(700)),
        ]),
    );
    assert!(
        out.source.contains("blake2b256(getVar[Coll[Byte]](0).get)"),
        "{}",
        out.source
    );
    assert!(out.source.contains("INPUTS.exists"), "{}", out.source);
    assert!(out.source.contains("getVar[Int](1)"), "{}", out.source);
    assert!(out.source.contains("OUTPUTS.fold"), "{}", out.source);
    let suite = out.suite.clone().unwrap();
    let names: Vec<&str> = suite.scenarios.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names
            .iter()
            .any(|n| n.contains("reveal: every condition met")),
        "{names:?}"
    );
    assert!(
        suite.scenarios.iter().any(
            |c| c.name.contains("reveal: every") && matches!(c.expect, testsuite::Expect::Pass)
        ),
        "the witness makes the reveal path pass"
    );
    green(&out);
}

#[test]
fn miner_rule_and_self_register_rule() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "only when the box says so", "who": { "anyOf": ["k"] },
        "conditions": [ { "box": { "which": "self", "registers": [ { "reg": "R5", "type": "Boolean", "op": "eq", "value": "flag" } ] } },
                        { "minerIs": "miner" } ] } ] }"#,
        v(&[
            ("k", "SigmaProp", serde_json::json!(A)),
            ("flag", "Boolean", serde_json::json!(true)),
            ("miner", "Coll[Byte]", serde_json::json!(G)),
        ]),
    );
    assert!(out.source.contains("SELF.R5[Boolean]"), "{}", out.source);
    assert!(
        out.source.contains("CONTEXT.minerPubKey") || out.source.contains("minerPubKey"),
        "{}",
        out.source
    );
    green(&out);
}

#[test]
fn contradictory_rules_are_refused_not_silently_green() {
    // Output 0 must carry a token, yet no output may: no transaction can do both.
    let err = compose(&spec(r#"{ "paths": [ { "name": "x", "who": { "anyOf": ["k"] },
        "conditions": [ { "box": { "which": "output", "index": 0, "token": { "id": "t" } } },
                        { "box": { "which": "output", "index": "all", "noTokens": true } } ] } ] }"#),
        &v(&[("k","SigmaProp",serde_json::json!(A)),("t","Coll[Byte]",serde_json::json!("ab".repeat(32)))])).unwrap_err();
    assert!(err.to_string().contains("contradict"), "{err}");
    let err = compose(
        &spec(
            r#"{ "paths": [ { "name": "x", "who": { "anyOf": ["k"] },
        "conditions": [ { "box": { "which": "output", "index": 0 } } ] } ] }"#,
        ),
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("nothing required"), "{err}");
}

#[test]
fn a_tokenless_state_box_may_keep_its_tokens_and_let_none_leave() {
    // keepsSelfTokens is `OUTPUTS(0).tokens == SELF.tokens`; with no tokens
    // on SELF, "no output carries tokens" agrees with it. Valid, and green.
    let out = compose_ok(
        r#"{ "paths": [ { "name": "x", "who": { "anyOf": ["k"] },
        "conditions": [ { "box": { "which": "output", "index": 0, "script": "self", "keepsSelfTokens": true } },
                        { "box": { "which": "output", "index": "all", "noTokens": true } } ] } ] }"#,
        v(&[("k", "SigmaProp", serde_json::json!(A))]),
    );
    green(&out);
}

#[test]
fn token_gate_and_fixed_input_count_agree_in_either_order() {
    for conds in [
        r#"[ { "inputCount": "n" }, { "tokenGated": { "tokenId": "m" } } ]"#,
        r#"[ { "tokenGated": { "tokenId": "m" } }, { "inputCount": "n" } ]"#,
    ] {
        let out = compose_ok(
            &format!(
                r#"{{ "paths": [ {{ "name": "x", "who": {{ "anyOne": true }}, "conditions": {conds} }} ] }}"#
            ),
            v(&[
                ("n", "Int", serde_json::json!(2)),
                ("m", "Coll[Byte]", serde_json::json!("cd".repeat(32))),
            ]),
        );
        green(&out);
    }
}

#[test]
fn the_contracts_own_token_does_not_open_a_token_gate() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "member", "who": { "anyOne": true },
        "conditions": [ { "tokenGated": { "tokenId": "m" } } ] } ] }"#,
        v(&[("m", "Coll[Byte]", serde_json::json!("cd".repeat(32)))]),
    );
    assert!(out.source.contains("bx.id != SELF.id"), "{}", out.source);
    // Only SELF holds the token: the gate must stay shut.
    let mut doc = serde_json::to_value(out.suite.clone().unwrap()).unwrap();
    doc["scenarios"].as_array_mut().unwrap().push(serde_json::json!({
        "name": "only this box holds the token", "expect": "fail", "height": 1,
        "selfBox": { "value": 1000000000, "tokens": [ { "id": "cd".repeat(32), "amount": 1 } ] } }));
    let suite: testsuite::Suite = serde_json::from_value(doc).unwrap();
    let r = testsuite::run(&suite).unwrap();
    assert_eq!(
        r.failed,
        0,
        "{:#?}",
        r.cases.iter().filter(|c| !c.passed).collect::<Vec<_>>()
    );
}

// ── arithmetic and conservation ───────────────────────────────────────────

#[test]
fn kept_funds_may_drop_by_at_most_an_amount() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "withdraw a little", "who": { "anyOf": ["k"] },
        "conditions": [ { "box": { "which": "output", "index": 0, "script": "self", "valueAtLeastSelfMinus": "cap" } } ] } ] }"#,
        v(&[
            ("k", "SigmaProp", serde_json::json!(A)),
            ("cap", "Long", serde_json::json!(100_000_000)),
        ]),
    );
    assert!(
        out.source.contains("OUTPUTS(0).value >= SELF.value - $cap"),
        "{}",
        out.source
    );
    green(&out);
}

#[test]
fn a_token_is_conserved_across_outputs() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "pass the token on", "who": { "anyOf": ["k"] },
        "conditions": [ { "tokenConserved": { "id": "t" } } ] } ] }"#,
        v(&[
            ("k", "SigmaProp", serde_json::json!(A)),
            ("t", "Coll[Byte]", serde_json::json!("ab".repeat(32))),
        ]),
    );
    assert!(out.source.contains("SELF.tokens.fold"), "{}", out.source);
    let suite = out.suite.clone().unwrap();
    assert!(
        suite.scenarios.iter().any(|c| c.name.contains("burning")),
        "{:?}",
        suite.scenarios.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    green(&out);
}

#[test]
fn an_output_mints_a_token_named_after_the_first_input() {
    let out = compose_ok(
        r#"{ "paths": [ { "name": "issue", "who": { "anyOf": ["k"] },
        "conditions": [ { "box": { "which": "output", "index": 0, "script": { "key": "k" }, "mints": { "atLeast": "supply" } } } ] } ] }"#,
        v(&[
            ("k", "SigmaProp", serde_json::json!(A)),
            ("supply", "Long", serde_json::json!(1000)),
        ]),
    );
    assert!(
        out.source
            .contains("OUTPUTS(0).tokens(0)._1 == INPUTS(0).id"),
        "{}",
        out.source
    );
    green(&out);
}
