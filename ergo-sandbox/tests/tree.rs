use ergo_primitives::digest::blake2b256;
use ergo_sandbox::{
    map::Fixture,
    tree::{decode_address, ingest_tree},
};
use ergo_ser::address::{encode_p2s, NetworkPrefix};
use serde_json::{json, Value};
use std::process::Command;

// Published mainnet vectors: sigma-rust ergotree-ir/src/chain/address.rs.
// https://github.com/ergoplatform/sigma-rust/blob/v0.28.0/ergotree-ir/src/chain/address.rs
const P2S: &str = "4MQyML64GnzMxZgm";
const TRUE_TREE: &str = "10010101d17300";
const P2PK: &str = "9fRAWhdxEsTcdb8PhGNrZfwqa65zfkuYHAMmkQLcic1gdLSV5vA";
const PK_TREE: &str = "0008cd02764ea2b0b9b06b5730a4257bba71fd7797eb1ec12bc3ae6025a01d7fba53830e";
const P2SH: &str = "8UApt8czfFVuTgQmMwtsRBZ4nfWquNiSwCWUjMg";
// Independent Scala serialized box vector (tree between value and height):
// https://gist.github.com/kushti/0eecded525f0c9778fd93858291fa770
const SH_TREE: &str =
    "00ea02d193b4cbe4e3010e040004300e18d62151f990f191c102a6fe995b89ed3d0f343a96f13789a3d40801";

#[test]
fn published_vectors_and_exact_roundtrips_both_networks() {
    for (address, expected) in [(P2S, TRUE_TREE), (P2PK, PK_TREE), (P2SH, SH_TREE)] {
        let report = ingest_tree(address, None, None).unwrap();
        assert_eq!(report.tree_hex, expected);
        assert_eq!(report.network, "mainnet");
        for network in [NetworkPrefix::Mainnet, NetworkPrefix::Testnet] {
            let from_tree = ingest_tree(expected, Some(network), None).unwrap();
            for address in [
                Some(from_tree.addresses.p2s),
                from_tree.addresses.p2pk,
                from_tree.addresses.p2sh,
            ]
            .into_iter()
            .flatten()
            {
                let (bytes, actual_network) = decode_address(&address, Some(network)).unwrap();
                assert_eq!(hex::encode(bytes), expected);
                assert_eq!(network, actual_network);
            }
        }
    }
    assert_eq!(
        ingest_tree(P2PK, None, None)
            .unwrap()
            .addresses
            .p2pk
            .as_deref(),
        Some(P2PK)
    );
    let sh = ingest_tree(P2SH, None, None).unwrap();
    assert_eq!(sh.addresses.p2sh.as_deref(), Some(P2SH));
    assert!(!sh.notes.is_empty());
}

fn checked_address(header: u8, content: &[u8]) -> String {
    let mut raw = vec![header];
    raw.extend_from_slice(content);
    let hash = blake2b256(&raw);
    raw.extend_from_slice(&hash.as_bytes()[..4]);
    bs58::encode(raw).into_string()
}

#[test]
fn rejects_corruption_networks_types_lengths_and_malformed_trees() {
    assert!(decode_address("4MQyML64GnzMxZgn", None)
        .unwrap_err()
        .contains("checksum"));
    assert!(decode_address(P2S, Some(NetworkPrefix::Testnet))
        .unwrap_err()
        .contains("network mismatch"));
    for header in [0x23, 0x83, 0xf3] {
        assert!(decode_address(&checked_address(header, &[0]), None)
            .unwrap_err()
            .contains("network prefix"));
    }
    for (header, bytes) in [
        (0x04, vec![0]),
        (0x02, vec![0; 23]),
        (0x01, vec![0; 32]),
        (0x01, vec![0; 33]),
    ] {
        assert!(decode_address(&checked_address(header, &bytes), None).is_err());
    }
    for input in [
        "",
        "0OIl",
        "111",
        "tree:ff",
        "tree:10010101d1730000",
        "box:ab",
    ] {
        assert!(ingest_tree(input, None, None).is_err(), "{input}");
    }
    assert!(ingest_tree(&encode_p2s(NetworkPrefix::Mainnet, &[0xff]), None, None).is_err());
}

#[test]
fn fixture_box_preserves_all_data_and_addresses_never_query_source() {
    let id = "ab".repeat(32);
    let mut fixture = Fixture::new("test", None, 123);
    let b = serde_json::from_value(json!({"boxId": id, "ergoTree": TRUE_TREE, "value": 1234567,
        "tokens": [{"id": "cd".repeat(32), "amount": 42}],
        "additionalRegisters": {"R4": "040e", "R5": "050e"}, "creationHeight": 100}))
    .unwrap();
    fixture.boxes.insert(id.clone(), b);
    let report = ingest_tree(
        &id.to_uppercase(),
        Some(NetworkPrefix::Testnet),
        Some(&fixture),
    )
    .unwrap();
    assert_eq!(report.box_data.as_ref(), fixture.boxes.get(&id));
    let serialized = serde_json::to_value(&report).unwrap();
    assert_eq!(serialized["boxData"]["additionalRegisters"]["R4"], "040e");
    assert_eq!(serialized["network"], "testnet");
    assert!(ingest_tree(&id, None, None)
        .unwrap_err()
        .contains("requires"));
    assert!(ingest_tree(&"cd".repeat(32), None, Some(&fixture)).is_err());
    fixture.boxes.clear(); // Any chain query against this empty fixture fails loudly.
    assert!(ingest_tree(P2S, None, Some(&fixture)).is_ok());
}

#[test]
fn findings_equal_existing_audit() {
    let compiled =
        ergo_sandbox::compile_source("sigmaProp(SELF.R4[Int].get > 0)", 0, NetworkPrefix::Mainnet)
            .unwrap();
    let report = ingest_tree(&compiled.p2s_address, None, None).unwrap();
    let tree = ergo_sandbox::inspect::parse_tree(&compiled.tree_bytes).unwrap();
    let existing = ergo_sandbox::audit::audit(&ergo_sandbox::lift_tree(&tree, false));
    assert!(!existing.findings.is_empty());
    let findings = report.audit["findings"].as_array().unwrap();
    assert_eq!(findings.len(), existing.findings.len());
    for (actual, expected) in findings.iter().zip(existing.findings) {
        assert_eq!(actual["lint"], expected.lint);
        assert_eq!(actual["message"], expected.message);
        assert_eq!(actual["severity"], expected.severity.label());
    }
}

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(args)
        .env_remove("EXPLORER_URL")
        .output()
        .unwrap()
}

#[test]
fn cli_json_is_stable_and_network_is_explicit() {
    let output = cli(&["tree", P2S, "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["formatVersion"], 1);
    assert_eq!(value["treeHex"], TRUE_TREE);
    assert_eq!(value["addresses"]["p2s"], P2S);
    assert!(value["boxData"].is_null());
    assert_eq!(output.stdout, cli(&["tree", P2S, "--json"]).stdout);
    assert!(!cli(&["tree", &"ab".repeat(32)]).status.success());
    assert!(!cli(&["tree", P2S, "--network", "testnet"]).status.success());
    assert!(!cli(&["tree", P2S, "--typo"]).status.success());
    let offline = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(["tree", P2S, "--json"])
        .env("EXPLORER_URL", "http://127.0.0.1:1")
        .output()
        .unwrap();
    assert!(offline.status.success());
}

#[cfg(feature = "explorer")]
#[test]
fn cli_explorer_fetches_box_and_preserves_registers() {
    use std::io::{Read, Write};
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.local_addr().unwrap());
    let id = "ab".repeat(32);
    let expected_id = id.clone();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = server.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut buffer = [0u8; 4096];
        let n = stream.read(&mut buffer).unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n])
            .starts_with(&format!("GET /api/v1/boxes/{expected_id} ")));
        let body = json!({"boxId": expected_id, "ergoTree": TRUE_TREE, "value": 1000000,
            "assets": [{"tokenId": "cd".repeat(32), "amount": 7}],
            "additionalRegisters": {"R4": {"serializedValue": "040e", "renderedValue": "7"}}})
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let output = cli(&["tree", &id, "--explorer", &url, "--json"]);
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["boxData"]["value"], 1000000);
    assert_eq!(value["boxData"]["tokens"][0]["amount"], 7);
    assert_eq!(value["boxData"]["additionalRegisters"]["R4"], "040e");
}
