//! The protocol map's acceptance criteria, run **offline** against recorded
//! chain fixtures.
//!
//! The fixtures under `tests/fixtures/map/` were fetched once from
//! `https://api.ergoplatform.com` by
//! `cargo run -p ergo-sandbox --example record_map_fixture` and committed.
//! Nothing here touches a network, and nothing here submits anything.
//!
//! Design record: `docs/superpowers/specs/2026-09-08-protocol-map-design.md`.

use std::collections::{BTreeMap, BTreeSet};

use ergo_sandbox::map::refs::Binding;
use ergo_sandbox::map::source::{ChainBox, ChainToken, TokenInfo};
use ergo_sandbox::map::{
    fixture::Recorded, json, map, Fixture, MapOptions, ProtocolMap, Role, Seed, Target,
};
use ergo_ser::address::NetworkPrefix;

// ── the two recorded deployments ─────────────────────────────────────────────

/// The USE/Dexy LP set, seeded with the LP NFT alone.
const USE_LP_NFT: &str = "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41";
/// The DexyGold set — the twin deployment.
const DEXY_LP_NFT: &str = "905ecdef97381b92c2f0ea9b516f312bfb18082c61b24b40affa6a55555c77c7";

// The USE boxes the acceptance criteria name, at the recorded height.
const USE_POOL: &str = "661fd7516b4a834073396862616f6f7ad5183fd9e693bb54804fb910b6e1598c";
const USE_SWAP: &str = "a78290a343683e9d5ba16837958608c2e5e7a4f6ea451148d95c5a990e07304e";
const USE_MINT: &str = "03adebbed89b876c44752428ee829ef0e49f932834f55c9518aaf6ac7cdaf446";
const USE_REDEEM: &str = "37ddfaddc7b1f96dad9ec65aae1d022bba487a9c4583d34a695f137554c30a38";
const USE_EXTRACT: &str = "8de335f740e9b8d7230dfd2c6f7419abacdb434c0d33df37bfdf2f15e2e28f95";
const USE_ORACLE: &str = "3264dbed972175dafbdc9f9834c6dcc624530ced0e1fc4c0d6b0eb582b660deb";
const USE_TRACKER_95: &str = "bfe2fa37e8a9273564ec260c793b2190924154364a8a4c8e4c4f7dd7618c4233";
const USE_TRACKER_98: &str = "a837d818c6c463959409c468783bc2e53776e7df9bc0450862c84eef50981a4b";
const USE_TRACKER_101: &str = "ab2e120c5c2b0dc495408deb29a1ea49fecdbf1ec7cf187a77254c86ff658b22";

/// The caps the fixtures were recorded with. Replaying with different caps
/// would ask the archive questions it was never asked live.
fn recorded_opts() -> MapOptions {
    MapOptions {
        max_depth: 6,
        max_nodes: 96,
        ..MapOptions::default()
    }
}

fn load(name: &str) -> Fixture {
    let text = std::fs::read_to_string(format!(
        "{}/tests/fixtures/map/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read fixture");
    Fixture::from_json(&text).expect("parse fixture")
}

fn use_map() -> ProtocolMap {
    map(
        &load("use-lp.json"),
        &Seed::TokenId(USE_LP_NFT.into()),
        &recorded_opts(),
    )
    .expect("map the USE set")
}

fn dexy_map() -> ProtocolMap {
    map(
        &load("dexy-gold.json"),
        &Seed::TokenId(DEXY_LP_NFT.into()),
        &recorded_opts(),
    )
    .expect("map the DexyGold set")
}

/// Box id → the protocol NFT it carries, for readable assertions.
fn nfts(m: &ProtocolMap) -> BTreeMap<&str, &str> {
    m.nodes
        .values()
        .filter_map(|n| Some((n.chain_box.box_id.as_str(), n.nft.as_deref()?)))
        .collect()
}

fn edges_between<'a>(m: &'a ProtocolMap, from: &str, to: &str) -> Vec<&'a ergo_sandbox::map::Edge> {
    m.edges
        .iter()
        .filter(|e| e.from == from && matches!(&e.to, Target::Node(t) if t == to))
        .collect()
}

fn finding_pairs(m: &ProtocolMap, lint: &str) -> BTreeSet<(String, String)> {
    m.findings
        .iter()
        .filter(|f| f.finding.lint == lint)
        .filter_map(|f| match &f.to {
            Target::Node(t) => Some((f.from.clone(), t.clone())),
            Target::Unresolved(_) => None,
        })
        .collect()
}

// ── criterion 1: rebuild the USE set from one seed ───────────────────────────

/// Every contract the incident was mapped to **by hand** comes back from the
/// LP NFT alone: pool, swap, mint, redeem, extract, intervention, bank,
/// free-mint, arbitrage-mint, payout, buyback, oracle, tracking, update.
#[test]
fn the_use_set_is_rebuilt_from_the_lp_nft_alone() {
    let m = use_map();
    let by_nft = nfts(&m);
    let held: BTreeSet<&str> = by_nft.values().copied().collect();

    // Contracts identified by their singleton NFT — the protocol NFTs.
    let expected: [(&str, &str); 11] = [
        (
            "pool",
            "4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41",
        ),
        (
            "swap",
            "ef461517a55b8bfcd30356f112928f3333b5b50faf472e8374081307a09110cf",
        ),
        (
            "mint",
            "2cf9fb512f487254777ac1d086a55cda9e74a1009fe0d30390a3792f050de58f",
        ),
        (
            "redeem",
            "1bfea21924f670ca5f13dd6819ed3bf833ec5a3113d5b6ae87d806db29b94b9a",
        ),
        (
            "extract",
            "bc685d6ad1703ba5775736308fd892807edc04f48ba7a52e802fab241a59962c",
        ),
        (
            "intervention",
            "dbf655f0f6101cb03316e931a689412126fefbfb7c78bd9869ad6a1a58c1b424",
        ),
        (
            "bank",
            "78c24bdf41283f45208664cd8eb78e2ffa7fbb29f26ebb43e6b31a46b3b975ae",
        ),
        (
            "free-mint",
            "40db16e1ed50b16077b19102390f36b41ca35c64af87426d04af3b9340859051",
        ),
        (
            "arbitrage-mint",
            "c79bef6fe21c788546beab08c963999d5ef74151a9b7fd6c1843f626eea0ecf5",
        ),
        (
            "payout",
            "a2482fca4ca774ef9d3896977e3677b031597c6e312b0c10d47157bb0d6ed69f",
        ),
        (
            "oracle",
            "6a2b821b5727e85beb5e78b4efb9f0250d59cd48481d2ded2c23e91ba1d07c66",
        ),
    ];
    for (name, nft) in expected {
        assert!(held.contains(nft), "{name} ({nft}) missing from the map");
    }

    // The three tracking boxes.
    for id in [USE_TRACKER_95, USE_TRACKER_98, USE_TRACKER_101] {
        assert!(m.nodes.contains_key(id), "tracking box {id} missing");
    }

    // The update NFT and the buyback are recovered as *boxes*, but neither
    // token is a protocol NFT: both were minted in quantity 3, so holding one
    // identifies no single box. That is the spec's definition doing its job —
    // they are referenced tokens, present in the map and in its edges, and
    // deliberately not identity.
    const USE_UPDATE: &str = "f77b3cac4f77a31aeffaf716070345b3b04330bbba02e27671015129fb74e883";
    const USE_BUYBACK: &str = "dcce07af04ea4f9b7979336476594dc16321547bcc9c6b95a67cb1a94192da4f";
    for (name, token) in [("update", USE_UPDATE), ("buyback", USE_BUYBACK)] {
        assert_eq!(
            m.tokens.get(token),
            Some(&ergo_sandbox::map::TokenClass::Fungible(3)),
            "{name} is referenced but minted in quantity 3"
        );
        assert!(
            !m.protocol_nfts.contains(token),
            "{name} cannot establish identity, so it is not a protocol NFT"
        );
        assert!(
            m.nodes
                .values()
                .any(|n| n.chain_box.tokens.iter().any(|t| t.id == token)),
            "the {name} box is still recovered as a node"
        );
    }

    assert!(
        m.nodes.len() >= 20,
        "the walk should recover the whole neighbourhood, got {}",
        m.nodes.len()
    );
}

/// Every edge in the USE map carries a binding drawn from the spec's table,
/// and an `nft` edge records whether the token is a singleton.
#[test]
fn every_use_edge_is_typed() {
    let m = use_map();
    assert!(!m.edges.is_empty());
    for e in &m.edges {
        match e.binding {
            Binding::Nft => assert!(
                e.singleton.is_some(),
                "an nft edge must say singleton or fungible: {e:?}"
            ),
            Binding::ScriptHash | Binding::SelfSuccessor => {
                assert!(e.covers.contains(&ergo_sandbox::map::refs::Cover::Script));
            }
            Binding::Positional => assert!(
                e.covers.is_empty(),
                "a positional site pins nothing, so it covers nothing: {e:?}"
            ),
            Binding::DataInput => {}
        }
        assert!(!e.site.is_empty());
    }
}

// ── criterion 2: find the asymmetry unaided ──────────────────────────────────

/// The pool pins the swap by NFT; the swap refers back to the pool only by
/// position and does the constant-product maths on whatever sits there. That
/// asymmetry is the 2026-09-08 drain, and the map finds it from one seed.
#[test]
fn the_pool_swap_asymmetry_is_found_unaided() {
    let m = use_map();

    let pool_to_swap = edges_between(&m, USE_POOL, USE_SWAP);
    assert!(
        pool_to_swap
            .iter()
            .any(|e| e.binding == Binding::Nft && e.singleton == Some(true)),
        "pool -> swap must type nft(singleton): {pool_to_swap:?}"
    );

    let swap_to_pool = edges_between(&m, USE_SWAP, USE_POOL);
    let positional = swap_to_pool
        .iter()
        .find(|e| e.binding == Binding::Positional)
        .expect("swap -> pool must type positional");
    assert!(
        positional.value_math,
        "the swap does value maths on the box it never pins"
    );
    assert!(
        positional.covers.is_empty(),
        "a positional site covers no identity dimension"
    );

    let unbound = finding_pairs(&m, ergo_sandbox::map::LINT_UNBOUND_SET);
    for (name, from) in [
        ("swap", USE_SWAP),
        ("mint", USE_MINT),
        ("redeem", USE_REDEEM),
    ] {
        assert!(
            unbound.contains(&(from.to_string(), USE_POOL.to_string())),
            "{name} must raise the set-level finding against the pool"
        );
    }
    assert!(
        !unbound.contains(&(USE_EXTRACT.to_string(), USE_POOL.to_string())),
        "extract pins the LP by NFT, so it must NOT be flagged"
    );

    let asym = finding_pairs(&m, ergo_sandbox::map::LINT_ASYMMETRY);
    assert!(asym.contains(&(USE_SWAP.to_string(), USE_POOL.to_string())));
}

/// Extract's reference to the pool is NFT-bound — the discrimination that
/// makes the finding above a signal rather than a blanket warning.
#[test]
fn extract_pins_the_pool_by_nft() {
    let m = use_map();
    let e = edges_between(&m, USE_EXTRACT, USE_POOL);
    assert!(
        e.iter().any(|e| e.binding == Binding::Nft),
        "extract -> pool must type nft: {e:?}"
    );
    assert!(
        !e.iter().any(|e| e.binding == Binding::Positional),
        "extract enforces the pairing from its own side, so no positional edge"
    );
}

// ── criterion 3: DexyGold, the twin deployment ───────────────────────────────

#[test]
fn dexy_gold_raises_the_same_finding() {
    let m = dexy_map();
    let unbound: BTreeSet<String> = m
        .findings
        .iter()
        .filter(|f| f.finding.lint == ergo_sandbox::map::LINT_UNBOUND_SET)
        .filter_map(|f| match &f.to {
            Target::Node(t) => Some(t.clone()),
            Target::Unresolved(_) => None,
        })
        .collect();
    let lp_pool = m
        .nodes
        .values()
        .find(|n| n.nft.as_deref() == Some(DEXY_LP_NFT))
        .expect("the DexyGold LP pool box");
    assert_eq!(lp_pool.role, Role::Protected);
    assert!(
        unbound.contains(&lp_pool.chain_box.box_id),
        "the DexyGold LP pool must be the target of the set-level finding"
    );
    assert!(
        m.nodes.len() >= 20,
        "the twin deployment's set should come back too, got {}",
        m.nodes.len()
    );
}

// ── criterion 4: the gallery negative control ────────────────────────────────

fn param(t: &str, v: serde_json::Value) -> ergo_sandbox::TypedValue {
    ergo_sandbox::TypedValue {
        r#type: t.to_string(),
        value: v,
    }
}

fn compiled(source: &str, params: &BTreeMap<String, ergo_sandbox::TypedValue>) -> String {
    hex::encode(
        ergo_sandbox::compile::compile_with_params(source, params, 3, NetworkPrefix::Mainnet)
            .expect("compile gallery contract")
            .tree_bytes,
    )
}

fn chain_box(id: &str, tree: &str, value: u64, tokens: &[(&str, u64)]) -> ChainBox {
    ChainBox {
        box_id: id.to_string(),
        ergo_tree: tree.to_string(),
        value,
        tokens: tokens
            .iter()
            .map(|(id, amount)| ChainToken {
                id: (*id).to_string(),
                amount: *amount,
            })
            .collect(),
        creation_height: 1_000_000,
        inclusion_height: 1_000_000,
        registers: BTreeMap::new(),
    }
}

/// The gallery's correctly-written AMM pair: the order names the pool by its
/// NFT, so no site is left for a decoy. Compiled from source and mapped
/// through a synthetic fixture — no network, and no deployed box needed.
#[test]
fn the_gallery_amm_pair_maps_clean() {
    let pool_nft = "11".repeat(32);
    let lp_token = "22".repeat(32);
    let token_y = "33".repeat(32);
    let pool_box = "aa".repeat(32);
    let order_box = "bb".repeat(32);

    let mut pool_params = BTreeMap::new();
    pool_params.insert(
        "lpSupply".into(),
        param("Long", serde_json::json!(1_000_000)),
    );
    pool_params.insert("feeNum".into(), param("Int", serde_json::json!(997)));
    pool_params.insert("feeDenom".into(), param("Int", serde_json::json!(1000)));
    let pool_tree = compiled(
        include_str!("../../examples/contracts/protocols/amm/pool.es"),
        &pool_params,
    );

    let mut order_params = BTreeMap::new();
    order_params.insert(
        "trader".into(),
        param(
            "SigmaProp",
            serde_json::json!("9fRAWhdxEsTcdb8PhGNrZfwqa65zfkuYHAMmkQLcic1gdLSV5vA"),
        ),
    );
    order_params.insert(
        "poolNft".into(),
        param("Coll[Byte]", serde_json::json!(pool_nft)),
    );
    order_params.insert(
        "tokenY".into(),
        param("Coll[Byte]", serde_json::json!(token_y)),
    );
    order_params.insert("minOutput".into(), param("Long", serde_json::json!(1)));
    let order_tree = compiled(
        include_str!("../../examples/contracts/protocols/amm/swap-order.es"),
        &order_params,
    );

    let pool = chain_box(
        &pool_box,
        &pool_tree,
        500_000_000_000,
        &[(&pool_nft, 1), (&lp_token, 900_000), (&token_y, 400_000)],
    );
    let order = chain_box(&order_box, &order_tree, 2_000_000_000, &[]);

    let mut fx = Fixture::new("fixture", None, 1_000_000);
    fx.tokens.insert(
        pool_nft.clone(),
        Some(TokenInfo {
            id: pool_nft.clone(),
            emission_amount: 1,
        }),
    );
    fx.tokens.insert(
        lp_token.clone(),
        Some(TokenInfo {
            id: lp_token.clone(),
            emission_amount: 1_000_000,
        }),
    );
    fx.tokens.insert(
        token_y.clone(),
        Some(TokenInfo {
            id: token_y.clone(),
            emission_amount: 1_000_000,
        }),
    );
    let holders = |b: &ChainBox| Recorded {
        items: vec![b.clone()],
        total: Some(1),
    };
    fx.boxes_by_token.insert(pool_nft.clone(), holders(&pool));
    fx.boxes_by_token.insert(lp_token.clone(), holders(&pool));
    fx.boxes_by_token.insert(token_y.clone(), holders(&pool));
    // The order box carries no NFT, so the walk needs it handed in: seed the
    // map from the address the pair shares by listing both boxes there.
    fx.boxes_by_address.insert(
        "gallery-amm".into(),
        Recorded {
            items: vec![pool.clone(), order.clone()],
            total: Some(2),
        },
    );

    let m = map(
        &fx,
        &Seed::Address("gallery-amm".into()),
        &MapOptions::default(),
    )
    .expect("map the gallery pair");

    assert_eq!(m.nodes.len(), 2, "just the pool and the order");
    assert!(
        m.findings.is_empty(),
        "the gallery pair is the negative control: {:?}",
        m.findings
            .iter()
            .map(|f| (f.finding.lint, f.finding.snippet.clone()))
            .collect::<Vec<_>>()
    );
    // Every cross-contract edge is NFT-bound; the only other edge is the
    // pool's own successor, which is the shape a self-validating pool is
    // written in.
    for e in &m.edges {
        let cross = matches!(&e.to, Target::Node(t) if t != &e.from);
        if cross {
            assert_eq!(
                e.binding,
                Binding::Nft,
                "every cross-contract edge in the gallery pair is nft-bound: {e:?}"
            );
        } else {
            assert_eq!(e.binding, Binding::SelfSuccessor, "{e:?}");
        }
    }
}

// ── criterion 5: roles feed the hunt ─────────────────────────────────────────

/// The labelling the phase-1 drain hunt does by hand, proposed by the map.
#[test]
fn the_use_roles_are_the_labelling_the_hunt_needs() {
    let m = use_map();
    let role = |id: &str| m.nodes.get(id).expect("node in map").role;
    assert_eq!(
        role(USE_POOL),
        Role::Protected,
        "the pool holds the reserves"
    );
    assert_eq!(
        role(USE_SWAP),
        Role::Companion,
        "the swap is the authorisation"
    );
    assert_eq!(
        role(USE_ORACLE),
        Role::External,
        "the oracle is read, not spent"
    );
    for t in [USE_TRACKER_95, USE_TRACKER_98, USE_TRACKER_101] {
        assert_eq!(role(t), Role::External, "trackers are read as data inputs");
    }
}

/// `unknown` is a real answer, emitted rather than defaulted — the hunt
/// refuses to run on it, which is the point.
#[test]
fn unknown_is_emitted_not_defaulted() {
    let m = use_map();
    assert!(
        m.nodes.values().any(|n| n.role == Role::Unknown),
        "the neighbourhood contains boxes the rules do not settle"
    );
    let doc = json::canonical(&m);
    let roles: BTreeSet<&str> = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["role"].as_str().unwrap())
        .collect();
    assert!(roles.contains("unknown"));
}

// ── criterion 6: determinism and honesty ─────────────────────────────────────

/// Reverse every list in a fixture: the map's own canonical order must make
/// the output identical anyway.
fn shuffled(mut fx: Fixture) -> Fixture {
    for r in fx.boxes_by_token.values_mut() {
        r.items.reverse();
    }
    for r in fx.boxes_by_address.values_mut() {
        r.items.reverse();
    }
    for t in fx.transactions.values_mut() {
        t.inputs.reverse();
        t.outputs.reverse();
    }
    fx
}

#[test]
fn the_canonical_json_is_byte_identical_across_runs_and_orderings() {
    let seed = Seed::TokenId(USE_LP_NFT.into());
    let a = map(&load("use-lp.json"), &seed, &recorded_opts()).unwrap();
    let b = map(&shuffled(load("use-lp.json")), &seed, &recorded_opts()).unwrap();

    let ja = serde_json::to_string(&json::canonical(&a)).unwrap();
    let jb = serde_json::to_string(&json::canonical(&b)).unwrap();
    assert_eq!(
        ja, jb,
        "canonical JSON must not depend on response ordering"
    );

    // The non-canonical `run` block is the only thing that may differ, and it
    // is absent from the canonical document.
    assert!(
        !ja.contains("fetchedAt"),
        "no wall clock in the canonical artifact"
    );
    assert!(!ja.contains("durationMs"));
    let full = serde_json::to_string(&json::document(&a)).unwrap();
    assert!(full.contains("fetchedAt") && full.contains("durationMs"));
}

/// Every array in the canonical document is sorted by its canonical key.
#[test]
fn every_array_is_canonically_sorted() {
    let m = use_map();
    let doc = json::canonical(&m);

    let node_ids: Vec<&str> = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["boxId"].as_str().unwrap())
        .collect();
    let mut sorted = node_ids.clone();
    sorted.sort_unstable();
    assert_eq!(node_ids, sorted, "nodes sort by boxId");

    for n in doc["nodes"].as_array().unwrap() {
        let ids: Vec<&str> = n["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap())
            .collect();
        let mut s = ids.clone();
        s.sort_unstable();
        assert_eq!(ids, s, "a node's tokens sort by token id");
    }

    let keys: Vec<(String, String, String, String)> = doc["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["from"]["boxId"].as_str().unwrap_or_default().to_string(),
                e["to"]["boxId"]
                    .as_str()
                    .map(|s| format!("0{s}"))
                    .unwrap_or_else(|| format!("1{}", e["to"]["unresolved"].as_str().unwrap())),
                e["binding"].as_str().unwrap().to_string(),
                e["site"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mut s = keys.clone();
    s.sort();
    assert_eq!(keys, s, "edges sort by (from, to, binding, site)");
}

/// Truncation is reported — per token id, not only overall — and unresolved
/// references are kept in the output rather than vanishing.
#[test]
fn truncation_and_unresolved_references_are_reported() {
    let m = use_map();
    assert!(
        !m.truncated.per_token.is_empty(),
        "the fungible tokens the set references have more holders than the cap admits"
    );
    let doc = json::canonical(&m);
    assert!(doc["truncated"]["perToken"].is_object());

    assert!(
        m.edges
            .iter()
            .any(|e| matches!(e.to, Target::Unresolved(_))),
        "a site with no constant to follow stays in the output as unresolved"
    );

    // A tighter node cap must be reported, never silently applied.
    let tight = MapOptions {
        max_nodes: 3,
        ..recorded_opts()
    };
    let small = map(
        &load("use-lp.json"),
        &Seed::TokenId(USE_LP_NFT.into()),
        &tight,
    )
    .unwrap();
    assert_eq!(small.nodes.len(), 3);
    assert!(small.truncated.nodes > 0, "the node cap must be reported");
    assert!(!json::canonical(&small)["truncated"].is_null());
}

/// A fixture gap is a loud error, never an empty answer: "not recorded" must
/// not read as "the chain has nothing there".
#[test]
fn a_fixture_gap_is_loud() {
    let mut fx = load("use-lp.json");
    fx.tokens.clear();
    let err = map(&fx, &Seed::TokenId(USE_LP_NFT.into()), &recorded_opts()).unwrap_err();
    assert!(
        err.to_string().contains("no recorded answer"),
        "unexpected error: {err}"
    );
}

/// The explorer has no `blake2b256(propositionBytes)` index, so a script-hash
/// reference it cannot resolve is an `unresolved` edge — kept, and excluded
/// from any completeness claim.
#[test]
fn script_hash_lookups_are_unsupported_not_empty() {
    use ergo_sandbox::map::source::{ChainSource, SourceError};
    let fx = load("use-lp.json");
    assert!(matches!(
        fx.boxes_by_script_hash(&"ab".repeat(32), 0, 8),
        Err(SourceError::Unsupported(_))
    ));
}
