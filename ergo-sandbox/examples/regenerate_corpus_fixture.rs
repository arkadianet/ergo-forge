//! Regenerate `tests/fixtures/compile_corpus_subset.json`.
//!
//! Needs the ergo node checkout as a sibling of this repo (it reads the node's
//! oracle-graded compile corpus). Re-run after bumping the engine rev, or after
//! decompiler changes that legitimately change the byte-exact set:
//!
//! ```text
//! cargo run -p ergo-sandbox --example regenerate_corpus_fixture \
//!   > ergo-sandbox/tests/fixtures/compile_corpus_subset.json
//! ```
//!
//! The emitted fixture is the strict regression net for `cargo test`; it is
//! committed so CI here needs no sibling checkout.
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ergo");
    let path = root.join("test-vectors/ergoscript/compile/compile_seed.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut out: Vec<serde_json::Value> = Vec::new();
    for v in doc["vectors"].as_array().unwrap() {
        if v["oracle"].as_str() != Some("ACCEPT") || v["tree_version"].as_u64() != Some(3) {
            continue;
        }
        let (Some(hex_str), Some(source)) = (v["tree_hex"].as_str(), v["source"].as_str()) else {
            continue;
        };
        // Keep only vectors that recompile under the fixture's empty env.
        if ergo_sandbox::compile_source(source, 3, ergo_ser::address::NetworkPrefix::Testnet)
            .is_err()
        {
            continue;
        }
        let bytes = hex::decode(hex_str).unwrap();
        // Keep only byte-exact round-trips for the strict fixture.
        let src = ergo_sandbox::decompile::decompile_bytes_net(&bytes, true).unwrap();
        if let Ok(c) =
            ergo_sandbox::compile_source(&src, 3, ergo_ser::address::NetworkPrefix::Testnet)
        {
            if c.tree_bytes == bytes {
                out.push(serde_json::json!({
                    "tree": hex_str,
                    "source": source,
                    "treeVersion": 3,
                    "network": "testnet",
                }));
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
