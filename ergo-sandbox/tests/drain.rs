//! Acceptance tests for the phase-1 drain hunt
//! (`docs/superpowers/specs/2026-09-08-drain-hunt-phase1-design.md`).
//!
//! The positive control is the USE LP drain: the honest builder shape — the
//! pool rebuilt with its full reserves, the swap preserved, a free trader
//! payout, the fee — must be left alone by the declared boxes, and the hunt
//! must *discover* the drain (decoy at `INPUTS(0)`, pool at `INPUTS(2)`,
//! drained successor, attacker payout) using only the generic decoy family.
//! The negative controls are the incident corpus's fixed swap and the
//! gallery's NFT-bound AMM pair: correct contracts accept the honest shape
//! and reject every drain probe.

use ergo_sandbox::drain::{drain_hunt, DrainRequest, DrainVerdict};
use ergo_sandbox::eval::eval_scenario;
use ergo_sandbox::txcheck::check as tx_check;
use ergo_sandbox::Verdict;
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};

const SWAP_SUITE: &str =
    include_str!("../../examples/incidents/use-lp-drain.deployed-swap.test.json");

fn with_key(key: &str, value: &str, box_obj: Value) -> Value {
    let mut m = box_obj.as_object().expect("box object").clone();
    m.insert(key.to_string(), json!(value));
    Value::Object(m)
}

/// The honest builder shape for the USE LP pair, declared from the incident
/// fixtures: the drain's boxes, but with the pool successor rebuilt with its
/// full reserves and the payout slot empty — what a legitimate fill of the
/// standing swap order would look like.
fn use_lp_request(swap_tree_override: Option<String>) -> Value {
    let fx: Value = serde_json::from_str(SWAP_SUITE).expect("fixture parses");
    let sc = &fx["scenarios"][0];
    let pool = sc["inputs"][2].clone();
    let swap = sc["inputs"][1].clone();
    let trader_tree = sc["inputs"][0]["ergoTree"].clone();
    let lp_nft = pool["tokens"][0]["id"].clone();

    let mut successor = pool.clone();
    successor["creationHeight"] = json!(1868202);
    let mut swap_box = swap.clone();
    if let Some(tree) = &swap_tree_override {
        swap_box["ergoTree"] = json!(tree);
    }

    let inputs = vec![
        with_key("role", "protected", pool.clone()),
        with_key("role", "companion", swap_box),
        json!({ "role": "attacker", "value": 2000000, "ergoTree": trader_tree }),
    ];
    let outputs = vec![
        with_key("payee", "fixed", successor),
        with_key("payee", "fixed", object(sc["outputs"][1].clone())),
        json!({ "payee": "free", "value": 0, "ergoTree": trader_tree, "tokens": [] }),
        with_key("payee", "fixed", object(sc["outputs"][3].clone())),
    ];
    json!({
        "inputs": inputs,
        "outputs": outputs,
        "protocolNfts": [lp_nft],
        "height": 1868204,
        "network": "mainnet",
    })
}

fn object(v: Value) -> Value {
    v
}

fn drain(request: Value) -> ergo_sandbox::drain::DrainReport {
    let req: DrainRequest = serde_json::from_value(request).expect("request parses");
    drain_request(req)
}

fn drain_request(req: DrainRequest) -> ergo_sandbox::drain::DrainReport {
    ergo_sandbox::decompile::with_large_stack(move || drain_hunt(&req)).expect("hunt runs")
}

#[test]
fn the_use_lp_drain_is_rediscovered_from_the_honest_template() {
    let report = drain(use_lp_request(None));
    assert!(
        !matches!(report.verdict, DrainVerdict::InvalidShape),
        "shape errors: {:?}",
        report.notes
    );
    assert_eq!(report.verdict, DrainVerdict::Drainable);
    let best = report.best.as_ref().expect("a best hit");

    // The extraction is the incident's: the whole reserve minus the drained
    // successor's keep-value, per asset.
    assert_eq!(
        best.extracted.get("nanoErg").map(String::as_str),
        Some("284695583089453")
    );
    let use_id = "a55b8735ed1a99e46c2c89f8994aacdf4b1109bdcf682f1e5b34479c6e392669";
    let lp_token_id = "804a66426283b8281240df8f9de783651986f20ad6391a71b26b9e7d6faad099";
    assert_eq!(
        best.extracted.get(use_id).map(String::as_str),
        Some("74519918")
    );
    assert_eq!(
        best.extracted.get(lp_token_id).map(String::as_str),
        Some("9223371891826792209")
    );

    // The winning shape is the incident's: decoy at INPUTS(0), pool at
    // INPUTS(2), drain-mode payout — and the winning decoy came from the
    // generic family (filler tokens), never hand-fed.
    assert_eq!(best.permutation, vec![2, 1, 0]);
    assert_eq!(best.payout, "drain");
    assert_eq!(best.decoys, vec!["input 2: filler tokens 0..=2"]);

    // Witness portability: the bundle's TxRequest validates, and every
    // protocol scenario re-evaluates to `pass`.
    let check = tx_check(&best.witness.tx_request).expect("txrequest checks");
    assert!(check.valid, "witness tx problems: {:?}", check.problems);
    for ps in &best.witness.protocol_scenarios {
        let outcome = eval_scenario(&ps.scenario).expect("scenario evaluates");
        assert_eq!(
            outcome.verdict,
            Verdict::Pass,
            "protocol input {} must pass",
            ps.index
        );
    }
}

#[test]
fn the_fixed_swap_accepts_the_honest_shape_and_rejects_every_drain() {
    let mut params = std::collections::BTreeMap::new();
    params.insert(
        "lpNft".to_string(),
        ergo_sandbox::TypedValue {
            r#type: "Coll[Byte]".to_string(),
            value: json!("4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41"),
        },
    );
    let src = include_str!("../../examples/incidents/fixed/use-lp-swap.es");
    let fixed = ergo_sandbox::compile::compile_with_params(src, &params, 3, NetworkPrefix::Mainnet)
        .expect("fixed swap compiles");
    let fixed_tree = hex::encode(fixed.tree_bytes);

    let report = drain(use_lp_request(Some(fixed_tree)));
    assert_eq!(report.verdict, DrainVerdict::NotUnderProbes);
    assert_eq!(
        report.hits, 0,
        "the fixed swap must not drain: {:?}",
        report.best
    );
}

#[test]
fn the_gallery_amm_pair_is_not_drainable() {
    // The gallery pair is NFT-bound on both sides: the pool checks its
    // successor's reserves (the product may not fall), the swap order binds
    // the pool by NFT. Every drain-mode probe fails a protocol script.
    let pool_suite: Value =
        serde_json::from_str(include_str!("../../examples/tests/amm-pool.test.json"))
            .expect("pool suite parses");
    let swap_suite: Value = serde_json::from_str(include_str!(
        "../../examples/tests/amm-swap-order.test.json"
    ))
    .expect("swap suite parses");

    let pool_tree = ergo_sandbox::compile::compile_with_params(
        pool_suite["source"].as_str().unwrap(),
        &serde_json::from_value(pool_suite["params"].clone()).unwrap(),
        3,
        NetworkPrefix::Mainnet,
    )
    .expect("pool compiles")
    .tree_bytes;
    let swap_tree = ergo_sandbox::compile::compile_with_params(
        swap_suite["source"].as_str().unwrap(),
        &serde_json::from_value(swap_suite["params"].clone()).unwrap(),
        3,
        NetworkPrefix::Mainnet,
    )
    .expect("swap order compiles")
    .tree_bytes;

    let pool = &pool_suite["scenarios"][0]["selfBox"];
    let fill_out = &pool_suite["scenarios"][0]["outputs"][0];

    let request = json!({
        "inputs": [
            { "role": "protected", "value": pool["value"], "ergoTree": hex::encode(&pool_tree),
              "tokens": pool["tokens"] },
            { "role": "companion", "value": 1000000000, "ergoTree": hex::encode(swap_tree) },
            { "role": "attacker", "value": 215496257079i64 },
        ],
        "outputs": [
            { "payee": "fixed", "value": fill_out["value"], "ergoTree": hex::encode(&pool_tree),
              "tokens": fill_out["tokens"] },
            { "payee": "free", "value": 0, "ergoTree": "0008cd03" ,
              "tokens": [ { "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                            "amount": 884264762 } ] },
        ],
        "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
        "height": 100,
        "network": "mainnet",
    });
    let report = drain(request);
    assert_eq!(report.verdict, DrainVerdict::NotUnderProbes);
    assert_eq!(
        report.hits, 0,
        "the gallery pair must not drain: {:?}",
        report.best
    );
}

#[test]
fn shape_errors_are_verdicts_not_panics() {
    let fx: Value = serde_json::from_str(SWAP_SUITE).unwrap();
    let sc = &fx["scenarios"][0];
    let mut pool = sc["inputs"][2].clone();

    // A protected box that does not carry a protocol NFT.
    pool["tokens"] = json!([{ "id": "1111111111111111111111111111111111111111111111111111111111111111", "amount": 1 }]);
    let request = json!({
        "inputs": [with_key("role", "protected", pool.clone())],
        "outputs": [{ "payee": "free", "value": 0, "ergoTree": pool["ergoTree"], "tokens": [] }],
        "protocolNfts": ["4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41"],
        "height": 1868204,
    });
    let report = drain(request);
    assert_eq!(report.verdict, DrainVerdict::InvalidShape);
    assert!(report.notes.iter().any(|n| n.contains("protocol NFT")));

    // An unresolved role from the protocol map is refused, never defaulted.
    let request = json!({
        "inputs": [{ "role": "unknown", "value": 1, "ergoTree": "d17300" }],
        "outputs": [{ "payee": "free", "value": 0, "ergoTree": "d17300" }],
        "protocolNfts": ["4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41"],
        "height": 1,
    });
    let report = drain(request);
    assert_eq!(report.verdict, DrainVerdict::InvalidShape);
    assert!(report.notes.iter().any(|n| n.contains("unknown")));
}

// ── The protocol map feeds the hunt (map #63 → drain integration) ────────────

mod map_feed {
    use super::*;
    use ergo_sandbox::map::{map, Fixture, MapOptions, ProtocolMap, Seed};

    fn load(name: &str) -> Fixture {
        let path = format!("{}/tests/fixtures/map/{name}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).expect("read fixture");
        Fixture::from_json(&text).expect("parse fixture")
    }

    /// The caps the fixtures were recorded with (tests/map.rs).
    fn recorded_opts() -> MapOptions {
        MapOptions {
            max_depth: 6,
            max_nodes: 96,
            ..MapOptions::default()
        }
    }

    fn attacker() -> Value {
        json!({
            "value": 2000000,
            "ergoTree": "10010101d17300",
        })
    }

    /// The recorded chain state is POST-drain (height 1,868,438 — both pools
    /// were drained at 1,868,204 and 1,868,221), so the honest assertion is
    /// that the mapped sets, as they stand today, have nothing left to drain.
    /// The funded rediscovery is the hand-built positive control above, over
    /// the same deployed trees the map carries.
    fn assert_mapped_set_has_nothing_left(name: &str, seed: &str) {
        let fixture = load(name);
        let m: ProtocolMap = map(&fixture, &Seed::TokenId(seed.to_string()), &recorded_opts())
            .expect("map the recorded set");
        let (mut request, skipped) = ergo_sandbox::drain::request_from_map(
            &m,
            serde_json::from_value(attacker()).expect("attacker box"),
        );
        // The mapped sets carry a dozen-plus inputs; phase-1 enumeration over
        // all of them is thousands of full validations. These tests assert the
        // seam (map roles -> request -> hunt), so the caps are tightened and
        // the truncation is deterministic.
        request.max_permutations = Some(24);
        request.max_probes = Some(500);
        assert!(
            !skipped.is_empty(),
            "the USE/Dexy maps carry unresolved nodes; they are returned, not defaulted"
        );
        assert!(
            request
                .inputs
                .iter()
                .any(|i| i.role == ergo_sandbox::DrainRole::Protected),
            "the map proposed at least one protected box"
        );
        assert!(
            request
                .inputs
                .iter()
                .any(|i| i.role == ergo_sandbox::DrainRole::Companion),
            "the map proposed at least one companion box"
        );
        let report = drain_request(request);
        assert_eq!(
            report.verdict,
            DrainVerdict::NotUnderProbes,
            "the mapped {} set is post-drain: {:?}",
            name,
            report.best
        );
        assert!(report.probes_run > 0, "the hunt actually ran probes");
    }

    #[test]
    fn the_mapped_use_set_feeds_the_hunt_and_has_nothing_left_to_drain() {
        assert_mapped_set_has_nothing_left(
            "use-lp.json",
            "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41",
        );
    }

    #[test]
    fn the_mapped_dexy_set_feeds_the_hunt_and_has_nothing_left_to_drain() {
        assert_mapped_set_has_nothing_left(
            "dexy-gold.json",
            "905ecdef97381b92c2f0ea9b516f312bfb18082c61b24b40affa6a55555c77c7",
        );
    }
}
