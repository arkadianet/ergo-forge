#[path = "engine_support/request.rs"]
mod engine_support;
use ergo_sandbox::{
    lockfile::{self, DriftKind, LiveStatus, Lockfile},
    map::{
        fixture::{Fixture, Recorded},
        source::{ChainBox, ChainToken, TokenInfo},
    },
    TypedValue,
};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const SOURCE: &str = "// exact text: λ\nsigmaProp(HEIGHT > $h && SELF.R4[Long].get > 200L)\n";
fn params() -> BTreeMap<String, TypedValue> {
    serde_json::from_value(json!({"h":{"type":"Int","value":100}})).unwrap()
}
fn create() -> Lockfile {
    lockfile::create(SOURCE, &params(), 3, NetworkPrefix::Mainnet).unwrap()
}

#[test]
fn lockfile_detects_source_param_and_engine_drift() {
    let lock = create();
    assert_eq!(
        lock.source_sha256,
        hex::encode(Sha256::digest(SOURCE.as_bytes()))
    );
    assert_eq!(
        lock.compiler_revision,
        ergo_sandbox::evidence::case::engine_revision()
    );
    assert_eq!(lock.node_revision, lock.compiler_revision);
    assert!(
        lockfile::verify_lock(&lock, SOURCE, &params(), 3, NetworkPrefix::Mainnet)
            .unwrap()
            .drifts
            .is_empty()
    );
    // Each field is independently changed, including all metadata. Expected
    // values come from external source/params/options, never the altered lock.
    let changes = [
        ("schemaVersion", json!(2), DriftKind::SchemaVersion),
        ("sourceSha256", json!("00".repeat(32)), DriftKind::Source),
        (
            "params",
            json!({"h":{"type":"Int","value":101}}),
            DriftKind::Params,
        ),
        (
            "compilerRevision",
            json!(engine_support::fixture_revision()),
            DriftKind::CompilerRevision,
        ),
        (
            "nodeRevision",
            json!(engine_support::fixture_revision()),
            DriftKind::NodeRevision,
        ),
        (
            "workbenchVersion",
            json!("0.0.0"),
            DriftKind::WorkbenchVersion,
        ),
        ("treeHex", json!("0008d3"), DriftKind::TreeBytes),
        // A valid address on the other network is address drift too.
        (
            "address",
            json!(
                lockfile::create(SOURCE, &params(), 3, NetworkPrefix::Testnet)
                    .unwrap()
                    .address
            ),
            DriftKind::Address,
        ),
        ("treeVersion", json!(1), DriftKind::TreeVersion),
        (
            "lintSweepDigest",
            json!("ff".repeat(32)),
            DriftKind::LintDigest,
        ),
        ("limitation", json!("omitted"), DriftKind::Limitation),
    ];
    let serialized = serde_json::to_value(&lock).unwrap();
    assert_eq!(
        serialized.as_object().unwrap().len(),
        changes.len(),
        "new fields need independent drift controls"
    );
    for (field, value, expected) in changes {
        let mut altered = serialized.clone();
        altered[field] = value;
        let altered: Lockfile = serde_json::from_value(altered).unwrap();
        let r =
            lockfile::verify_lock(&altered, SOURCE, &params(), 3, NetworkPrefix::Mainnet).unwrap();
        assert_eq!(
            r.drifts.iter().map(|d| d.kind).collect::<Vec<_>>(),
            [expected],
            "{field}"
        );
        assert_eq!(r.drifts[0].field, field);
        assert_eq!(r.limitation, ergo_sandbox::identity::LIMITATION);
        assert_eq!(r.live.status, LiveStatus::Unverified);
        assert_eq!(serde_json::to_value(r).unwrap()["nodeValidated"], false);
    }
    let newline_drift = lockfile::verify_lock(
        &lock,
        &SOURCE.replace('\n', "\r\n"),
        &params(),
        3,
        NetworkPrefix::Mainnet,
    )
    .unwrap();
    assert_eq!(
        newline_drift
            .drifts
            .iter()
            .map(|d| d.kind)
            .collect::<Vec<_>>(),
        [DriftKind::Source]
    );
    let mut changed_params = params();
    changed_params.get_mut("h").unwrap().value = json!(101);
    let r =
        lockfile::verify_lock(&lock, SOURCE, &changed_params, 3, NetworkPrefix::Mainnet).unwrap();
    assert_eq!(
        r.drifts.iter().map(|d| d.kind).collect::<Vec<_>>(),
        [DriftKind::Params, DriftKind::TreeBytes, DriftKind::Address]
    );
}

#[test]
fn lint_digest_uses_complete_sorted_static_projection() {
    let lock = create();
    let tree = ergo_sandbox::inspect::parse_tree(&hex::decode(&lock.tree_hex).unwrap()).unwrap();
    let audit = ergo_sandbox::audit::audit(&ergo_sandbox::lift_tree(&tree, false));
    assert!(!audit.findings.is_empty());
    let mut expected: Vec<_> = audit
        .findings
        .iter()
        .map(|f| (f.lint.to_string(), f.node_id, f.message.clone()))
        .collect();
    expected.reverse();
    expected.sort();
    assert_eq!(
        lock.lint_sweep_digest,
        hex::encode(Sha256::digest(serde_json::to_vec(&expected).unwrap()))
    );
    assert_eq!(
        lock.lint_sweep_digest,
        lockfile::create(SOURCE, &params(), 3, NetworkPrefix::Testnet)
            .unwrap()
            .lint_sweep_digest
    );
    let empty = lockfile::create(
        "proveDlog(decodePoint(fromBase16(\"03eba1dbe56504389ed03b5d63f052ba88d2d9951443b0dd2c51d7b14806f76961\")))",
        &BTreeMap::new(),
        3,
        NetworkPrefix::Mainnet,
    )
    .unwrap();
    assert_eq!(empty.lint_sweep_digest, hex::encode(Sha256::digest(b"[]")));
    assert_eq!(
        serde_json::to_value(&lock).unwrap(),
        serde_json::to_value(create()).unwrap()
    );
}

fn recorded_box() -> ChainBox {
    // Actual explorer transaction recording. The first input has a P2PK tree
    // retained verbatim as a historical byte-comparison reference.
    let archive: Value = serde_json::from_str(include_str!(
        "fixtures/evidence/claim-vectors/use-archive.fixture"
    ))
    .unwrap();
    let b = &archive["inputs"][0];
    ChainBox {
        box_id: b["boxId"].as_str().unwrap().into(),
        ergo_tree: b["ergoTree"].as_str().unwrap().into(),
        value: b["value"].as_u64().unwrap(),
        tokens: b["assets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| ChainToken {
                id: t["tokenId"].as_str().unwrap().into(),
                amount: t["amount"].as_u64().unwrap(),
            })
            .collect(),
        registers: BTreeMap::new(),
        creation_height: 0,
        inclusion_height: 0,
    }
}

#[test]
fn lockfile_matches_live_box_bytes() {
    let b = recorded_box();
    // Binding the recorded public key as a SigmaProp lets the pinned engine
    // emit the same bare P2PK constant. No lock field is replaced to make the
    // positive control match: this is a freshly generated compilation lock.
    let source = "$owner";
    let params =
        serde_json::from_value(json!({"owner": {"type": "SigmaProp", "value": &b.ergo_tree[6..]}}))
            .unwrap();
    let recorded_lock = lockfile::create(source, &params, 3, NetworkPrefix::Mainnet).unwrap();
    assert_eq!(recorded_lock.tree_hex, b.ergo_tree);
    assert!(
        lockfile::verify_lock(&recorded_lock, source, &params, 3, NetworkPrefix::Mainnet)
            .unwrap()
            .drifts
            .is_empty()
    );
    let r = lockfile::compare_live_box(&recorded_lock, &b);
    assert_eq!(r.status, LiveStatus::BytesMatch);
    assert_eq!(r.observed_tree_hex.as_deref(), Some(b.ergo_tree.as_str()));
    assert_eq!(r.box_id.as_deref(), Some(b.box_id.as_str()));
    assert_eq!(r.limitation, ergo_sandbox::identity::LIMITATION);
    let nft = &b.tokens.iter().find(|t| t.amount == 1).unwrap().id;
    let mut fixture = Fixture::new("recorded-use-archive", None, 1868204);
    fixture.tokens.insert(
        nft.clone(),
        Some(TokenInfo {
            id: nft.clone(),
            emission_amount: 1,
        }),
    );
    fixture.boxes_by_token.insert(
        nft.clone(),
        Recorded {
            items: vec![b.clone()],
            total: Some(1),
        },
    );
    // Same lookup and byte-comparison functions as the CLI explorer path.
    assert_eq!(
        lockfile::compare_live(&recorded_lock, Some(nft), Some(&fixture)).status,
        LiveStatus::BytesMatch
    );
    let changed = lockfile::create(
        "sigmaProp(false)",
        &BTreeMap::new(),
        3,
        NetworkPrefix::Mainnet,
    )
    .unwrap();
    assert_eq!(
        lockfile::compare_live(&changed, Some(nft), Some(&fixture)).status,
        LiveStatus::BytesDiffer
    );
    assert_eq!(
        lockfile::compare_live(&recorded_lock, Some(nft), None).status,
        LiveStatus::Unverified
    );
    assert_eq!(
        lockfile::compare_live(&recorded_lock, None, Some(&fixture)).status,
        LiveStatus::Unverified
    );
    fixture
        .boxes_by_token
        .get_mut(nft)
        .unwrap()
        .items
        .push(b.clone());
    assert_eq!(
        lockfile::compare_live(&recorded_lock, Some(nft), Some(&fixture)).status,
        LiveStatus::Unverified
    );
    fixture.boxes_by_token.get_mut(nft).unwrap().items.clear();
    assert_eq!(
        lockfile::compare_live(&recorded_lock, Some(nft), Some(&fixture)).status,
        LiveStatus::Unverified
    );
    let mut malformed = b;
    malformed.ergo_tree = "not hex".into();
    assert_eq!(
        lockfile::compare_live_box(&recorded_lock, &malformed).status,
        LiveStatus::Unverified
    );
}

#[test]
fn lock_cli_roundtrip_drift_and_offline_live_status() {
    let dir = std::env::temp_dir().join(format!("forge-lock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("contract.es"), SOURCE).unwrap();
    std::fs::write(
        dir.join("params.json"),
        serde_json::to_vec(&params()).unwrap(),
    )
    .unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_ergo-es"))
            .current_dir(&dir)
            .env_remove("EXPLORER_URL")
            .args(args)
            .output()
            .unwrap()
    };
    let r = run(&[
        "lock",
        "contract.es",
        "--params",
        "params.json",
        "--network",
        "testnet",
    ]);
    assert_eq!(
        r.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&r.stderr)
    );
    assert!(String::from_utf8_lossy(&r.stdout).contains(ergo_sandbox::identity::LIMITATION));
    let args = [
        "verify-lock",
        "contract.lock.json",
        "--source",
        "contract.es",
        "--params",
        "params.json",
        "--network",
        "testnet",
        "--json",
    ];
    let r = run(&args);
    assert_eq!(r.status.code(), Some(0));
    let result: Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(result["live"]["status"], "unverified");
    assert_eq!(result["status"], "current");
    let mut live_args = args.to_vec();
    let nft = "11".repeat(32);
    live_args.extend(["--nft", &nft]);
    let r = run(&live_args);
    assert_eq!(r.status.code(), Some(6));
    assert_eq!(
        serde_json::from_slice::<Value>(&r.stdout).unwrap()["live"]["status"],
        "unverified"
    );
    let mut lock: Value =
        serde_json::from_slice(&std::fs::read(dir.join("contract.lock.json")).unwrap()).unwrap();
    lock["nodeRevision"] = json!("00".repeat(20));
    std::fs::write(
        dir.join("contract.lock.json"),
        serde_json::to_vec(&lock).unwrap(),
    )
    .unwrap();
    let r = run(&args);
    assert_eq!(r.status.code(), Some(5));
    let result: Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(result["drifts"].as_array().unwrap().len(), 1);
    assert_eq!(result["drifts"][0]["kind"], "node_revision");
    let r = run(&["verify-lock", "contract.lock.json", "--source"]);
    assert_eq!(r.status.code(), Some(1));
    std::fs::remove_dir_all(dir).unwrap();
}
