use ergo_sandbox::evidence::wire::{CandidateSpec, CreationReference, TokenSpec, WireBox};
use ergo_sandbox::properties::schema::*;
use serde_json::{json, Value};

fn source() -> Value {
    json!({"origin":"hypothetical","reference":"local:author-review","sha256":"ab".repeat(32)})
}
fn declaration() -> Value {
    json!({
        "schemaVersion":"author-property:v1", "propertyId":"reserve-change", "revision":"1",
        "contracts":{"reserve":{"script":"10010100d17300","compilerRevision":"pinned-author-compiler"}},
        "scope":{"kind":"transition"},
        "roles":{
            "before":{"collection":"inputs","selector":{"kind":"position","index":0},"cardinality":"exactly-one"},
            "after":{"collection":"outputs","selector":{"kind":"script","hex":"10010100d17300"},"cardinality":"all-matches"}
        },
        "registers":{"liability":{"role":"before","index":4,"valueType":"long","unit":{"kind":"nano-erg"}}},
        "guard":{"op":"boolean","value":true},
        "assertion":{"op":"ge","left":{"op":"sub","left":{"op":"sum-erg","role":"after"},"right":{"op":"sum-erg","role":"before"}},"right":{"op":"integer","value":"0","unit":{"kind":"nano-erg"}}},
        "authorizationPremises":[{"id":"funding","statement":"Only supplied test funding credentials are available","source":source()}],
        "sources":[source()]
    })
}
fn parse(v: &Value) -> Result<Declaration, SchemaError> {
    Declaration::parse(&v.to_string())
}
fn rejected(v: &Value) {
    assert!(parse(v).is_err(), "unexpected acceptance: {v}");
}
fn boolean() -> Value {
    json!({"op":"boolean","value":true})
}
fn integer(value: &str, kind: &str) -> Value {
    json!({"op":"integer","value":value,"unit":{"kind":kind}})
}
fn equality(left: Value, right: Value) -> Value {
    json!({"op":"eq","left":left,"right":right})
}

#[test]
fn property_versions_units_and_limits_fail_closed() {
    // The frozen D00 manifest owns numeric requirements; implementation ceilings
    // must agree with it before any boundary examples can count as passing.
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/properties/manifest.json")).unwrap();
    for (key, ceiling) in [
        ("expressionNodes", MAX_EXPRESSION_NODES),
        ("roles", MAX_ROLES),
        ("boxesPerCollection", MAX_BOXES),
        ("tokensPerBox", MAX_TOKENS),
        ("transactionsPerTrace", MAX_TRANSACTIONS),
    ] {
        assert_eq!(manifest["budgets"][key], json!(ceiling));
    }
    let base = declaration();
    parse(&base).unwrap();
    // Required fields and unknown fields are enforced at every schema level.
    fn probe(v: &Value, path: &str, base: &Value) {
        match v {
            Value::Object(o) => {
                for key in o.keys() {
                    let mut bad = base.clone();
                    bad.pointer_mut(path)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(key);
                    // Map members are declarations, not mandatory struct fields.
                    if !["/roles", "/registers", "/contracts"].contains(&path) {
                        rejected(&bad);
                    }
                }
                if !["/roles", "/registers", "/contracts"].contains(&path) {
                    let mut bad = base.clone();
                    bad.pointer_mut(path).unwrap()["violated"] = json!(true);
                    rejected(&bad);
                }
                for (key, member) in o {
                    probe(member, &format!("{path}/{key}"), base);
                }
            }
            Value::Array(a) => {
                for (i, member) in a.iter().enumerate() {
                    probe(member, &format!("{path}/{i}"), base);
                }
            }
            _ => (),
        }
    }
    probe(&base, "", &base);
    for (path, bad_value) in [
        ("/schemaVersion", json!("author-property:v2")),
        ("/scope/kind", json!("eventually")),
        ("/guard", Value::Null),
        ("/assertion/op", json!("division")),
        ("/assertion/right/value", json!(0.0)),
        (
            "/assertion/right/value",
            json!("170141183460469231731687303715884105728"),
        ),
        ("/assertion/right/unit/kind", json!("height")),
        ("/registers/liability/valueType", json!("bytes")),
        ("/registers/liability/index", json!(3)),
        ("/registers/liability/index", json!(10)),
        ("/sources/0/sha256", json!("00")),
        ("/contracts/reserve/script", json!("xyz")),
        ("/roles/before/selector/index", json!(16)),
        ("/roles/before/selector/index", json!(-1)),
    ] {
        let mut bad = base.clone();
        *bad.pointer_mut(path).unwrap() = bad_value;
        rejected(&bad);
    }
    for op in [
        "divide",
        "multiply",
        "float",
        "price-feed",
        "recurse",
        "hash-invert",
        "avl",
        "execute",
        "alternative-trace",
        "eventually",
    ] {
        let mut bad = base.clone();
        bad["assertion"] = json!({"op":op});
        rejected(&bad);
    }
    for kind in ["count", "height", "scalar", "nano-erg"] {
        let mut v = base.clone();
        v["assertion"] = equality(
            integer("-170141183460469231731687303715884105728", kind),
            integer("170141183460469231731687303715884105727", kind),
        );
        parse(&v).unwrap();
        for other in ["count", "height", "scalar", "nano-erg"] {
            if kind == other {
                continue;
            }
            v["assertion"]["right"]["unit"]["kind"] = json!(other);
            rejected(&v);
        }
    }

    // Every supported connective/comparison/arithmetic branch is type checked.
    for op in ["eq", "lt", "le", "gt", "ge"] {
        let mut v = base.clone();
        v["assertion"] = json!({"op":op,"left":integer("1","count"),"right":integer("2","count")});
        parse(&v).unwrap();
        v["assertion"]["left"] = boolean();
        rejected(&v);
    }
    for op in ["add", "sub"] {
        let mut v = base.clone();
        v["assertion"] = equality(
            json!({"op":op,"left":integer("1","count"),"right":integer("2","count")}),
            integer("3", "count"),
        );
        parse(&v).unwrap();
        v["assertion"]["left"]["right"] = integer("2", "height");
        rejected(&v);
    }
    for op in ["and", "or"] {
        let mut v = base.clone();
        v["assertion"] = json!({"op":op,"args":[boolean(), {"op":"not","arg":boolean()}]});
        parse(&v).unwrap();
        v["assertion"]["args"][0] = integer("1", "count");
        rejected(&v);
    }
    let mut v = base.clone();
    v["assertion"] = equality(json!({"op":"height"}), integer("100", "height"));
    parse(&v).unwrap();
    v["assertion"]["left"]["accepted"] = json!(true);
    rejected(&v);
    let mut v = base.clone();
    v["assertion"] = equality(
        json!({"op":"sum-token","role":"after","token":"ab".repeat(32)}),
        json!({"op":"integer","value":"0","unit":{"kind":"token","id":"ab".repeat(32)}}),
    );
    parse(&v).unwrap();
    v["assertion"]["right"]["unit"]["id"] = json!("cd".repeat(32));
    rejected(&v);
    let mut v = base.clone();
    v["scope"] = json!({"kind":"bounded-response","horizon":1,"schedule":source()});
    probe(&v, "", &v);
    // Fixed scale preserves units; variable multiplication is not in the syntax.
    let mut v = base.clone();
    v["assertion"] = equality(
        json!({"op":"scale","factor":"100","arg":{"op":"register","name":"liability"}}),
        integer("100", "nano-erg"),
    );
    parse(&v).unwrap();
    v["assertion"]["left"]["factor"] = integer("100", "scalar");
    rejected(&v);
    for horizon in [1, 2, 3, 4] {
        let mut v = base.clone();
        v["scope"] = json!({"kind":"bounded-response","horizon":horizon,"schedule":source()});
        parse(&v).unwrap();
        v["scope"]["alternativeTrace"] = json!(source());
        rejected(&v);
    }
    for horizon in [0, 5] {
        let mut v = base.clone();
        v["scope"] = json!({"kind":"bounded-response","horizon":horizon,"schedule":source()});
        rejected(&v);
    }
    // Both guard and assertion contribute to the SAME 32-node ceiling.
    let mut v = base.clone();
    v["assertion"] = json!({"op":"and","args":vec![boolean();30]});
    parse(&v).unwrap();
    v["assertion"]["args"]
        .as_array_mut()
        .unwrap()
        .push(boolean());
    assert!(matches!(parse(&v), Err(SchemaError::Capped(_))));
    let mut v = base.clone();
    for i in 0..6 {
        v["roles"][format!("extra{i}")] = base["roles"]["before"].clone();
    }
    parse(&v).unwrap();
    v["roles"]["ninth"] = base["roles"]["before"].clone();
    assert!(matches!(parse(&v), Err(SchemaError::Capped(_))));
    check_collection_limits(16, [8; 16]).unwrap();
    assert!(matches!(
        check_collection_limits(17, [8; 17]),
        Err(SchemaError::Capped(_))
    ));
    assert!(matches!(
        check_collection_limits(1, [9]),
        Err(SchemaError::Capped(_))
    ));
    assert!(check_collection_limits(2, [8]).is_err());
    assert!(check_collection_limits(0, [8]).is_err());
    check_collection_limits(0, []).unwrap();
    check_trace_limit(8).unwrap();
    assert!(check_trace_limit(9).is_err());
    assert!(check_trace_limit(0).is_err());
    // Inputs round-trip, but status/acceptance/evidence can never be imported.
    let d = parse(&base).unwrap();
    let input: DeclarationInput = serde_json::from_slice(d.canonical_json()).unwrap();
    assert_eq!(
        Declaration::parse(&serde_json::to_string(&input).unwrap())
            .unwrap()
            .digest(),
        d.digest()
    );
    println!("strict v1 inputs; 32 nodes/8 roles/16 boxes/8 tokens/8 transactions; horizon 1..4; no execution or verdict authority");
}

fn wire(index: u16, tokens: Vec<TokenSpec>) -> WireBox {
    WireBox::hypothetical(
        &CandidateSpec {
            value: 10_000_000,
            ergo_tree: "10010100d17300".into(),
            creation_height: 100,
            tokens,
            registers: "00".into(),
        },
        &CreationReference {
            transaction_id: "01".repeat(32),
            index,
        },
    )
    .unwrap()
}
#[test]
fn bindings_never_infer_missing_roles_or_authority() {
    let base = declaration();
    let d = parse(&base).unwrap();
    let boxes = vec![wire(0, vec![]), wire(1, vec![])];
    let bound = d.bind_roles(&boxes, &[], &[]).unwrap();
    assert_eq!(bound["before"], vec![0]);
    assert!(bound["after"].is_empty());
    assert!(matches!(
        d.bind_roles(&[], &boxes, &[]),
        Err(SchemaError::Unresolved(_))
    ));
    let mut v = base.clone();
    v["roles"]["before"]["selector"] = json!({"kind":"script","hex":"10010100d17300"});
    let d = parse(&v).unwrap();
    assert!(matches!(
        d.bind_roles(&boxes, &[], &[]),
        Err(SchemaError::Unresolved(_))
    ));
    assert_eq!(
        d.bind_roles(&boxes[..1], &[], &[]).unwrap()["before"],
        vec![0]
    );
    // Fungible token membership provides no uniqueness and no named signer.
    let token = "02".repeat(32);
    let tokens = vec![TokenSpec {
        id: token.clone(),
        amount: 1,
    }];
    let token_boxes = vec![wire(0, tokens.clone()), wire(1, tokens)];
    v["roles"]["before"]["selector"] = json!({"kind":"token-id","hex":token});
    let d = parse(&v).unwrap();
    assert!(matches!(
        d.bind_roles(&token_boxes, &[], &[]),
        Err(SchemaError::Unresolved(_))
    ));
    assert!(matches!(
        d.bind_roles(&boxes, &[], &[]),
        Err(SchemaError::Unresolved(_))
    ));
    assert_eq!(
        d.bind_roles(&token_boxes[..1], &[], &[]).unwrap()["before"],
        vec![0]
    );
    v["registers"] = json!({});
    v["roles"]["before"]["cardinality"] = json!("all-matches");
    assert_eq!(
        parse(&v)
            .unwrap()
            .bind_roles(&token_boxes, &[], &[])
            .unwrap()["before"],
        vec![0, 1]
    );
    v["roles"]["before"]["collection"] = json!("data-inputs");
    assert!(parse(&v)
        .unwrap()
        .bind_roles(&token_boxes, &[], &[])
        .unwrap()["before"]
        .is_empty());
    assert_eq!(
        parse(&v)
            .unwrap()
            .bind_roles(&[], &[], &token_boxes)
            .unwrap()["before"],
        vec![0, 1]
    );
    v["roles"]["before"]["selector"] =
        json!({"kind":"box-id","hex":hex::encode(boxes[1].node().box_id().unwrap().as_bytes())});
    v["roles"]["before"]["collection"] = json!("inputs");
    assert_eq!(
        parse(&v).unwrap().bind_roles(&boxes, &[], &[]).unwrap()["before"],
        vec![1]
    );
    v["roles"]["before"]["selector"] = json!({"kind":"signer","name":"Alice"});
    rejected(&v);
    for selector in [
        json!({"kind":"first-match"}),
        json!({"kind":"successor-of","role":"before"}),
        json!({"kind":"token-id","hex":"00"}),
    ] {
        v["roles"]["before"]["selector"] = selector;
        rejected(&v);
    }
    let mut v = base.clone();
    v["roles"].as_object_mut().unwrap().remove("before");
    rejected(&v);
    let mut v = base.clone();
    v["roles"]["before"]["cardinality"] = json!("all-matches");
    rejected(&v);
    let mut v = base.clone();
    v["registers"]["alias"] = v["registers"]["liability"].clone();
    v["registers"]["alias"]["unit"] = json!({"kind":"height"});
    rejected(&v);
    let mut v = base.clone();
    v["assertion"] = json!({"op":"bytes-eq","left":{"op":"read-bytes","role":"after","field":"script"},"right":{"op":"bytes","hex":"10010100d17300"}});
    rejected(&v);
    v["roles"]["after"]["cardinality"] = json!("exactly-one");
    parse(&v).unwrap();
    v["assertion"] = equality(json!({"op":"count","role":"after"}), integer("0", "count"));
    parse(&v).unwrap();
    // Duplicate keys must fail before any last-value-wins interpretation.
    let raw = base.to_string();
    let duplicate_role = format!("\"roles\":{{\"before\":{},", base["roles"]["before"]);
    assert!(Declaration::parse(&raw.replacen("\"roles\":{", &duplicate_role, 1)).is_err());
    let duplicate_contract = format!(
        "\"contracts\":{{\"reserve\":{},",
        base["contracts"]["reserve"]
    );
    assert!(Declaration::parse(&raw.replacen("\"contracts\":{", &duplicate_contract, 1)).is_err());

    assert!(
        Declaration::parse(&raw.replacen("\"roles\":{", "\"roles\":{\"before\":{},", 1)).is_err()
    );
    assert!(Declaration::parse(&raw.replacen(
        "\"propertyId\":",
        "\"propertyId\":\"forged\",\"propertyId\":",
        1
    ))
    .is_err());
    let many: Vec<_> = (0..17).map(|i| wire(i, vec![])).collect();
    assert!(matches!(
        parse(&base).unwrap().bind_roles(&many, &[], &[]),
        Err(SchemaError::Capped(_))
    ));
    let too_many_tokens = vec![wire(
        0,
        (0..9)
            .map(|i| TokenSpec {
                id: format!("{i:064x}"),
                amount: 1,
            })
            .collect(),
    )];
    assert!(matches!(
        parse(&base).unwrap().bind_roles(&too_many_tokens, &[], &[]),
        Err(SchemaError::Capped(_))
    ));
    println!("missing/ambiguous roles unresolved; exact collections/IDs/scripts/tokens; empty all-matches retained; no inferred signer or successor");
}

#[test]
fn declaration_identity_binds_all_semantic_premises() {
    let base = declaration();
    let d = parse(&base).unwrap();
    assert_eq!(
        d.digest(),
        Declaration::parse(&serde_json::to_string_pretty(&base).unwrap())
            .unwrap()
            .digest()
    );
    let mut normalized = base.clone();
    normalized["contracts"]["reserve"]["script"] = json!("10010100D17300");
    normalized["roles"]["after"]["selector"]["hex"] = json!("10010100D17300");
    normalized["assertion"]["right"]["value"] = json!("+000");
    normalized["sources"][0]["sha256"] = json!("AB".repeat(32));
    assert_eq!(d.digest(), parse(&normalized).unwrap().digest());
    for (path, change) in [
        ("/propertyId", json!("other")),
        ("/revision", json!("2")),
        ("/contracts/reserve/script", json!("10010000d17300")),
        ("/contracts/reserve/compilerRevision", json!("other")),
        (
            "/scope",
            json!({"kind":"bounded-response","horizon":2,"schedule":source()}),
        ),
        ("/roles/before/collection", json!("data-inputs")),
        ("/roles/before/selector/index", json!(1)),
        ("/roles/after/cardinality", json!("exactly-one")),
        ("/registers/liability/index", json!(5)),
        ("/registers/liability/valueType", json!("int")),
        (
            "/registers/liability/unit",
            json!({"kind":"named-amount","name":"liability-units"}),
        ),
        ("/guard/value", json!(false)),
        ("/assertion/op", json!("gt")),
        ("/assertion/right/value", json!("1")),
        ("/authorizationPremises/0/id", json!("other")),
        (
            "/authorizationPremises/0/statement",
            json!("All named test credentials are available"),
        ),
        (
            "/authorizationPremises/0/source/reference",
            json!("local:other"),
        ),
        (
            "/authorizationPremises/0/source/sha256",
            json!("bc".repeat(32)),
        ),
        ("/sources/0/origin", json!("source-recorded")),
        ("/sources/0/reference", json!("local:other")),
        ("/sources/0/sha256", json!("cd".repeat(32))),
    ] {
        let mut changed = base.clone();
        *changed.pointer_mut(path).unwrap() = change;
        assert_ne!(
            d.digest(),
            parse(&changed).unwrap().digest(),
            "unbound {path}"
        );
    }
    let mut response = base.clone();
    response["scope"] = json!({"kind":"bounded-response","horizon":2,"schedule":source()});
    let before = parse(&response).unwrap();
    response["scope"]["horizon"] = json!(3);
    assert_ne!(before.digest(), parse(&response).unwrap().digest());
    response["scope"]["horizon"] = json!(2);
    response["scope"]["schedule"]["sha256"] = json!("de".repeat(32));
    assert_ne!(before.digest(), parse(&response).unwrap().digest());
    // A caller may edit a cloned input, but cannot mutate the checked identity.
    let mut clone = d.input().clone();
    clone.property_id = "changed".into();
    assert_eq!(d.input().property_id, "reserve-change");
    assert_ne!(
        d.digest(),
        Declaration::parse(&serde_json::to_string(&clone).unwrap())
            .unwrap()
            .digest()
    );
    for field in [
        "violated",
        "accepted",
        "safe",
        "proved",
        "digest",
        "reachability",
        "contractDefect",
    ] {
        let mut forged = base.clone();
        forged[field] = json!(true);
        rejected(&forged);
    }
    println!("normalized immutable SHA-256 binds every declaration premise; input round-trip never imports status; model assertions confer no contract-defect authority");
}
