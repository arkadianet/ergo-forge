//! Clause recognition: a decompiled contract back into the composer's
//! language, so Read can say what an address means without ErgoScript.

use ergo_sandbox::recognize::plain;

const G: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

fn words(source: &str) -> ergo_sandbox::recognize::Plain {
    let needs = ergo_sandbox::compile::scan_params(source);
    let p: std::collections::BTreeMap<String, ergo_sandbox::TypedValue> = needs
        .iter()
        .map(|n| {
            let t = n.type_hint.clone().unwrap_or("Long".into());
            let v = match t.as_str() {
                "SigmaProp" | "GroupElement" => serde_json::json!(G),
                "Coll[Byte]" => serde_json::json!("aa".repeat(32)),
                "Boolean" => serde_json::json!(true),
                "Int" => serde_json::json!(900_000),
                _ => serde_json::json!(2_000_000_000i64),
            };
            (
                n.name.clone(),
                ergo_sandbox::TypedValue {
                    r#type: t,
                    value: v,
                },
            )
        })
        .collect();
    let out = ergo_sandbox::compile::compile_with_params(
        source,
        &p,
        3,
        ergo_ser::address::NetworkPrefix::Mainnet,
    )
    .unwrap();
    let lifted = ergo_sandbox::lift_tree(&out.ergo_tree, false);
    plain(&lifted)
}

fn recipe(name: &str) -> String {
    std::fs::read_to_string(format!("../examples/contracts/recipes/{name}.es")).unwrap()
}

#[test]
fn inheritance_reads_as_owner_any_time_or_heir_after_the_date() {
    let p = words(&recipe("inheritance"));
    assert!(p.complete, "{:?}", p.paths);
    assert_eq!(p.paths.len(), 2, "{:?}", p.paths);
    assert!(p.paths[0].contains("the key 9fSgJ7"), "{:?}", p.paths);
    assert!(p.paths[1].contains("from block 900000"), "{:?}", p.paths);
}

#[test]
fn two_of_three_reads_as_a_threshold() {
    let p = words(&recipe("two-of-three"));
    assert!(p.complete);
    assert_eq!(p.paths.len(), 1);
    assert!(p.paths[0].starts_with("2 of these 3 keys"), "{:?}", p.paths);
}

#[test]
fn bounty_reads_the_secret_and_the_deadline() {
    let p = words(&recipe("bounty"));
    assert!(p.complete, "{:?}", p.paths);
    assert!(
        p.paths[0].contains("anyone")
            && p.paths[0].contains("reveals a secret")
            && p.paths[0].contains("before block"),
        "{:?}",
        p.paths
    );
    assert!(p.paths[1].contains("from block"), "{:?}", p.paths);
}

#[test]
fn price_gate_reads_the_oracle_box() {
    let p = words(&recipe("price-gate"));
    assert!(p.complete, "{:?}", p.paths);
    assert!(
        p.paths[0].contains("data input 0")
            && p.paths[0].contains("first token is")
            && p.paths[0].contains("R4 is at least"),
        "{:?}",
        p.paths
    );
}

#[test]
fn subscription_reads_outputs_registers_and_this_box() {
    let p = words(&recipe("subscription"));
    assert!(p.complete, "{:?}", p.paths);
    let all = p.paths.join(" | ");
    assert!(
        all.contains("output 0 goes to the key")
            && all.contains("output 1 stays under this contract")
            && all.contains("records the current height"),
        "{all}"
    );
}

#[test]
fn every_recipe_is_put_into_words_completely() {
    for f in std::fs::read_dir("../examples/contracts/recipes").unwrap() {
        let path = f.unwrap().path();
        if path.extension().map(|e| e == "es") != Some(true) {
            continue;
        }
        let p = words(&std::fs::read_to_string(&path).unwrap());
        assert!(
            p.complete && !p.paths.is_empty(),
            "{}: {:?}",
            path.display(),
            p.paths
        );
    }
}

#[test]
fn unknown_shapes_are_quoted_not_invented() {
    let p = words("sigmaProp(SELF.bytes.size > 10 && HEIGHT > 5)");
    assert!(!p.complete);
    assert!(
        p.paths[0].contains("after block 5") && p.paths[0].contains("`"),
        "{:?}",
        p.paths
    );
}

#[test]
fn a_raw_public_key_is_shortened_like_an_address() {
    let p = words("proveDlog(decodePoint(fromBase16(\"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798\"))) && sigmaProp(HEIGHT > 100)");
    assert_eq!(
        p.paths,
        vec!["the key 0279be66…1798, if after block 100"],
        "{:?}",
        p.paths
    );
    assert!(p.complete);
}
