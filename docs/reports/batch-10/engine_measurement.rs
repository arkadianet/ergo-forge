//! Session diagnostic: copy to ergo-sandbox/tests/engine_measurement.rs to run.
use ergo_sandbox::{compile_source_raw, decompile};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf};

#[test]
fn measure_node_corpora() {
    decompile::with_large_stack(|| {
        let root = PathBuf::from(std::env::var("DC_NODE_CHECKOUT").unwrap());
        let seed: Value = serde_json::from_slice(&std::fs::read(root.join("test-vectors/ergoscript/compile/compile_seed.json")).unwrap()).unwrap();
        let mut rows = vec![];
        for (i, v) in seed["vectors"].as_array().unwrap().iter().enumerate() {
            if v["oracle"] != "ACCEPT" || v["tree_version"] != 3 { continue; }
            let (Some(tree), Some(source)) = (v["tree_hex"].as_str(), v["source"].as_str()) else {continue};
            if compile_source_raw(source, 3, NetworkPrefix::Testnet).is_err() {continue;}
            rows.push(measure("seed", i, tree, 3, true));
        }
        let mainnet: Value = serde_json::from_slice(&std::fs::read(root.join("test-vectors/mainnet/scala_tx_json/diff_corpus.json")).unwrap()).unwrap();
        let mut seen = BTreeSet::new();
        for tx in mainnet.as_array().unwrap() {
            for out in tx["scalaJson"]["outputs"].as_array().unwrap() {
                if let Some(tree) = out["ergoTree"].as_str() {
                    if seen.insert(tree) { rows.push(measure("mainnet", seen.len()-1, tree, 0, false)); }
                }
            }
        }
        for corpus in ["seed", "mainnet"] {
            let group: Vec<_> = rows.iter().filter(|r| r["corpus"] == corpus).collect();
            println!("{corpus}: {}/{} byte-exact", group.iter().filter(|r| r["exact"] == true).count(), group.len());
        }
        std::fs::write(std::env::var("DC_REPORT").unwrap(), serde_json::to_string_pretty(&json!({"engineRevision":ergo_sandbox::evidence::case::engine_revision(),"rows":rows})).unwrap()+"\n").unwrap();
    });
}
fn measure(corpus: &str, index: usize, tree: &str, version: u8, testnet: bool) -> Value {
    let bytes = hex::decode(tree).unwrap();
    let result = decompile::decompile_report(&bytes, testnet)
        .map_err(|e| e.to_string())
        .and_then(|r| compile_source_raw(&r.source, version, if testnet {NetworkPrefix::Testnet} else {NetworkPrefix::Mainnet}).map_err(|e| e.to_string()));
    json!({"corpus":corpus,"index":index,"treeSha256":hex::encode(Sha256::digest(&bytes)),
        "exact":result.as_ref().is_ok_and(|out| out.tree_bytes == bytes),
        "error":result.err()})
}
