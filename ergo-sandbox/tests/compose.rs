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
