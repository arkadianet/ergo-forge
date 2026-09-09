//! Full bundled corpus measurement. Run with DC_REPORT=/absolute/path.json
//! to retain source, bytes and diagnostics for every entry.
use std::{collections::BTreeMap, path::Path};

use ergo_primitives::writer::VlqWriter;
use ergo_sandbox::{compile, decompile, TypedValue};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn files(root: &Path, extension: &str) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root).expect("corpus directory") {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files(&path, extension));
        } else if path.extension().is_some_and(|e| e == extension) {
            found.push(path);
        }
    }
    found.sort();
    found
}

// Deterministic sample instantiations, never claimed to be deployed values.
fn parameters(source: &str) -> BTreeMap<String, TypedValue> {
    let mut params: BTreeMap<_, _> = compile::scan_params(source)
        .into_iter()
        .enumerate()
        .filter(|(_, p)| p.default.is_none())
        .map(|(i, p)| {
            let tpe = p.type_hint.unwrap_or_else(|| {
                if source.contains(&format!("\"${}\"", p.name)) {
                    "String".into()
                } else {
                    "Int".into()
                }
            });
            let value = match tpe.as_str() {
                "Boolean" => json!(true),
                "Byte" | "Short" | "Int" | "Long" | "BigInt" => json!(100 + i),
                "SigmaProp" | "GroupElement" => {
                    json!("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
                }
                "Coll[Byte]" => json!(format!("{:02x}", i + 1).repeat(32)),
                // Hex digits also form valid base64, making literal substitution
                // deterministic for both vendored deployment styles.
                "String" if source.contains(&format!("fromBase58(\"${}\")", p.name)) => {
                    json!(bs58::encode(vec![(i + 1) as u8; 32]).into_string())
                }
                "String" => json!(format!("{:02x}", i + 1).repeat(32)),
                _ => Value::Null,
            };
            (p.name, TypedValue { r#type: tpe, value })
        })
        .collect();
    for (name, tpe, value) in [
        ("PoolNFT", "Coll[Byte]", json!("ab".repeat(32))),
        ("CLEANUP_CONFIRMATION", "Int", json!(100)),
        ("SelfX", "Long", json!(1000000)),
        ("DexFee", "Long", json!(1000)),
        ("MaxMinerFee", "Long", json!(1000000)),
        ("MinerPropBytes", "Coll[Byte]", json!("cd".repeat(32))),
        ("RedeemerPropBytes", "Coll[Byte]", json!("de".repeat(32))),
        ("QuoteId", "Coll[Byte]", json!("ef".repeat(32))),
        (
            "Pk",
            "SigmaProp",
            json!("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"),
        ),
        (
            "RefundProp",
            "SigmaProp",
            json!("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"),
        ),
    ] {
        if source.contains(name) {
            params.entry(name.into()).or_insert(TypedValue {
                r#type: tpe.into(),
                value,
            });
        }
    }
    params
}

fn measure(id: String, bytes: &[u8], version: u8, testnet: bool) -> Value {
    let mut row = json!({"id":id, "tree":hex::encode(bytes)});
    let report = match decompile::decompile_report(bytes, testnet) {
        Ok(report) => report,
        Err(e) => {
            row["bucket"] = json!("fails-to-decompile");
            row["reason"] = json!(e.to_string());
            return row;
        }
    };
    row["source"] = json!(report.source);
    row["raw_placeholders"] = json!(report.raw_placeholders);
    row["truncated"] = json!(report.truncated);
    let net = if testnet {
        NetworkPrefix::Testnet
    } else {
        NetworkPrefix::Mainnet
    };
    match compile::compile_source_raw(&report.source, version, net) {
        Ok(mut out) => {
            // Header metadata is not ErgoScript syntax. Reapply the original
            // version and size flag, as wallets do; never rewrite body/constants.
            out.ergo_tree.version = bytes[0] & 7;
            out.ergo_tree.has_size = bytes[0] & 8 != 0;
            let mut writer = VlqWriter::new();
            if let Err(e) = ergo_ser::ergo_tree::write_ergo_tree(&mut writer, &out.ergo_tree) {
                row["bucket"] = json!("fails-to-recompile");
                row["reason"] = json!(format!("serialization: {e}"));
                return row;
            }
            out.tree_bytes = writer.result();
            row["bucket"] = json!(if out.tree_bytes == bytes {
                "byte-identical"
            } else {
                "recompiles-but-differs"
            });
            if out.tree_bytes != bytes {
                row["recompiled_tree"] = json!(hex::encode(out.tree_bytes));
            }
        }
        Err(e) => {
            row["bucket"] = json!("fails-to-recompile");
            row["reason"] = json!(e.to_string());
        }
    }
    row
}

fn trees(value: &Value, pointer: &str, found: &mut BTreeMap<String, Vec<String>>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let pointer = format!("{pointer}/{key}");
                if matches!(key.as_str(), "tree" | "ergoTree" | "deployedTree") {
                    if let Some(s) = child.as_str() {
                        if !s.is_empty() && hex::decode(s).is_ok() {
                            found.entry(s.into()).or_default().push(pointer.clone());
                        }
                    }
                }
                trees(child, &pointer, found);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                trees(child, &format!("{pointer}/{i}"), found);
            }
        }
        _ => {}
    }
}

#[test]
fn bundled_contracts_and_compiled_fixtures_round_trip() {
    decompile::with_large_stack(|| {
        let start = std::time::Instant::now();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut rows = Vec::new();
        for path in files(&root.join("examples/contracts"), "es") {
            let id = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let source = std::fs::read_to_string(&path).unwrap();
            let params = parameters(&source);
            let mut testnet = false;
            let mut compiled =
                compile::compile_with_params(&source, &params, 3, NetworkPrefix::Mainnet);
            if compiled
                .as_ref()
                .is_err_and(|e| e.to_string().contains("network mismatch"))
            {
                testnet = true;
                compiled =
                    compile::compile_with_params(&source, &params, 3, NetworkPrefix::Testnet);
            }
            let mut row = match compiled {
                Ok(out) => measure(id, &out.tree_bytes, 3, testnet),
                Err(e) => {
                    json!({"id":id, "bucket":"initial-compile-failed", "reason":e.to_string()})
                }
            };
            row["params"] = serde_json::to_value(params).unwrap();
            rows.push(row);
        }
        let mut fixtures = files(&root.join("examples"), "json");
        fixtures.extend(files(&root.join("ergo-sandbox/tests/fixtures"), "json"));
        for path in fixtures {
            let doc: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            if path.ends_with("compile_corpus_subset.json") {
                for (i, vector) in doc.as_array().unwrap().iter().enumerate() {
                    let bytes = hex::decode(vector["tree"].as_str().unwrap()).unwrap();
                    let id = format!("{}#/{i}", path.strip_prefix(root).unwrap().display());
                    rows.push(measure(
                        id,
                        &bytes,
                        vector["treeVersion"].as_u64().unwrap_or(3) as u8,
                        true,
                    ));
                }
                continue;
            }
            let mut found = BTreeMap::new();
            trees(&doc, "", &mut found);
            for (tree, pointers) in found {
                let bytes = hex::decode(&tree).unwrap();
                let id = format!(
                    "{}#{}",
                    path.strip_prefix(root).unwrap().display(),
                    pointers[0]
                );
                let mut row = measure(id, &bytes, 3, false);
                row["aliases"] = json!(pointers);
                rows.push(row);
            }
        }
        let mut counts = BTreeMap::<String, usize>::new();
        for row in &rows {
            *counts
                .entry(row["bucket"].as_str().unwrap().into())
                .or_default() += 1;
        }
        println!(
            "{} entries in {:?}: {:?}",
            rows.len(),
            start.elapsed(),
            counts
        );
        if let Ok(path) = std::env::var("DC_REPORT") {
            std::fs::write(path, serde_json::to_string_pretty(&rows).unwrap()).unwrap();
        }
        // Pin each input's emitted bytes and outcome, not just an aggregate
        // floor: an exact contract cannot silently trade places with a failure.
        // Keep known divergences visible until deliberately fixed and reviewed.
        let actual: BTreeMap<String, Value> = rows
            .iter()
            .map(|row| {
                let hash = row["tree"]
                    .as_str()
                    .map(|tree| hex::encode(Sha256::digest(hex::decode(tree).unwrap())));
                (
                    row["id"].as_str().unwrap().into(),
                    json!({
                        "bucket": row["bucket"],
                        "raw_placeholders": row["raw_placeholders"].as_u64().unwrap_or(0),
                        "truncated": row["truncated"].as_bool().unwrap_or(false),
                        "tree_sha256": hash,
                    }),
                )
            })
            .collect();
        let expected: BTreeMap<String, Value> =
            serde_json::from_str(include_str!("fixtures/decompile_corpus_expectations.json"))
                .unwrap();
        assert_eq!(actual.len(), rows.len(), "duplicate measurement IDs");
        assert_eq!(
            actual.keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>(),
            "corpus membership changed; measure and review new entries"
        );
        for (id, outcome) in actual {
            assert_eq!(outcome, expected[&id], "{id}: round-trip outcome or original bytes changed; inspect DC_REPORT before updating expectations");
        }
    });
}

/// Optional measurement of the same external node fixtures used by the older
/// floor tests. Never a dependency of the bundled CI test.
#[test]
#[ignore = "external node checkout; set DC_NODE_CHECKOUT and DC_REPORT"]
fn external_compiled_fixtures_measurement() {
    decompile::with_large_stack(|| {
        let root =
            std::path::PathBuf::from(std::env::var("DC_NODE_CHECKOUT").expect("DC_NODE_CHECKOUT"));
        let vectors = root.join("test-vectors");
        let seed: Value = serde_json::from_str(
            &std::fs::read_to_string(vectors.join("ergoscript/compile/compile_seed.json")).unwrap(),
        )
        .unwrap();
        let mut rows = Vec::new();
        for (i, v) in seed["vectors"].as_array().unwrap().iter().enumerate() {
            if let Some(tree) = v["tree_hex"].as_str() {
                let mut row = measure(
                    format!("ergoscript/compile/compile_seed.json#/vectors/{i}"),
                    &hex::decode(tree).unwrap(),
                    3,
                    true,
                );
                row["original_source"] = v["source"].clone();
                rows.push(row);
            }
        }
        let mainnet: Value = serde_json::from_str(
            &std::fs::read_to_string(vectors.join("mainnet/scala_tx_json/diff_corpus.json"))
                .unwrap(),
        )
        .unwrap();
        let mut found = BTreeMap::new();
        trees(&mainnet, "", &mut found);
        for (tree, pointers) in found {
            let mut row = measure(
                format!("mainnet/scala_tx_json/diff_corpus.json#{}", pointers[0]),
                &hex::decode(tree).unwrap(),
                3,
                false,
            );
            row["aliases"] = json!(pointers);
            rows.push(row);
        }
        for entry in std::fs::read_dir(&vectors).unwrap() {
            let path = entry.unwrap().path();
            if path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("failing_tree_")
                && path.extension().is_some_and(|e| e == "hex")
            {
                let bytes = hex::decode(std::fs::read_to_string(&path).unwrap().trim()).unwrap();
                rows.push(measure(
                    path.file_name().unwrap().to_string_lossy().into(),
                    &bytes,
                    3,
                    false,
                ));
            }
        }
        rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        let mut counts = BTreeMap::<String, usize>::new();
        for row in &rows {
            *counts
                .entry(row["bucket"].as_str().unwrap().into())
                .or_default() += 1;
        }
        println!("External: {} entries: {counts:?}", rows.len());
        std::fs::write(
            std::env::var("DC_REPORT").expect("DC_REPORT"),
            serde_json::to_string_pretty(&rows).unwrap(),
        )
        .unwrap();
    });
}
