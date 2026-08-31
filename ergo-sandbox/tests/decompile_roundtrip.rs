//! Decompiler tests — the P2 verification bar.
//!
//! The bar is `decompile → recompile → byte-identical`. It runs here over a
//! COMMITTED fixture (so CI in this repo needs no ergo node checkout), plus
//! optional whole-corpus checks that run when the node checkout is present as
//! a sibling.
//!
//! Section dividers per repo convention: helpers / round-trip / shapes /
//! network / known-upstream / corpora.

use ergo_sandbox::{compile_source, decompile};
use ergo_ser::address::NetworkPrefix;

// ----- helpers -----

struct Vector {
    tree: Vec<u8>,
    tree_hex: String,
    source: String,
    tree_version: u8,
}

fn fixture() -> Vec<Vector> {
    let raw = include_str!("fixtures/compile_corpus_subset.json");
    let parsed: Vec<serde_json::Value> = serde_json::from_str(raw).expect("fixture JSON");
    parsed
        .into_iter()
        .map(|v| {
            let hex_str = v["tree"].as_str().expect("tree").to_string();
            Vector {
                tree: hex::decode(&hex_str).expect("fixture hex"),
                tree_hex: hex_str,
                source: v["source"].as_str().unwrap_or_default().to_string(),
                tree_version: v["treeVersion"].as_u64().unwrap_or(3) as u8,
            }
        })
        .collect()
}

/// Decompile with a stack large enough for deep trees — the test harness
/// gives each test a 2 MiB thread stack, and deeply nested contracts need
/// about 3 MiB (see `decompile::LARGE_STACK_BYTES`).
fn decompile_report_net(bytes: &[u8], testnet: bool) -> decompile::Decompiled {
    let owned = bytes.to_vec();
    decompile::with_large_stack(move || {
        decompile::decompile_report(&owned, testnet).expect("decompile")
    })
}

fn decompile_net(bytes: &[u8], testnet: bool) -> String {
    decompile_report_net(bytes, testnet).source
}

fn recompile(src: &str, tv: u8, net: NetworkPrefix) -> Result<Vec<u8>, String> {
    // The compiler's parse recursion is unbounded upstream, so deeply nested
    // source needs the same stack headroom the decompiler needs.
    let owned = src.to_string();
    decompile::with_large_stack(move || {
        compile_source(&owned, tv, net)
            .map(|o| o.tree_bytes)
            .map_err(|e| e.to_string())
    })
}

fn fixture_by_source(needle: &str) -> Vector {
    fixture()
        .into_iter()
        .find(|v| v.source == needle)
        .unwrap_or_else(|| panic!("fixture is missing the vector for {needle:?}"))
}

// ----- round-trip -----

#[test]
fn fixture_trees_decompile_and_recompile_byte_identically() {
    // The load-bearing regression net: every committed fixture vector must
    // survive decompile → recompile unchanged. A lift or pretty-print
    // regression that changes the emitted bytes fails here.
    let vectors = fixture();
    assert!(!vectors.is_empty(), "fixture must not be empty");
    let mut failures: Vec<String> = Vec::new();
    for v in &vectors {
        // A raw placeholder means "not liftable yet" — a fixture vector must
        // be fully liftable, otherwise the fixture has silently regressed.
        // Counted structurally (decompile_report), never by re-scanning text.
        let report = decompile_report_net(&v.tree, true);
        if report.raw_placeholders > 0 || report.truncated {
            failures.push(format!(
                "{}: {} raw placeholder(s) in `{}`",
                v.tree_hex, report.raw_placeholders, report.source
            ));
            continue;
        }
        let src = report.source;
        match recompile(&src, v.tree_version, NetworkPrefix::Testnet) {
            Ok(bytes) if bytes == v.tree => {}
            Ok(bytes) => failures.push(format!(
                "{}: byte mismatch\n  want {}\n  got  {}",
                v.tree_hex,
                v.tree_hex,
                hex::encode(bytes)
            )),
            Err(e) => failures.push(format!("{}: recompile failed: {}", v.tree_hex, e)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} fixture vectors failed:\n{}",
        failures.len(),
        vectors.len(),
        failures.join("\n")
    );
}

// ----- shapes -----

#[test]
fn fixture_shapes_are_pinned() {
    // Exact text pins for representative lifts. These catch pretty-print
    // drift (spacing, precedence, naming) that the byte comparison can't see,
    // because a re-render may still recompile to the same bytes.
    let pins: &[(&str, &str)] = &[
        ("sigmaProp(HEIGHT > 100)", "HEIGHT > 100"),
        ("HEIGHT>5 && HEIGHT<9", "HEIGHT > 5 && HEIGHT < 9"),
        (
            "allOf(Coll(HEIGHT > 5, HEIGHT < 9))",
            "allOf(Coll(HEIGHT > 5, HEIGHT < 9))",
        ),
        ("1 == 2", "1 == 2"),
        // `!true` was constant-folded by the reference compiler: the wire
        // holds `false`, so the honest decompilation is `false`.
        ("!true", "false"),
        (
            "PK(\"3WwXpssaZwcNzaGMv3AgxBdTPJQBt5gCmqBsg3DykQ39bYdhJBsN\")",
            "PK(\"3WwXpssaZwcNzaGMv3AgxBdTPJQBt5gCmqBsg3DykQ39bYdhJBsN\")",
        ),
    ];
    for (source, expected) in pins {
        let v = fixture_by_source(source);
        assert_eq!(
            decompile_net(&v.tree, true),
            *expected,
            "for source {source:?}"
        );
    }
}

#[test]
fn fold_wire_tuple_lambda_is_unwrapped_to_a_two_arg_lambda() {
    // The wire stores fold's (acc, elem) as ONE tuple-typed lambda arg; source
    // must be the 2-arg form or recompilation produces a different tree.
    let v = fixture_by_source("sigmaProp(Coll(1, 2).fold(0, {(a: Int, b: Int) => a + b}) == 3)");
    let out = decompile_net(&v.tree, true);
    assert!(
        !out.contains("(Int, Int)"),
        "fold lambda must not keep the wire tuple-arg form: {out}"
    );
    assert!(
        out.contains("{ (t: Int, t2: Int)"),
        "fold lambda must render as a typed 2-arg lambda: {out}"
    );
    // …and it must still round-trip.
    let bytes = recompile(&out, v.tree_version, NetworkPrefix::Testnet).expect("recompile");
    assert_eq!(bytes, v.tree);
}

#[test]
fn negation_of_a_constant_is_rendered_faithfully() {
    // Wire: BoolToSigmaProp(LT(Minus(Negation(2147483647), 2), 0)). The
    // decompiler's job is fidelity; see the known-upstream test below for why
    // this particular tree cannot currently be recompiled.
    let bytes = hex::decode("100304feffffffffffffffff0104040400d18f99f0730073017302").expect("hex");
    assert_eq!(decompile_net(&bytes, true), "-2147483647 - 2 < 0");
}

// ----- network -----

#[test]
fn decompile_is_network_aware_for_pk_constants() {
    // A bare P2PK tree (mainnet shape from the diff corpus). PK addresses
    // must be encoded for the network the source will be compiled against,
    // or recompilation rejects the address as a network mismatch.
    let bytes =
        hex::decode("0008cd034a53f17d249721c647c13477bb16982c8b2b16daa923d2a49dee8a88593c8356")
            .expect("hex");
    let mainnet_src = decompile_net(&bytes, false);
    let testnet_src = decompile_net(&bytes, true);
    assert_ne!(
        mainnet_src, testnet_src,
        "network must change the PK address"
    );
    assert!(mainnet_src.starts_with("PK(\""), "got {mainnet_src}");

    // Each rendering recompiles byte-identically under its own network.
    let m = recompile(&mainnet_src, 0, NetworkPrefix::Mainnet).expect("mainnet recompile");
    assert_eq!(m, bytes, "mainnet rendering must round-trip");
    let t = recompile(&testnet_src, 0, NetworkPrefix::Testnet).expect("testnet recompile");
    assert_eq!(t, bytes, "testnet rendering must round-trip");

    // …and the wrong network is rejected (that's why this matters).
    assert!(
        recompile(&testnet_src, 0, NetworkPrefix::Mainnet).is_err(),
        "a testnet PK must not compile under mainnet"
    );
}

// ----- known upstream -----

/// KNOWN ergo-compiler BUG (upstream), not a decompiler defect.
///
/// The reference compiler accepts `sigmaProp((-(0 + 2147483647) - 2) < 0)` and
/// emits `Minus(Negation(Const(2147483647)), Const(2))`. Our decompiler renders
/// that faithfully as `-2147483647 - 2`, and the compiler then REJECTS it:
/// `constant fold overflows Int`. Binding the value to a `val` (so the operand
/// isn't a literal) compiles fine, which shows the failure is the literal
/// constant-fold, not the tree shape.
#[test]
#[ignore = "upstream ergo-compiler bug; un-ignore and invert when fixed"]
fn known_upstream_bug_constant_fold_overflow_rejects_a_faithful_rendering() {
    let bytes = hex::decode("100304feffffffffffffffff0104040400d18f99f0730073017302").expect("hex");
    let src = decompile_net(&bytes, true);
    // Sanity: the same value through a val compiles today.
    let val_form = "{ val x = 2147483647; sigmaProp((-x - 2) < 0) }";
    assert!(
        recompile(val_form, 0, NetworkPrefix::Mainnet).is_ok(),
        "val form should compile"
    );
    // The faithful rendering currently does NOT — that is the upstream bug.
    assert!(
        recompile(&src, 0, NetworkPrefix::Mainnet).is_err(),
        "if this now compiles, the upstream bug is fixed: invert this test"
    );
}

// ----- corpora (optional: needs the ergo node checkout as a sibling) -----

fn node_checkout() -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ergo");
    if p.join("test-vectors/ergoscript/compile/compile_seed.json")
        .is_file()
    {
        Some(p)
    } else {
        None
    }
}

fn skip_why(what: &str) -> bool {
    println!(
        "skipping {what}: ergo node checkout not found as a sibling of this repo \
         (committed fixture tests still ran)"
    );
    true
}

#[test]
fn seed_corpus_holds_the_exact_floor_when_checkout_present() {
    // Whole-corpus floor. The committed fixture is the strict net; this keeps
    // the FULL oracle corpus (incl. env-dependent vectors the fixture drops)
    // from silently collapsing.
    let Some(root) = node_checkout() else {
        assert!(skip_why("seed corpus floor"));
        return;
    };
    let path = root.join("test-vectors/ergoscript/compile/compile_seed.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    let mut exact = 0usize;
    let mut total = 0usize;
    for v in doc["vectors"].as_array().expect("vectors") {
        if v["oracle"].as_str() != Some("ACCEPT") || v["tree_version"].as_u64() != Some(3) {
            continue;
        }
        let (Some(hex_str), Some(source)) = (v["tree_hex"].as_str(), v["source"].as_str()) else {
            continue;
        };
        // Oracle demo-env vectors can't compile with the sandbox's empty env.
        if recompile(source, 3, NetworkPrefix::Testnet).is_err() {
            continue;
        }
        total += 1;
        let bytes = hex::decode(hex_str).expect("hex");
        let src = decompile_net(&bytes, true);
        if let Ok(out) = recompile(&src, 3, NetworkPrefix::Testnet) {
            if out == bytes {
                exact += 1;
            }
        }
    }
    assert_eq!(
        total, 87,
        "compile corpus size changed — re-check the floor"
    );
    assert!(
        exact >= 68,
        "seed corpus dropped to {exact}/87 byte-exact (floor is 68)"
    );
}

#[test]
fn mainnet_corpus_holds_the_exact_floor_when_checkout_present() {
    let Some(root) = node_checkout() else {
        assert!(skip_why("mainnet corpus floor"));
        return;
    };
    let path = root.join("test-vectors/mainnet/scala_tx_json/diff_corpus.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    let mut trees: Vec<String> = Vec::new();
    for tx in doc.as_array().expect("array") {
        for out in tx["scalaJson"]["outputs"].as_array().unwrap_or(&vec![]) {
            if let Some(t) = out["ergoTree"].as_str() {
                if !trees.iter().any(|t0| t0 == t) {
                    trees.push(t.to_string());
                }
            }
        }
    }
    let mut exact = 0usize;
    for t in &trees {
        let bytes = hex::decode(t).expect("hex");
        let src = decompile_net(&bytes, false);
        if let Ok(out) = recompile(&src, 0, NetworkPrefix::Mainnet) {
            if out == bytes {
                exact += 1;
            }
        }
    }
    assert!(
        exact >= 250,
        "mainnet corpus dropped to {exact}/{} byte-exact (floor is 250)",
        trees.len()
    );
}
