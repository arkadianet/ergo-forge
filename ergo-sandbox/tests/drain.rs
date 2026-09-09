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

fn drain_request(mut req: DrainRequest) -> ergo_sandbox::drain::DrainReport {
    req.objective.get_or_insert_with(Default::default);
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

    // Recognized receipts less realized attacker funding. The fixed fee
    // output is not a recognized sink; its 2M nanoERG is excluded.
    assert_eq!(
        best.extracted.get("nanoErg").map(String::as_str),
        Some("284695581089453")
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
    use ergo_sandbox::drain::DrainRole;
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

    #[test]
    fn registers_survive_fixture_map_and_request() {
        const SEED: &str = "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41";
        const POOL: &str = "661fd7516b4a834073396862616f6f7ad5183fd9e693bb54804fb910b6e1598c";
        const SWAP: &str = "a78290a343683e9d5ba16837958608c2e5e7a4f6ea451148d95c5a990e07304e";
        const ORACLE: &str = "3264dbed972175dafbdc9f9834c6dcc624530ced0e1fc4c0d6b0eb582b660deb";
        const R4: &str = "040e"; // Serialized Int(7).

        for register in [
            json!(R4),
            json!({"serializedValue": R4, "sigmaType": "SInt", "renderedValue": "7"}),
        ] {
            let mut archive = serde_json::to_value(load("use-lp.json")).unwrap();
            for recorded in archive["boxesByToken"]
                .as_object_mut()
                .unwrap()
                .values_mut()
            {
                for b in recorded["items"].as_array_mut().unwrap() {
                    if [POOL, SWAP, ORACLE].contains(&b["boxId"].as_str().unwrap()) {
                        b["additionalRegisters"] = json!({"R4": register});
                    }
                }
            }
            let fixture = Fixture::from_json(&archive.to_string()).expect("register fixture");
            // Recording and replay must retain the serialized bytes too.
            let fixture = Fixture::from_json(&fixture.to_json().unwrap()).unwrap();
            let m = map(&fixture, &Seed::TokenId(SEED.into()), &recorded_opts()).unwrap();
            let (request, skipped) = ergo_sandbox::drain::request_from_map(
                &m,
                serde_json::from_value(attacker()).unwrap(),
            );
            for id in [POOL, SWAP, ORACLE] {
                assert_eq!(m.nodes[id].chain_box.registers["R4"], R4);
                assert!(!skipped.iter().any(|skipped| skipped == id));
            }
            for (id, role) in [(POOL, DrainRole::Protected), (SWAP, DrainRole::Companion)] {
                let input = request
                    .inputs
                    .iter()
                    .find(|i| i.box_.box_id.as_deref() == Some(id))
                    .unwrap();
                assert_eq!(input.role, role);
                assert_eq!(input.box_.registers["R4"].r#type, "raw");
                assert_eq!(input.box_.registers["R4"].value, R4);
            }
            let data = request
                .data_inputs
                .iter()
                .find(|b| b.box_id.as_deref() == Some(ORACLE))
                .unwrap();
            assert_eq!(data.registers["R4"].r#type, "raw");
            assert_eq!(data.registers["R4"].value, R4);
            let successor = request
                .outputs
                .iter()
                .find(|o| o.box_.box_id.as_deref() == Some(POOL))
                .unwrap();
            assert_eq!(successor.box_.registers["R4"].r#type, "raw");
            assert_eq!(successor.box_.registers["R4"].value, R4);
        }
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

    /// Unused quota flows forward: a shape whose whole family fits inside
    /// its slice leaves the remainder to later shapes, so a later shape's
    /// real room is its ceiling minus the probes already spent — more than
    /// its nominal slice. Judged on the nominal number, such a shape would
    /// be reported starved while it had room to spare.
    ///
    /// The vault set is the case: shape `none` exhausts at 252 probes
    /// against a 400-probe slice, so `sinks(1)` inherits the difference.
    #[test]
    fn inherited_quota_counts_as_capacity_not_starvation() {
        let corpus: Value =
            serde_json::from_str(include_str!("../../examples/incidents/use-bank-vault.json"))
                .expect("vault corpus parses");
        let vault = &corpus["vault"];
        let admin = &corpus["adminBox"];
        let bank_nft = vault["tokens"][0]["id"].clone();
        let use_id = vault["tokens"][1]["id"].clone();
        let vault_tree = vault["ergoTree"].clone();
        let request = json!({
            "inputs": [
                { "role": "protected", "value": vault["value"], "ergoTree": vault_tree,
                  "tokens": vault["tokens"], "boxId": vault["boxId"],
                  "creationHeight": vault["creationHeight"] },
                { "role": "companion", "value": admin["value"], "ergoTree": admin["ergoTree"],
                  "tokens": admin["tokens"], "boxId": admin["boxId"],
                  "creationHeight": admin["creationHeight"] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 2000000i64, "ergoTree": "10010101d17300", "tokens": [] },
                { "payee": "fixed", "value": 2000000i64, "ergoTree": vault_tree,
                  "tokens": [{ "id": bank_nft, "amount": 1 }, { "id": use_id, "amount": 1 }] },
                { "payee": "free", "value": 0, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": [bank_nft],
            "height": 1868438u32,
            "network": "mainnet",
            "maxProbes": 1200,
            "synthesis": { "maxNewOutputs": 2, "companionRecreations": true,
                           "successorStates": true, "splits": true, "mints": true,
                           "permuteOutputs": true },
        });
        let report = drain(request);
        let syn = &report.synthesis;

        let none = syn
            .shapes
            .iter()
            .find(|t| t.shape == "none")
            .expect("the none tally");
        assert!(
            none.run < none.budget,
            "shape `none` must exhaust inside its slice for quota to flow: {none:?}"
        );
        assert!(
            syn.shapes.iter().any(|t| t.capacity > t.budget),
            "a later shape must inherit the unused quota: {:?}",
            syn.shapes
        );
        // The allocator hands room forward; it never takes room away.
        assert!(
            syn.shapes.iter().all(|t| t.capacity >= t.budget),
            "capacity is never under budget: {:?}",
            syn.shapes
        );
        // A shape with inherited room above the floor is not starved, and a
        // shape that merely finished its own family was never starved.
        assert!(
            syn.shapes
                .iter()
                .all(|t| t.capacity < syn.slice_floor || t.capacity >= t.budget),
            "no shape above the floor may be reported thin: floor={} {:?}",
            syn.slice_floor,
            syn.shapes
        );
    }

    #[test]
    fn the_mapped_dexy_set_feeds_the_hunt_and_has_nothing_left_to_drain() {
        assert_mapped_set_has_nothing_left(
            "dexy-gold.json",
            "905ecdef97381b92c2f0ea9b516f312bfb18082c61b24b40affa6a55555c77c7",
        );
    }

    /// THE FIELD ASSERTION — the one that would have caught both rounds of
    /// the budget bug. On a real mapped set (12 inputs, not a hand-built
    /// 3-input fixture), shape `none` alone produces more points than any
    /// sane cap; depth-first spending therefore zeroed every synthesis
    /// shape. With the allocator, under a binding cap, synthesis shapes
    /// must    /// Breadth the budget cannot pay for is recorded, not hidden. A slice
    /// thinner than one input arrangement's decoy sweep means the shape ran
    /// and covered less than a single arrangement — "it ran" and "it
    /// explored something" are different claims. Asserted both ways: a
    /// starved run names it, a healthy run reports zero.
    #[test]
    fn thin_shape_slices_are_named_and_a_healthy_allocation_is_not() {
        let fixture = load("use-lp.json");
        let m: ProtocolMap = map(
            &fixture,
            &Seed::TokenId(
                "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41".to_string(),
            ),
            &recorded_opts(),
        )
        .expect("map the recorded set");
        let build = |max_probes: usize| {
            let (mut request, _skipped) = ergo_sandbox::drain::request_from_map(
                &m,
                serde_json::from_value(attacker()).expect("attacker box"),
            );
            request.max_probes = Some(max_probes);
            request.synthesis = serde_json::from_value(json!({
                "declaredOutputModifications": true,
            "maxNewOutputs": 2, "companionRecreations": true, "successorStates": true,
                "splits": true, "mints": true, "permuteOutputs": true
            }))
            .expect("synthesis block");
            drain_request(request)
        };

        // Starved: 200 probes over 19 shapes cannot pay for one arrangement's
        // decoy sweep per shape.
        let starved = build(200);
        let syn = &starved.synthesis;
        assert!(
            syn.slice_floor > 0,
            "the floor is one arrangement's decoy sweep, never zero"
        );
        assert!(
            syn.thin_slices > 0,
            "200 probes must starve the slices (floor {}): {:?}",
            syn.slice_floor,
            syn.shapes
        );
        assert!(
            starved
                .notes
                .iter()
                .any(|n| n.contains("cut off with under") && n.contains("decoy sweep")),
            "the thin allocation is named in the notes: {:?}",
            starved.notes
        );
        assert!(
            syn.shapes.iter().any(|t| t.budget < syn.slice_floor),
            "the thin slices are attributable per shape: {:?}",
            syn.shapes
        );

        // Healthy: the same set at the cap the sibling test uses reports
        // zero — the signal is not "synthesis is on", it is "the budget is
        // too thin for the breadth".
        let healthy = build(900);
        assert_eq!(
            healthy.synthesis.thin_slices, 0,
            "900 probes pays for every slice (floor {}): {:?}",
            healthy.synthesis.slice_floor, healthy.synthesis.shapes
        );
        assert!(
            !healthy
                .notes
                .iter()
                .any(|n| n.contains("cut off with under") && n.contains("decoy sweep")),
            "no thin-slice note on a healthy allocation: {:?}",
            healthy.notes
        );
    }

    #[test]
    fn the_mapped_use_set_runs_synthesis_shapes_under_a_binding_cap() {
        let fixture = load("use-lp.json");
        let m: ProtocolMap = map(
            &fixture,
            &Seed::TokenId(
                "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41".to_string(),
            ),
            &recorded_opts(),
        )
        .expect("map the recorded set");
        let (mut request, _skipped) = ergo_sandbox::drain::request_from_map(
            &m,
            serde_json::from_value(attacker()).expect("attacker box"),
        );
        request.max_probes = Some(900);
        // (The review harness used 3000; 900 binds identically on this set —
        // shape `none` alone has >12k points — and keeps the debug-build CI
        // cost at ~40 s instead of ~2 min.)
        request.synthesis = serde_json::from_value(json!({
            "declaredOutputModifications": true,
            "maxNewOutputs": 2, "companionRecreations": true, "successorStates": true,
            "splits": true, "mints": true, "permuteOutputs": true
        }))
        .expect("synthesis block");
        let input_count = request.inputs.len();
        let output_count = request.outputs.len();
        let report = drain_request(request);
        println!("MAPPED_USE_MEASUREMENT {}", json!({
            "inputs": input_count, "outputs": output_count,
            "probesRun": report.probes_run, "capped": report.capped,
            "synthesis": report.synthesis, "rejections": report.rejections,
        }));

        assert!(report.capped, "900 probes must bind on a 12-input set");
        assert!(
            report.probes_run <= 900,
            "the cap is respected: {}",
            report.probes_run
        );
        // The phase-1 point gets a slice, not the whole budget.
        let none = report
            .synthesis
            .shapes
            .iter()
            .find(|t| t.shape == "none")
            .expect("the none tally");
        assert!(
            none.run < 900,
            "shape `none` must not consume the whole budget: run={} of {}",
            none.run,
            report.probes_run
        );
        // At least one re-creation shape ran — on a mapped set, with the
        // request's own protocolNfts (the protected pools' NFTs) untouched.
        let recreates: Vec<_> = report
            .synthesis
            .shapes
            .iter()
            .filter(|t| t.shape.starts_with("recreate("))
            .collect();
        assert!(
            !recreates.is_empty(),
            "re-creation shapes must exist on the mapped set: {:?}",
            report.synthesis.shapes
        );
        assert!(
            recreates.iter().any(|t| t.run > 0),
            "at least one re-creation shape must run under a binding cap: {recreates:?}"
        );
        // Every enabled shape got a slice of the cap.
        assert!(
            report.synthesis.shapes.iter().all(|t| t.run > 0),
            "allocation must reach every synthesis shape: {:?}",
            report.synthesis.shapes
        );
        // The allocator policy is recorded next to the pinned order.
        assert!(
            report.synthesis.allocation.contains("allocated across"),
            "{}",
            report.synthesis.allocation
        );
        // Companion visibility: the mapped set's companions (order boxes
        // with their own amount-1 singletons) are considered and qualify.
        assert!(
            report.synthesis.companions_considered > 0 && report.synthesis.companions_qualified > 0,
            "mapped companions carry amount-1 singletons: considered={} qualified={}",
            report.synthesis.companions_considered,
            report.synthesis.companions_qualified
        );
    }
}

// ── Phase 2: output synthesis ─────────────────────────────────────────────────
//
// Spec: `docs/superpowers/specs/2026-09-08-drain-hunt-phase2-design.md` (merged
// in #65). The non-negotiable gate: with `synthesis` absent or all-off,
// behavior is byte-identical to phase 1 — the recorded tests above are that
// gate. Everything here is additive.

/// A minimal balanced shape: one protected box, one attacker box, the
/// successor rebuilt verbatim, the attacker's own value returned to them.
/// Small enough to run three ways in a regression test.
fn minimal_request() -> Value {
    let pass_tree = "10010101d17300"; // sigmaProp(true)
    json!({
        "inputs": [
            { "role": "protected", "value": 10000000i64, "ergoTree": pass_tree,
              "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
            { "role": "attacker", "value": 2000000i64, "ergoTree": pass_tree },
        ],
        "outputs": [
            { "payee": "fixed", "value": 10000000i64, "ergoTree": pass_tree,
              "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
            { "payee": "free", "value": 2000000i64, "ergoTree": pass_tree, "tokens": [] },
        ],
        "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
        "height": 100,
        "network": "mainnet",
    })
}

#[test]
fn declared_attacker_keys_do_not_relax_the_protected_input_gate() {
    let key = "028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197";
    let mut request = minimal_request();
    request["inputs"][0]["ergoTree"] = json!(format!("0008cd{key}"));
    request["attackerPublicKeys"] = json!([key]);
    request["synthesis"] = json!({ "successorStates": true });
    let report = drain(request);
    assert_eq!(report.verdict, DrainVerdict::NotUnderProbes);
    assert_eq!(report.hits, 0);
    assert!(report.rejections.missing_key > 0);
    assert!(report.first_accounting.is_none());
}

#[test]
fn undeclared_objective_reports_incomplete_instead_of_an_extraction_verdict() {
    let req: DrainRequest = serde_json::from_value(minimal_request()).unwrap();
    let report = ergo_sandbox::decompile::with_large_stack(move || drain_hunt(&req)).unwrap();
    assert_eq!(report.verdict, DrainVerdict::IncompleteObjective);
    assert_eq!(report.hits, 0);
    assert!(report.best.is_none());
    assert!(report
        .notes
        .iter()
        .any(|n| n.contains("incomplete objective")));
}

mod synthesis {
    use super::*;

    fn with_synthesis(request: Value, block: Value) -> Value {
        let mut r = request;
        r["synthesis"] = block;
        r
    }

    /// The non-negotiable gate, at the report level: the omitted block, the
    /// all-off block, and the Rust default produce identical behavior — and
    /// the omitted block and the all-off block produce identical reports.
    #[test]
    fn omitted_all_off_and_default_synthesis_are_identical() {
        let omitted = drain(minimal_request());
        let all_off = drain(with_synthesis(
            minimal_request(),
            json!({ "companionRecreations": false, "successorStates": false,
                    "splits": false, "mints": false, "permuteOutputs": false }),
        ));
        let a = serde_json::to_value(&omitted).unwrap();
        let b = serde_json::to_value(&all_off).unwrap();
        assert_eq!(a, b, "omitted vs all-off synthesis must be byte-identical");
        assert_eq!(omitted.verdict, DrainVerdict::Drainable); // canonical successor is spendable
        assert_eq!(omitted.probes_run, all_off.probes_run);

        // An explicit `maxNewOutputs: 0` is the spec's all-off block too:
        // behavior is identical (the cap is inert while every degree is off);
        // only the recorded cap differs, as declared.
        let explicit_zero = drain(with_synthesis(
            minimal_request(),
            json!({ "maxNewOutputs": 0, "companionRecreations": false, "successorStates": false,
                    "splits": false, "mints": false, "permuteOutputs": false }),
        ));
        assert_eq!(omitted.verdict, explicit_zero.verdict);
        assert_eq!(omitted.probes_run, explicit_zero.probes_run);
        assert_eq!(omitted.probes_total, explicit_zero.probes_total);
        assert_eq!(omitted.hits, explicit_zero.hits);
        assert_eq!(
            serde_json::to_value(&omitted.best).unwrap(),
            serde_json::to_value(&explicit_zero.best).unwrap()
        );
        assert_eq!(explicit_zero.synthesis.caps.max_new_outputs, 0);

        // An explicit cap alone is NOT a degree: with every degree off the
        // block is inactive, no shape beyond `none` exists, and the report
        // is phase 1's — byte for byte.
        let cap_only = drain(with_synthesis(
            minimal_request(),
            json!({ "maxNewOutputs": 2 }),
        ));
        assert_eq!(
            serde_json::to_value(&cap_only).unwrap(),
            a,
            "an inactive block with an explicit cap must be byte-identical to phase 1"
        );
    }

    /// Truncation is the normal case, so the pinned axis order decides which
    /// probes exist at all: synthesized-output shapes outermost (none →
    /// companion re-creations → sinks), the phase-1 axes innermost, filler
    /// counts innermost of all. With a cap that truncates inside the first
    /// re-creation's inner space, the run must contain shape-`none` probes,
    /// the unpadded re-creation, AND the padded ones — no degree may starve
    /// another.
    ///
    /// The companion qualifies through its OWN singleton (amount 1), not
    /// through `protocolNfts`, which names only the protected box's NFT —
    /// that overloading was the one field-tested way to fire this axis, and
    /// it is exactly what this test no longer does.
    #[test]
    fn the_pinned_axis_order_survives_truncation() {
        // One protected box, one companion carrying its own singleton, one
        // attacker slot. Recreations + output permutation only, so the space
        // is exactly: [none] × 2 outperms × 8 combos × 2 payouts = 32 runs,
        // then the re-creation shape × 6 outperms × 8 combos × 2 payouts ×
        // 4 filler counts.
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 100000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "companion", "value": 1000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 100000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "payee": "free", "value": 3000000i64, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": [
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ],
            "height": 100,
            "network": "mainnet",
            "maxPermutations": 1,
            "maxProbes": 60,
            "synthesis": { "companionRecreations": true, "permuteOutputs": true },
        });
        let report = drain(request);
        // The verdict is incidental here (the protected box carries no
        // guard): this test pins the AXIS ORDER and the BUDGET ALLOCATION
        // under truncation.
        assert!(report.capped, "the cap must bind: 32 + 384 >> 60");
        let shapes = &report.synthesis.shapes;
        assert_eq!(shapes[0].shape, "none");
        assert!(shapes[0].run > 0);
        // ALLOCATION, not depth-first spending: shape `none` alone has 32
        // unique points on this request — it must NOT eat the budget. (This
        // is the field assertion: depth-first spending made `none` consume
        // everything and every synthesis shape came back run=0.)
        assert!(
            shapes[0].run < 32,
            "shape `none` must not consume the whole budget: {shapes:?}"
        );
        // The blocking-bug regression: protocolNfts names ONLY the protected
        // box's NFT, yet re-creation shapes must fire — the companion's own
        // amount-1 token is what drives them.
        assert!(
            shapes
                .iter()
                .any(|t| t.shape.starts_with("recreate(") && t.run > 0),
            "re-creations must fire on a request whose protocolNfts is just the protected NFT: {shapes:?}"
        );
        // Padded shapes are reachable by construction: the filler count is
        // the innermost axis, so fillers=1..3 run inside the first
        // re-creation's budget instead of starving behind the unpadded one.
        for f in 1..=3 {
            let bucket = format!("recreate(companion=1,nft=0,fillers={f})");
            assert!(
                shapes.iter().any(|t| t.shape == bucket && t.run > 0),
                "padded shape {bucket} must run under truncation: {shapes:?}"
            );
        }
        // ALLOCATION: under a binding cap every synthesis shape keeps a
        // slice — truncation preserves the degrees instead of letting the
        // first shape (or the first shapes) consume everything.
        assert!(
            shapes.iter().all(|t| t.run > 0),
            "every synthesis shape must get budget under truncation: {shapes:?}"
        );
        // The slices are recorded per shape (buckets of one shape share
        // that shape's slice): 4 shapes — none, recreate, recreate+sink,
        // sinks(1) — split the 60-probe cap evenly; the 10 tally buckets
        // each report their shape's 15-probe slice.
        assert!(shapes.iter().all(|t| t.budget == 60 / 4));
        assert!(shapes.iter().all(|t| t.run <= t.budget));
        // The caps and the pinned order are recorded with the miss.
        assert_eq!(report.synthesis.caps.max_probes, 60);
        assert_eq!(report.synthesis.caps.max_new_outputs, 2);
        assert_eq!(
            report.synthesis.axis_order,
            vec![
                "output shapes: none → declared-output modifications → companion re-creations → sinks",
                "output permutation",
                "per-successor states",
                "value splits",
                "mint variants",
                "input permutation + decoy combinations",
                "filler counts (re-creation shapes, innermost)",
            ]
        );
    }

    /// The field-shaped control for the re-creation axis: a request whose
    /// `protocolNfts` is exactly the protected box's NFT — as every
    /// naturally-constructed request is — must still generate re-creation
    /// shapes. This is the assertion the axis's first version could not
    /// pass (it gated on `protocolNfts`, which no companion satisfies).
    #[test]
    fn recreations_fire_without_overloading_protocol_nfts() {
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "companion", "value": 1000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "payee": "free", "value": 4000000i64, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": [
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ],
            "height": 100,
            "network": "mainnet",
            "maxPermutations": 1,
            "synthesis": { "companionRecreations": true },
        });
        let report = drain(request);
        assert!(
            report
                .synthesis
                .shapes
                .iter()
                .any(|t| t.shape.starts_with("recreate(") && t.generated > 0 && t.run > 0),
            "the axis must fire on the natural request: {:?}",
            report.synthesis.shapes
        );
    }

    /// The vault's admin route, refused by construction (corrected decode: the
    /// whitelist is `INPUTS(0).tokens(0)._1`): carrying `useUpdateNft` means
    /// spending the admin's P2PK box — `needsProof` on a companion input,
    /// disqualified by the phase-1 gate. Deterministic, asserted
    /// unconditionally.
    #[test]
    fn the_vault_admin_route_is_refused_by_construction() {
        let corpus: Value =
            serde_json::from_str(include_str!("../../examples/incidents/use-bank-vault.json"))
                .expect("vault corpus parses");
        let vault = &corpus["vault"];
        let admin = &corpus["adminBox"];
        let bank_nft = vault["tokens"][0]["id"].clone();
        let use_id = vault["tokens"][1]["id"].clone();
        let vault_tree = vault["ergoTree"].clone();

        // The corrected-decode shape: the faithful continuation sits at
        // OUTPUTS(1) (what the deployed script checks), everything else is
        // attacker-buildable, synthesis fully on.
        let request = json!({
            "inputs": [
                { "role": "protected", "value": vault["value"], "ergoTree": vault_tree,
                  "tokens": vault["tokens"], "boxId": vault["boxId"],
                  "creationHeight": vault["creationHeight"] },
                { "role": "companion", "value": admin["value"], "ergoTree": admin["ergoTree"],
                  "tokens": admin["tokens"], "boxId": admin["boxId"],
                  "creationHeight": admin["creationHeight"] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 2000000i64, "ergoTree": "10010101d17300", "tokens": [] },
                { "payee": "fixed", "value": 2000000i64, "ergoTree": vault_tree,
                  "tokens": [{ "id": bank_nft, "amount": 1 }, { "id": use_id, "amount": 1 }] },
                { "payee": "free", "value": 0, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": [bank_nft],
            "height": 1868438u32,
            "network": "mainnet",
            "maxProbes": 3000,
            "synthesis": { "maxNewOutputs": 2, "companionRecreations": true,
                           "successorStates": true, "splits": true, "mints": true,
                           "permuteOutputs": true },
        });
        let report = drain(request);
        assert_eq!(report.verdict, DrainVerdict::NotUnderProbes);
        assert_eq!(
            report.hits, 0,
            "no witness may exist at all: {:?}",
            report.best
        );
        // The refusal is the gate, not the script: some probe must have put
        // the admin box at INPUTS(0) (whitelist satisfied) and been
        // disqualified for holding a key the attacker does not have.
        //
        // What this test can and cannot catch, on purpose: every probe here
        // dies on that P2PK gate (rejections come back ~all missingKey), so
        // the test can only regress on the GATE — a change that let a
        // needsProof companion pass. It cannot speak to the three script
        // authorizers at all: they are deliberately absent from the input
        // set, so the empirical flagship question (do freeMint/arbMint/
        // payout constrain the bank's value?) is not exercised in CI, and
        // this test structurally cannot surface a live vault drain.
        assert!(
            report
                .notes
                .iter()
                .any(|n| n.contains("needs a key the attacker does not hold")),
            "the admin route must be tried and refused: {:?}",
            report.notes
        );
        // Companion visibility, recorded per the review: the corpus request's
        // only companion is the admin box, whose useUpdateNft sits at AMOUNT
        // 3 — above the amount-1 singleton shape — so the re-creation axis
        // generates nothing on this request. That is correct under the
        // corrected decode (the vault whitelist is input-side; the vault
        // needs no output padding), and the report says so instead of
        // leaving a silently-empty axis.
        assert_eq!(report.synthesis.companions_considered, 1);
        assert_eq!(report.synthesis.companions_qualified, 0);
        assert!(
            report
                .notes
                .iter()
                .any(|n| n.contains("0 qualified") && n.contains("axis will generate nothing")),
            "the empty axis must be readable in the report: {:?}",
            report.notes
        );
        assert!(
            report
                .synthesis
                .shapes
                .iter()
                .all(|t| !t.shape.starts_with("recreate(")),
            "no re-creation shape is generated on the corpus request: {:?}",
            report.synthesis.shapes
        );
    }

    /// Canonical sinks are recognized even when script-matched to a victim.
    /// NFT detachment remains a separate synthesis diagnostic.
    #[test]
    fn an_nft_detached_successor_shields_nothing_and_is_reported() {
        let pool_tree = "10010101d17300";
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 100000000i64, "ergoTree": pool_tree,
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 },
                             { "id": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "amount": 1000 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": pool_tree },
            ],
            "outputs": [
                { "payee": "fixed", "value": 100000000i64, "ergoTree": pool_tree,
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 },
                             { "id": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "amount": 1000 }] },
                // The trap: the free payee carries the pool's script — and so
                // does the attacker tree drain mode re-trees it to.
                { "payee": "free", "value": 0, "ergoTree": pool_tree, "tokens": [] },
            ],
            "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "attackerTree": pool_tree,
            "height": 100,
            "network": "mainnet",
        });
        // Recognition is independent of synthesis and script custody.
        let phase1 = drain(request.clone());
        assert_eq!(phase1.verdict, DrainVerdict::Drainable);
        assert!(phase1.nft_detached.is_empty());

        // Synthesis on also records the NFT-less clone diagnostic.
        let mut phase2 = request;
        phase2["synthesis"] = json!({ "successorStates": true });
        let phase2 = drain(phase2);
        assert_eq!(phase2.verdict, DrainVerdict::Drainable);
        let best = phase2.best.as_ref().expect("a hit");
        assert!(
            best.extracted
                .get("nanoErg")
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 90_000_000,
            "the reserves must count as leaked: {:?}",
            best.extracted
        );
        assert!(
            !phase2.nft_detached.is_empty(),
            "the script-matched NFT-less output must be named: {:?}",
            phase2.nft_detached
        );
    }

    /// Rejection classification: a synthesized probe's rejection is tallied
    /// as `conservation`, `missingKey`, `script`, or `invalid` — a `notUnderProbes` can
    /// never silently mean "conservation blocked us".
    #[test]
    fn synthesized_rejections_are_classified_and_tallied() {
        // Conservation: a companion whose NFT the template already outputs —
        // every re-creation of it doubles the NFT and fails conservation.
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "companion", "value": 1000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                // The companion's NFT already lands here.
                { "payee": "fixed", "value": 3000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 1 }] },
                { "payee": "free", "value": 0, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": [
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ],
            "height": 100,
            "network": "mainnet",
            "maxPermutations": 1,
            "synthesis": { "companionRecreations": true },
        });
        let report = drain(request);
        assert!(
            report.rejections.conservation > 0,
            "{:?}",
            report.rejections
        );

        // Missing key: the companion is a P2PK box — every probe that spends
        // it needs a signature the attacker does not hold.
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "companion", "value": 2000000i64,
                  "ergoTree": "0008cd03ff71c397ce77130349773ed64818e7c084845ba8b27fcd097ed989e063afc3d8",
                  "tokens": [{ "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "payee": "free", "value": 4000000i64, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "height": 100,
            "network": "mainnet",
            "synthesis": { "successorStates": true },
        });
        let report = drain(request);
        assert!(report.rejections.missing_key > 0, "{:?}", report.rejections);

        // Script: the protected box demands a shape the probes do not build.
        let false_tree = hex::encode(
            ergo_sandbox::compile::compile_source("sigmaProp(false)", 3, NetworkPrefix::Mainnet)
                .expect("false script compiles")
                .tree_bytes,
        );
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000i64, "ergoTree": false_tree,
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "payee": "free", "value": 2000000i64, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "height": 100,
            "network": "mainnet",
            "synthesis": { "successorStates": true },
        });
        let report = drain(request);
        assert!(report.rejections.script > 0, "{:?}", report.rejections);
        assert_eq!(report.rejections.invalid, 0);
    }

    #[test]
    fn malformed_probes_are_invalid_not_script_rejections() {
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "attacker", "value": 2000000, "ergoTree": "10010101d17300" }
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "payee": "free", "value": 2000000, "ergoTree": "10010101d17300" }
            ],
            "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "height": 100,
            "synthesis": { "successorStates": true }
        });
        let mut cases = Vec::new();
        for tree in ["amm-pool", "ff"] {
            let mut bad_tree = request.clone();
            bad_tree["inputs"][0]["ergoTree"] = json!(tree);
            cases.push(bad_tree);
        }
        let mut bad_output = request.clone();
        bad_output["outputs"][0]["ergoTree"] = json!("not-hex");
        cases.push(bad_output);

        let mut bad_box = request.clone();
        bad_box["inputs"][0]["boxId"] = json!("abcd");
        cases.push(bad_box);

        let mut bad_register = request.clone();
        bad_register["inputs"][0]["registers"] = json!({
            "R4": { "type": "raw", "value": "not-hex" }
        });
        cases.push(bad_register);

        // The drain marshaller requires raw registers; failure happens
        // before txcheck returns per-input verdicts and must still count.
        let mut marshalling = request;
        marshalling["inputs"][0]["registers"] = json!({
            "R4": { "type": "Long", "value": 1 }
        });
        cases.push(marshalling);

        for request in cases {
            let report = drain(request.clone());
            assert_eq!(report.verdict, DrainVerdict::NotUnderProbes);
            assert_eq!(report.hits, 0);
            assert!(report.probes_run > 0);
            assert_eq!(report.rejections.invalid, report.probes_run, "{request}");
            assert_eq!(report.rejections.script, 0);
            assert_eq!(report.rejections.conservation, 0);
            assert_eq!(report.rejections.missing_key, 0);
            let json = serde_json::to_value(&report).unwrap();
            assert_eq!(json["rejections"]["invalid"], report.probes_run);
        }
    }

    /// Mint probes: conservation allows one minted id — the first input's box
    /// id. A protocol whose script demands a token the inputs do not carry
    /// (`OUTPUTS(2).tokens(0)._1 == INPUTS(0).id`) is satisfied by a minted
    /// placeholder, and the amount comes from the declared request alone.
    #[test]
    fn a_minted_placeholder_forges_the_successor_shaped_box() {
        // `d17300` = sigmaProp(false): the protected box demands the
        // impossible — implemented below by a tree that checks OUTPUTS(2).
        let pool_tree_src = "sigmaProp(OUTPUTS(2).tokens(0)._1 == INPUTS(0).id)";
        let pool_tree = hex::encode(
            ergo_sandbox::compile::compile_source(pool_tree_src, 3, NetworkPrefix::Mainnet)
                .expect("pool script compiles")
                .tree_bytes,
        );
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000i64, "ergoTree": pool_tree,
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000i64, "ergoTree": pool_tree,
                  "tokens": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "amount": 1 }] },
                { "payee": "free", "value": 2000000i64, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "height": 100,
            "network": "mainnet",
        });
        // Without mints (phase 1), no probe can satisfy the script.
        let without = drain(request.clone());
        assert_eq!(without.verdict, DrainVerdict::NotUnderProbes);

        // With mints, the sink carries the minted placeholder and drains.
        let mut with = request;
        with["synthesis"] = json!({ "mints": true });
        let with = drain(with);
        assert_eq!(
            with.verdict,
            DrainVerdict::Drainable,
            "{:?}",
            with.rejections
        );
        let best = with.best.as_ref().expect("a hit");
        // 10M recognized receipts minus 2M realized attacker funding.
        // The declared free amount is not an additional allowance.
        assert_eq!(
            best.extracted.get("nanoErg").map(String::as_str),
            Some("8000000")
        );
        // The witness validates: the minted id is the first input's box id.
        let check = tx_check(&best.witness.tx_request).expect("txrequest checks");
        assert!(check.valid, "witness problems: {:?}", check.problems);
        let first_input = &best.witness.tx_request.tx.inputs[0].box_id;
        assert_eq!(best.accounting.n[first_input], "1");
        assert!(!best.extracted.contains_key(first_input));
        let sink = &best.witness.tx_request.tx.outputs[2];
        let sink_tokens = sink["assets"].as_array().unwrap();
        assert_eq!(
            sink_tokens[0]["tokenId"].as_str(),
            Some(first_input.as_str())
        );
        // Amounts come from the declared request: {1, largest declared input
        // holding (1), largest declared output amount (1)}.
        assert_eq!(sink_tokens[0]["amount"].as_u64(), Some(1));
    }

    /// Minimizing a canonical successor changes its position/quantity but
    /// not the total recognized receipts: its retained value already counts.
    #[test]
    fn per_successor_states_preserve_recognized_receipt_accounting() {
        let p1_src = "sigmaProp(OUTPUTS(0).value == SELF.value)";
        let p1_tree = hex::encode(
            ergo_sandbox::compile::compile_source(p1_src, 3, NetworkPrefix::Mainnet)
                .expect("p1 script compiles")
                .tree_bytes,
        );
        let nft_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let nft_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let request = json!({
            "inputs": [
                { "role": "protected", "value": 10000000i64, "ergoTree": p1_tree,
                  "tokens": [{ "id": nft_a, "amount": 1 }] },
                { "role": "protected", "value": 50000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": nft_b, "amount": 1 }] },
                { "role": "attacker", "value": 2000000i64, "ergoTree": "10010101d17300" },
            ],
            "outputs": [
                { "payee": "fixed", "value": 10000000i64, "ergoTree": p1_tree,
                  "tokens": [{ "id": nft_a, "amount": 1 }] },
                { "payee": "fixed", "value": 50000000i64, "ergoTree": "10010101d17300",
                  "tokens": [{ "id": nft_b, "amount": 1 }] },
                { "payee": "free", "value": 2000000i64, "ergoTree": "10010101d17300", "tokens": [] },
            ],
            "protocolNfts": [nft_a, nft_b],
            "height": 100,
            "network": "mainnet",
        });
        // P2's canonical successor already contributes 50M to A.
        let phase1 = drain(request.clone());
        assert_eq!(phase1.verdict, DrainVerdict::Drainable);
        assert_eq!(
            phase1.best.as_ref().unwrap().extracted["nanoErg"],
            "50000000"
        );

        // Phase 2: [P1 verbatim, P2 minimized] drains P2 through the shape
        // P1's script permits.
        let mut phase2 = request;
        phase2["synthesis"] = json!({ "successorStates": true });
        let phase2 = drain(phase2);
        assert_eq!(phase2.verdict, DrainVerdict::Drainable);
        let best = phase2.best.as_ref().expect("a hit");
        // All P2 holdings remain recognized wherever the axis moves them;
        // P1's 10M stays outside A, and the attacker's 2M is subtracted once.
        assert_eq!(
            best.extracted.get("nanoErg").map(String::as_str),
            Some("50000000")
        );
    }
}
