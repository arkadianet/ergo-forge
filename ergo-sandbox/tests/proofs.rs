//! Proof tooling: spending proofs from secrets (the node's wallet prover)
//! and AVL+ tree digests/proofs from a real prover, both driven from the
//! scenario JSON and checked through the consensus verification path.

use ergo_sandbox::testsuite::{self, Suite};

/// Run one case through the suite runner; return (actual verdict, error).
fn run(
    source: &str,
    params: serde_json::Value,
    mut case: serde_json::Value,
) -> (String, Option<String>) {
    case["name"] = serde_json::json!("case");
    if case.get("expect").is_none() {
        case["expect"] = serde_json::json!("pass");
    }
    let suite: Suite = serde_json::from_value(
        serde_json::json!({ "source": source, "params": params, "scenarios": [case] }),
    )
    .expect("suite parses");
    let r = testsuite::run(&suite).expect("runs");
    let c = &r.cases[0];
    (c.actual.to_string(), c.error.clone())
}

const X1: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const X2: &str = "0000000000000000000000000000000000000000000000000000000000000002";
const X3: &str = "0000000000000000000000000000000000000000000000000000000000000003";

#[test]
fn two_of_three_keys_prove_and_verify_from_secrets() {
    let pk = |x: &str| ergo_sandbox::prove::pubkey_hex(x).unwrap();
    let (a, b, c) = (pk(X1), pk(X2), pk(X3));
    let src = "atLeast(2, Coll($a, $b, $c))";
    let params = serde_json::json!({ "a": {"type":"SigmaProp","value":a}, "b": {"type":"SigmaProp","value":b}, "c": {"type":"SigmaProp","value":c} });
    let ok = run(
        src,
        params.clone(),
        serde_json::json!({ "height": 1, "secrets": [ {"dlog": X1}, {"dlog": X3} ] }),
    );
    assert_eq!(ok.0, "proofAccepted", "{:?}", ok.1);
    let short = run(
        src,
        params,
        serde_json::json!({ "height": 1, "secrets": [ {"dlog": X1} ] }),
    );
    assert_eq!(short.0, "needsProof", "{:?}", short.1);
    assert!(
        short.1.as_deref().unwrap_or("").contains("no proof"),
        "{:?}",
        short.1
    );
}

#[test]
fn a_diffie_hellman_tuple_is_proven_with_the_shared_secret() {
    let g = ergo_sandbox::prove::generator_hex();
    let h = ergo_sandbox::prove::pubkey_hex(X2).unwrap(); // any second base
    let (u, v) = ergo_sandbox::prove::dht_hex(&g, &h, X3).unwrap();
    let src = "proveDHTuple($g, $h, $u, $v)";
    let ge = |p: &str| serde_json::json!({"type":"GroupElement","value":p});
    let params = serde_json::json!({ "g": ge(&g), "h": ge(&h), "u": ge(&u), "v": ge(&v) });
    let ok = run(
        src,
        params.clone(),
        serde_json::json!({ "height": 1, "secrets": [ {"dht": {"g": g, "h": h, "x": X3}} ] }),
    );
    assert_eq!(ok.0, "proofAccepted", "{:?}", ok.1);
    let wrong = run(
        src,
        params,
        serde_json::json!({ "height": 1, "secrets": [ {"dht": {"g": g, "h": h, "x": X1}} ] }),
    );
    assert_eq!(wrong.0, "needsProof", "{:?}", wrong.1);
}

#[test]
fn an_avl_insert_proof_from_the_prover_satisfies_a_registry_script() {
    // The box's R4 holds the tree; the spender supplies the proof (var 0)
    // and the new key/value (vars 1, 2); the successor must carry the
    // digest the prover computed.
    let src = r#"{
      val tree = SELF.R4[AvlTree].get
      val proof = getVar[Coll[Byte]](0).get
      val key = getVar[Coll[Byte]](1).get
      val value = getVar[Coll[Byte]](2).get
      val next = tree.insert(Coll((key, value)), proof)
      sigmaProp(next.isDefined && OUTPUTS(0).R4[AvlTree].get.digest == next.get.digest)
    }"#;
    let key = "11".repeat(32);
    let none = serde_json::json!({});
    let scenario = |avl_ops: serde_json::Value| {
        serde_json::json!({
            "height": 1,
            "avl": { "names": { "keyLength": 32, "entries": [["22".repeat(32), "aa"], ["33".repeat(32), "bb"]], "operations": avl_ops } },
            "selfBox": { "value": 1, "registers": { "R4": { "type": "AvlTree", "value": "@avl.names" } } },
            "outputs": [ { "value": 1, "ergoTree": "$self", "registers": { "R4": { "type": "AvlTree", "value": "@avl.names.after" } } } ],
            "contextVars": { "0": { "type": "Coll[Byte]", "value": "@avl.names.proof" }, "1": { "type": "Coll[Byte]", "value": key }, "2": { "type": "Coll[Byte]", "value": "cc" } }
        })
    };
    let ok = run(
        src,
        none.clone(),
        scenario(serde_json::json!([ { "insert": { "key": key, "value": "cc" } } ])),
    );
    assert_eq!(ok.0, "pass", "{:?}", ok.1);
    // A proof for a different operation does not authenticate this insert.
    let other = run(
        src,
        none.clone(),
        scenario(serde_json::json!([ { "lookup": { "key": "22".repeat(32) } } ])),
    );
    assert_ne!(other.0, "pass", "{:?}", other.1);
    // Inserting a key that already exists fails the insert.
    let dup = run(
        src,
        none,
        serde_json::json!({
            "height": 1,
            "avl": { "names": { "keyLength": 32, "entries": [[key, "aa"]], "operations": [ { "lookup": { "key": key } } ] } },
            "selfBox": { "value": 1, "registers": { "R4": { "type": "AvlTree", "value": "@avl.names" } } },
            "outputs": [ { "value": 1, "ergoTree": "$self", "registers": { "R4": { "type": "AvlTree", "value": "@avl.names" } } } ],
            "contextVars": { "0": { "type": "Coll[Byte]", "value": "@avl.names.proof" }, "1": { "type": "Coll[Byte]", "value": key }, "2": { "type": "Coll[Byte]", "value": "cc" } }
        }),
    );
    assert_ne!(dup.0, "pass", "{:?}", dup.1);
}

#[test]
fn an_avl_lookup_proof_answers_a_membership_script() {
    let src = r#"{
      val tree = SELF.R4[AvlTree].get
      val proof = getVar[Coll[Byte]](0).get
      sigmaProp(tree.get(getVar[Coll[Byte]](1).get, proof).isDefined)
    }"#;
    let key = "22".repeat(32);
    let sc = |lookup: &str| {
        serde_json::json!({
            "height": 1,
            "avl": { "t": { "keyLength": 32, "entries": [[key, "aa"]], "operations": [ { "lookup": { "key": lookup } } ] } },
            "selfBox": { "value": 1, "registers": { "R4": { "type": "AvlTree", "value": "@avl.t" } } },
            "contextVars": { "0": { "type": "Coll[Byte]", "value": "@avl.t.proof" }, "1": { "type": "Coll[Byte]", "value": lookup } }
        })
    };
    assert_eq!(run(src, serde_json::json!({}), sc(&key)).0, "pass");
    assert_eq!(
        run(src, serde_json::json!({}), sc(&"44".repeat(32))).0,
        "fail"
    );
}

#[test]
fn an_avl_operation_the_tree_refuses_is_reported_not_faked() {
    // Inserting an existing key: the prover itself refuses, and the
    // scenario says so instead of producing a proof of nothing.
    let sc: ergo_sandbox::Scenario = serde_json::from_value(serde_json::json!({
        "source": "sigmaProp(true)", "height": 1,
        "avl": { "t": { "keyLength": 32, "entries": [["22".repeat(32), "aa"]], "operations": [ { "insert": { "key": "22".repeat(32), "value": "cc" } } ] } }
    })).unwrap();
    let err = ergo_sandbox::eval_scenario(&sc).unwrap_err().to_string();
    assert!(err.contains("avl operation"), "{err}");
}

#[test]
fn a_scenario_can_carry_the_last_headers() {
    // CONTEXT.headers: the newest first; a script may read heights,
    // timestamps, miner keys, votes. LastBlockUtxoRootHash comes from
    // headers(0).stateRoot.
    let src = r#"{
      val h = CONTEXT.headers
      // `h.size` is left out on purpose: the pinned node evaluator has no
      // SizeOf arm for Coll[Header] (fixed upstream; re-enable on the bump).
      sigmaProp(h(0).height == HEIGHT - 1 && h(1).height == HEIGHT - 2 &&
                h(0).timestamp > h(1).timestamp && h(0).votes == Coll(1.toByte, 0.toByte, 0.toByte) &&
                h(0).minerPk == decodePoint(getVar[Coll[Byte]](0).get) &&
                CONTEXT.LastBlockUtxoRootHash.digest == h(0).stateRoot.digest)
    }"#;
    let g = ergo_sandbox::prove::generator_hex();
    let sc = serde_json::json!({
        "height": 1000,
        "headers": [
            { "height": 999, "timestamp": 1_700_000_120_000u64, "votes": [1, 0, 0], "minerPk": g, "stateRoot": "aa".repeat(32) + "07" },
            { "height": 998, "timestamp": 1_700_000_000_000u64 }
        ],
        "contextVars": { "0": { "type": "Coll[Byte]", "value": g } }
    });
    let r = run(src, serde_json::json!({}), sc);
    assert_eq!(r.0, "pass", "{:?}", r.1);
}

#[test]
fn two_parties_sign_a_two_of_three_without_sharing_secrets() {
    // The standard Ergo multi-party flow: each party generates commitments
    // for its own key, the first proves with the others' commitments, the
    // next completes from the extracted hints. No registry ever holds two
    // parties' secrets.
    let pk = |x: &str| ergo_sandbox::prove::pubkey_hex(x).unwrap();
    let (a, b, c) = (pk(X1), pk(X2), pk(X3));
    let src = "atLeast(2, Coll($a, $b, $c))";
    let params = serde_json::json!({ "a": {"type":"SigmaProp","value":a}, "b": {"type":"SigmaProp","value":b}, "c": {"type":"SigmaProp","value":c} });
    let ok = run(
        src,
        params.clone(),
        serde_json::json!({ "height": 1, "parties": [ { "name": "alice", "secrets": [ {"dlog": X1} ] }, { "name": "carol", "secrets": [ {"dlog": X3} ] } ] }),
    );
    assert_eq!(ok.0, "proofAccepted", "{:?}", ok.1);
    let one = run(
        src,
        params,
        serde_json::json!({ "height": 1, "parties": [ { "secrets": [ {"dlog": X1} ] } ] }),
    );
    assert_eq!(one.0, "needsProof", "{:?}", one.1);
    assert!(
        one.1.as_deref().unwrap_or("").contains("no proof"),
        "{:?}",
        one.1
    );
}

#[test]
fn three_parties_sign_an_and_of_three_in_sequence() {
    let pk = |x: &str| ergo_sandbox::prove::pubkey_hex(x).unwrap();
    let (a, b, c) = (pk(X1), pk(X2), pk(X3));
    let src = "$a && $b && $c";
    let params = serde_json::json!({ "a": {"type":"SigmaProp","value":a}, "b": {"type":"SigmaProp","value":b}, "c": {"type":"SigmaProp","value":c} });
    let ok = run(
        src,
        params,
        serde_json::json!({ "height": 1, "parties": [ { "secrets": [ {"dlog": X1} ] }, { "secrets": [ {"dlog": X2} ] }, { "secrets": [ {"dlog": X3} ] } ] }),
    );
    assert_eq!(ok.0, "proofAccepted", "{:?}", ok.1);
}

#[test]
fn headers_size_is_answered_after_the_pin_bump() {
    let r = run(
        "sigmaProp(CONTEXT.headers.size == 2)",
        serde_json::json!({}),
        serde_json::json!({ "height": 5, "headers": [ {"height": 4}, {"height": 3} ] }),
    );
    assert_eq!(r.0, "pass", "{:?}", r.1);
}
