use ergo_sandbox::{
    lockfile::{self, LiveStatus},
    map::{fixture::Fixture, source::*},
    watch::{self, RegisterStatus, WatchInput},
};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use std::{cell::RefCell, collections::BTreeMap};
const NFT: &str = "1111111111111111111111111111111111111111111111111111111111111111";
fn fixture() -> Fixture {
    Fixture::from_json(include_str!("fixtures/watch/holder.fixture")).unwrap()
}
fn input() -> WatchInput {
    let b = &fixture().boxes_by_token[NFT].items[0];
    let params =
        serde_json::from_value(json!({"owner": {"type": "SigmaProp", "value": &b.ergo_tree[6..]}}))
            .unwrap();
    let lockfile = lockfile::create("$owner", &params, 3, NetworkPrefix::Mainnet).unwrap();
    assert_eq!(lockfile.tree_hex, b.ergo_tree);
    WatchInput {
        lockfile,
        nfts: vec![NFT.into()],
        registers: vec!["R4".into(), "R5".into()],
        baseline_boxes: BTreeMap::new(),
    }
}
fn run(input: &WatchInput, source: Option<&dyn ChainSource>) -> watch::WatchReport {
    watch::observe(std::slice::from_ref(input), source)
        .unwrap()
        .remove(0)
}

#[test]
fn watch_reports_script_change_under_nft() {
    let mut input = input();
    let mut f = fixture();
    let first = run(&input, Some(&f));
    assert_eq!(first.live.status, LiveStatus::BytesMatch);
    assert_eq!(
        first.live.box_id.as_deref(),
        Some("2222222222222222222222222222222222222222222222222222222222222222")
    );
    assert_eq!(first.height, Some(321));
    assert_eq!(first.chain_source.kind, "fixture");
    assert_eq!(
        first.chain_source.url.as_deref(),
        Some("https://watch-fixture.invalid")
    );
    assert_eq!(first.limitation, ergo_sandbox::identity::LIMITATION);
    assert!(first.observation.contains("source's response"));
    assert!(first.observation.contains("not independently verified"));
    assert_eq!(first.baseline_origin, "first_observation");
    assert!(first
        .registers
        .iter()
        .all(|r| r.status == RegisterStatus::Unchanged));
    assert!(first.registers[0].reason.contains("First observation"));
    assert_eq!(first.registers[0].current.as_deref(), Some("040e"));
    assert_eq!(watch::exit_code(std::slice::from_ref(&first)), 0);
    assert_eq!(
        serde_json::to_value(&first).unwrap()["live"]["nodeValidated"],
        false
    );
    input
        .baseline_boxes
        .insert(NFT.into(), first.baseline_box.unwrap());
    let b = &mut f.boxes_by_token.get_mut(NFT).unwrap().items[0];
    b.ergo_tree = "0008d3".into();
    b.registers.insert("R4".into(), "0410".into());
    b.registers.insert("R5".into(), "0E02ABCD".into());
    let changed = run(&input, Some(&f));
    assert_eq!(changed.live.status, LiveStatus::BytesDiffer);
    assert_eq!(changed.registers[0].status, RegisterStatus::Changed);
    assert_eq!(changed.registers[0].expected.as_deref(), Some("040e"));
    assert_eq!(changed.registers[1].status, RegisterStatus::Unchanged);
    assert_eq!(changed.baseline_origin, "supplied");
    assert_eq!(watch::exit_code(std::slice::from_ref(&changed)), 3);
    assert_eq!(changed.lockfile_fingerprint, first.lockfile_fingerprint);
    f.boxes_by_token.get_mut(NFT).unwrap().items[0].ergo_tree = input.lockfile.tree_hex.clone();
    let registers_only = run(&input, Some(&f));
    assert_eq!(registers_only.live.status, LiveStatus::BytesMatch);
    assert_eq!(watch::exit_code(&[registers_only]), 3);
    let absent = run(&input, None);
    assert_eq!(absent.live.status, LiveStatus::Unverified);
    assert_eq!(absent.live.reason, watch::NO_EXPLORER);
    assert!(absent
        .registers
        .iter()
        .all(|r| r.status == RegisterStatus::Unverified && r.reason == watch::NO_EXPLORER));
    assert_eq!(absent.height, None);
    assert_eq!(absent.chain_source.kind, "none");
    assert_eq!(absent.chain_source.url, None);
    assert_eq!(watch::exit_code(&[changed, absent]), 4);
}

struct Recording {
    fixture: Fixture,
    calls: RefCell<Vec<String>>,
    height_error: bool,
}
impl Recording {
    fn new() -> Self {
        Self {
            fixture: fixture(),
            calls: RefCell::new(vec![]),
            height_error: false,
        }
    }
    fn record(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }
}
impl ChainSource for Recording {
    fn kind(&self) -> &str {
        self.fixture.kind()
    }
    fn url(&self) -> Option<&str> {
        self.fixture.url()
    }
    fn height(&self) -> Result<u32, SourceError> {
        self.record("height".into());
        if self.height_error {
            Err(SourceError::Backend("missing height".into()))
        } else {
            self.fixture.height()
        }
    }
    fn token_info(&self, id: &str) -> Result<TokenInfo, SourceError> {
        self.record(format!("token_info {id}"));
        self.fixture.token_info(id)
    }
    fn boxes_by_token_id(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        self.record(format!("boxes_by_token_id {id} {offset} {limit}"));
        assert_eq!((offset, limit), (0, 2));
        self.fixture.boxes_by_token_id(id, offset, limit)
    }
    fn box_by_id(&self, id: &str) -> Result<ChainBox, SourceError> {
        self.record(format!("box_by_id {id}"));
        self.fixture.box_by_id(id)
    }
    fn boxes_by_address(&self, _: &str, _: usize, _: usize) -> Result<Page, SourceError> {
        panic!("unexpected address read")
    }
    fn transaction(&self, _: &str) -> Result<TxBoxes, SourceError> {
        panic!("unexpected transaction read")
    }
}

#[test]
fn watch_never_broadcasts() {
    let source = Recording::new();
    let input = input();
    run(&input, Some(&source));
    assert_eq!(
        *source.calls.borrow(),
        [
            "height".to_string(),
            format!("token_info {NFT}"),
            format!("boxes_by_token_id {NFT} 0 2")
        ]
    );
    // Production imports and calls must stay on the read/comparison path. The
    // trait has no write method; the recording rejects all unrelated reads.
    for source in [
        include_str!("../src/watch.rs"),
        include_str!("../../ergo-web/src/routes/watch.rs"),
        include_str!("../src/bin/watching/mod.rs"),
    ] {
        for forbidden in [
            "::prove",
            "prove(",
            "txcheck",
            "submit(",
            "broadcast(",
            "sign(",
            "ergo_wallet",
            "crate::play",
            "crate::replay",
            "crate::compile",
            "reqwest",
            "ureq",
            "DEFAULT_EXPLORER_URL",
        ] {
            assert!(!source.contains(forbidden), "watch references {forbidden}");
        }
    }
    let route = include_str!("../../ergo-web/src/routes/watch.rs");
    assert!(route.contains("state.cfg.explorer_url"));
    assert!(route.contains("watch::observe"));
    assert!(include_str!("../src/watch.rs").contains("lockfile::compare_live("));
}

#[test]
fn watch_gaps_ambiguity_and_malformed_responses_remain_unverified() {
    let input = input();
    for case in 0..11 {
        let mut f = fixture();
        match case {
            0 => f.boxes_by_token.get_mut(NFT).unwrap().items.clear(),
            1 => {
                let b = f.boxes_by_token[NFT].items[0].clone();
                f.boxes_by_token.get_mut(NFT).unwrap().items.push(b);
            }
            2 => f.boxes_by_token.get_mut(NFT).unwrap().total = Some(9),
            3 => {
                f.tokens.clear();
            }
            4 => {
                f.tokens
                    .get_mut(NFT)
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .emission_amount = 2
            }
            5 => f.tokens.get_mut(NFT).unwrap().as_mut().unwrap().id = "33".repeat(32),
            6 => {
                f.boxes_by_token.clear();
            }
            7 => f.boxes_by_token.get_mut(NFT).unwrap().items[0].tokens[0].amount = 2,
            8 => f.boxes_by_token.get_mut(NFT).unwrap().items[0].box_id = "bad".into(),
            9 => f.boxes_by_token.get_mut(NFT).unwrap().items[0].ergo_tree = "bad hex".into(),
            10 => {
                let t = f.boxes_by_token[NFT].items[0].tokens[0].clone();
                f.boxes_by_token.get_mut(NFT).unwrap().items[0]
                    .tokens
                    .push(t);
            }
            _ => unreachable!(),
        }
        let report = run(&input, Some(&f));
        assert_eq!(report.live.status, LiveStatus::Unverified, "case {case}");
        assert!(report
            .registers
            .iter()
            .all(|r| r.status == RegisterStatus::Unverified));
        assert!(report.baseline_box.is_none());
        assert_eq!(watch::exit_code(&[report]), 4);
    }
    let mut source = Recording::new();
    source.height_error = true;
    let report = run(&input, Some(&source));
    assert_eq!(report.live.status, LiveStatus::Unverified);
    assert!(report.live.reason.contains("Source height unavailable"));
    assert_eq!(*source.calls.borrow(), ["height"]);
    for value in [None, Some(""), Some("xyz")] {
        let mut f = fixture();
        let regs = &mut f.boxes_by_token.get_mut(NFT).unwrap().items[0].registers;
        if let Some(value) = value {
            regs.insert("R4".into(), value.into());
        } else {
            regs.remove("R4");
        }
        let report = run(&input, Some(&f));
        assert_eq!(report.live.status, LiveStatus::BytesMatch);
        assert_eq!(report.registers[0].status, RegisterStatus::Unverified);
        assert!(report.baseline_box.is_none());
        assert_eq!(watch::exit_code(&[report]), 4);
    }
    let mut input = input;
    let mut baseline = fixture().boxes_by_token[NFT].items[0].clone();
    baseline.registers.clear();
    input.baseline_boxes.insert(NFT.into(), baseline);
    let report = run(&input, Some(&fixture()));
    assert!(report
        .registers
        .iter()
        .all(|r| r.status == RegisterStatus::Unverified));
}

#[test]
fn watch_batch_validation_fingerprints_and_baselines_are_per_lock_and_nft() {
    let input = input();
    let mut second = input.clone();
    second.lockfile.tree_hex = "0008d3".into();
    second.registers.clear();
    let reports = watch::observe(&[input.clone(), second], Some(&fixture())).unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].live.status, LiveStatus::BytesMatch);
    assert_eq!(reports[1].live.status, LiveStatus::BytesDiffer);
    assert_ne!(
        reports[0].lockfile_fingerprint,
        reports[1].lockfile_fingerprint
    );
    assert!(reports[1].registers.is_empty());
    let roundtrip =
        serde_json::from_str(&serde_json::to_string_pretty(&input.lockfile).unwrap()).unwrap();
    assert_eq!(
        watch::fingerprint(&input.lockfile),
        watch::fingerprint(&roundtrip)
    );
    for case in 0..8 {
        let mut invalid = input.clone();
        match case {
            0 => invalid.nfts.clear(),
            1 => invalid.nfts = vec!["bad".into()],
            2 => invalid.nfts.push(NFT.into()),
            3 => invalid.registers = vec!["R3".into()],
            4 => invalid.registers.push("R4".into()),
            5 => invalid.lockfile.schema_version = 2,
            6 => invalid.lockfile.tree_hex = "".into(),
            7 => {
                invalid.baseline_boxes.insert(
                    "wrong".into(),
                    fixture().boxes_by_token[NFT].items[0].clone(),
                );
            }
            _ => unreachable!(),
        }
        let source = Recording::new();
        assert!(watch::observe(&[invalid], Some(&source)).is_err());
        assert!(source.calls.borrow().is_empty());
    }
    assert!(watch::observe(&[], None).is_err());
    assert!(watch::observe(&vec![input; watch::MAX_REPORTS + 1], None).is_err());
}

#[test]
fn watch_cli_offline_and_input_exit_codes() {
    let dir = std::env::temp_dir().join(format!("forge-watch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = input();
    std::fs::write(
        dir.join("contract.lock.json"),
        serde_json::to_vec(&input.lockfile).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("box.json"),
        serde_json::to_vec(&fixture().boxes_by_token[NFT].items[0]).unwrap(),
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
    let result = run(&[
        "watch",
        "contract.lock.json",
        "contract.lock.json",
        "--nft",
        NFT,
        "--register",
        "R4",
        "R5",
        "--baseline",
        "box.json",
        "--json",
    ]);
    assert_eq!(
        result.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let reports: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(reports.as_array().unwrap().len(), 2);
    assert_eq!(reports[0]["live"]["reason"], watch::NO_EXPLORER);
    assert_eq!(reports[0]["registers"].as_array().unwrap().len(), 2);
    assert_eq!(reports[0]["baselineOrigin"], "supplied");
    for args in [
        vec!["watch"],
        vec!["watch", "contract.lock.json"],
        vec!["watch", "contract.lock.json", "--nft"],
        vec!["watch", "contract.lock.json", "--nft", "bad"],
        vec![
            "watch",
            "contract.lock.json",
            "--nft",
            NFT,
            "--register",
            "R10",
        ],
        vec!["watch", "contract.lock.json", "--nft", NFT, "--unknown"],
        vec!["watch", "missing", "--nft", NFT],
        vec!["watch", "box.json", "--nft", NFT],
    ] {
        assert_eq!(run(&args).status.code(), Some(1), "{args:?}");
    }
    let help = run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&help.stdout)
        .contains("4 any unverified (takes precedence over 3); 1 input error"));
    let text = run(&["watch", "contract.lock.json", "--nft", NFT]);
    assert_eq!(text.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&text.stdout).contains(watch::OBSERVATION));
    std::fs::remove_dir_all(dir).unwrap();
}
