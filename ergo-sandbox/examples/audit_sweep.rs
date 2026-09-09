//! Static audit of every .es file, with deterministic structural placeholders.
//!
//! CARGO_TARGET_DIR=./target-sweep cargo run --release -p ergo-sandbox \
//!   --example audit_sweep > sweep.json
//!
//! Optional argument: corpus directory (defaults to examples/contracts).
//! Compile failures and partial lifts are records, not silently skipped files.
//! No evaluation, transaction construction, or network access is performed.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use ergo_sandbox::audit::{audit, Completeness};
use ergo_sandbox::compile::{compile_with_params, scan_params, ParamError, ParamNeed};
use ergo_sandbox::{lift_tree, TypedValue};
use ergo_ser::address::NetworkPrefix;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn contracts(dir: &Path, paths: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            contracts(&entry.path(), paths)?;
        } else if entry.path().extension().is_some_and(|ext| ext == "es") {
            paths.push(entry.path());
        }
    }
    Ok(())
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            out.push(if i > chunk.len() {
                '='
            } else {
                ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char
            });
        }
    }
    out
}

fn placeholder(source: &str, need: &ParamNeed, index: usize) -> Result<TypedValue, String> {
    let name = need.name.as_str();
    let bytes = Sha256::digest(format!("audit-sweep:{name}"));
    let encoded = [
        ("fromBase64", base64(&bytes)),
        ("fromBase58", bs58::encode(bytes).into_string()),
        ("fromBase16", hex::encode(bytes)),
    ];
    // Handle both "$name" and Rosen's "ALL_CAPS_NAME" string placeholders.
    for (decoder, value) in encoded {
        if [
            format!("{decoder}(\"${name}\")"),
            format!("{decoder}(\"{name}\")"),
        ]
        .iter()
        .any(|pattern| source.contains(pattern))
        {
            return Ok(TypedValue {
                r#type: "String".into(),
                value: json!(value),
            });
        }
    }
    // Untyped numeric constants in the vendored Dexy sources.
    let numeric = match name {
        "initialDexyTokens" | "initialLp" | "lpSupply" => {
            Some(("Long", 1_000_000_000_000_000_000i64))
        }
        "intMax" => Some(("Int", i64::from(i32::MAX))),
        "epochLength" => Some(("Int", 720)),
        "startHeight" => Some(("Int", 1000)),
        "cliffHeight" => Some(("Int", 2000)),
        "endHeight" => Some(("Int", 3000)),
        "royaltyPercent" => Some(("Int", 5)),
        "feeNumLp" => Some(("Int", 3)),
        "feeNum" => Some(("Int", 997)),
        "feeDenomLp" | "feeDenom" => Some(("Int", 1000)),
        "minRatioPercent" => Some(("Int", 400)),
        "maxRatioPercent" => Some(("Int", 800)),
        "feePercent" => Some(("Int", 2)),
        "brunoNum" | "phoenixNum" | "kushtiNum" => Some(("Long", 10)),
        "creatorNum" => Some(("Long", 1)),
        _ => None,
    };
    if let Some((tpe, value)) = numeric {
        return Ok(TypedValue {
            r#type: tpe.into(),
            value: json!(value),
        });
    }
    let tpe = need
        .type_hint
        .as_deref()
        .ok_or_else(|| format!("no placeholder type for ${name}"))?;
    let value = if let Some(default) = &need.default {
        json!(default)
    } else {
        match tpe {
            "Int" => json!(1000),
            "Long" => json!(1_000_000),
            "Boolean" => json!(true),
            "Coll[Byte]" => json!(hex::encode(bytes)),
            "SigmaProp" | "GroupElement" => {
                let point = k256::ProjectivePoint::GENERATOR * k256::Scalar::from(index as u64 + 1);
                json!(hex::encode(
                    point.to_affine().to_encoded_point(true).as_bytes()
                ))
            }
            other => return Err(format!("no placeholder policy for ${name}: {other}")),
        }
    };
    Ok(TypedValue {
        r#type: tpe.into(),
        value,
    })
}

// Explicit policies for bare deployment constants reported by the compiler.
// Unknown missing names remain failures; never guess their types repeatedly.
fn bare_need(name: &str) -> Option<ParamNeed> {
    let tpe = match name {
        "PoolNFT" | "QuoteId" | "MinerPropBytes" | "RedeemerPropBytes" => "Coll[Byte]",
        "SelfX" | "DexFee" | "MaxMinerFee" => "Long",
        "Pk" | "RefundProp" => "SigmaProp",
        "CLEANUP_CONFIRMATION" => "Int",
        _ => return None,
    };
    Some(ParamNeed {
        name: name.into(),
        type_hint: Some(tpe.into()),
        default: None,
        description: None,
    })
}

fn analyse(path: &Path, root: &Path) -> Value {
    let relative = path
        .strip_prefix(root)
        .expect("discovered beneath root")
        .to_string_lossy();
    let mut record = json!({
        "contract": relative,
        "protocol": relative.split('/').next().unwrap_or(""),
        "status": "failed",
        "params": {},
        "findings": [],
    });
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            record["stage"] = json!("read");
            record["error"] = json!(error.to_string());
            return record;
        }
    };
    record["sourceSha256"] = json!(hex::encode(Sha256::digest(source.as_bytes())));
    let mut params = BTreeMap::new();
    for (index, need) in scan_params(&source).iter().enumerate() {
        match placeholder(&source, need, index) {
            Ok(value) => {
                params.insert(need.name.clone(), value);
            }
            Err(error) => {
                record["params"] = json!(params);
                record["stage"] = json!("parameters");
                record["error"] = json!(error);
                return record;
            }
        }
    }
    let mut network = NetworkPrefix::Testnet;
    let mut network_name = "testnet";
    let mut attempts = Vec::new();
    let compiled = loop {
        record["params"] = json!(params);
        record["network"] = json!(network_name);
        match compile_with_params(&source, &params, 3, network) {
            Ok(compiled) => break compiled,
            Err(error) => {
                attempts.push(json!({"network": network_name, "error": error.to_string()}));
                record["attemptErrors"] = json!(attempts);
                if network_name == "testnet" && error.to_string().contains("network mismatch") {
                    network = NetworkPrefix::Mainnet;
                    network_name = "mainnet";
                    continue;
                }
                if let ParamError::Missing(names) = &error {
                    let additions: Option<Vec<_>> = names
                        .iter()
                        .filter(|name| !params.contains_key(*name))
                        .map(|name| {
                            bare_need(name).and_then(|need| {
                                placeholder(&source, &need, params.len())
                                    .ok()
                                    .map(|value| (name.clone(), value))
                            })
                        })
                        .collect();
                    if let Some(additions) = additions.filter(|items| !items.is_empty()) {
                        params.extend(additions);
                        continue;
                    }
                }
                record["stage"] = json!("compile");
                record["error"] = json!(error.to_string());
                return record;
            }
        }
    };
    record["treeSha256"] = json!(hex::encode(Sha256::digest(&compiled.tree_bytes)));
    let lifted = lift_tree(&compiled.ergo_tree, false);
    let result = audit(&lifted);
    record["status"] = json!(match result.completeness {
        Completeness::Complete => "complete",
        Completeness::Partial { .. } => "partial",
    });
    if matches!(result.completeness, Completeness::Partial { .. }) {
        record["liftedSource"] = json!(ergo_sandbox::decompile::print(&lifted.node));
    }
    record["rawPlaceholders"] = json!(lifted.raw_placeholders);
    record["truncated"] = json!(lifted.truncated);
    record["findings"] = json!(result
        .findings
        .iter()
        .map(|finding| json!({
            "lint": finding.lint,
            "severity": finding.severity.label(),
            "nodeId": finding.node_id,
            "irId": finding.ir_id,
            "snippet": finding.snippet,
            "message": finding.message,
        }))
        .collect::<Vec<_>>());
    record
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/contracts"));
    if args.next().is_some() {
        return Err("usage: audit_sweep [corpus-directory]".into());
    }
    let mut paths = Vec::new();
    contracts(&root, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err("no .es contracts found".into());
    }
    let records: Vec<_> = paths.iter().map(|path| analyse(path, &root)).collect();
    let mut by_lint = BTreeMap::<String, usize>::new();
    let mut by_protocol = BTreeMap::<String, usize>::new();
    let mut by_status = BTreeMap::<String, usize>::new();
    let mut by_severity = BTreeMap::<String, usize>::new();
    for record in &records {
        *by_status
            .entry(record["status"].as_str().unwrap().into())
            .or_default() += 1;
        let count = by_protocol
            .entry(record["protocol"].as_str().unwrap().into())
            .or_default();
        for finding in record["findings"].as_array().unwrap() {
            *count += 1;
            *by_lint
                .entry(finding["lint"].as_str().unwrap().into())
                .or_default() += 1;
            *by_severity
                .entry(finding["severity"].as_str().unwrap().into())
                .or_default() += 1;
        }
    }
    let output = json!({
        "schemaVersion": 1,
        "treeVersion": 3,
        "networkPolicy": "testnet, retry mainnet on address network mismatch; recorded per contract",
        "parameterPolicy": "Structural placeholders, not deployment values; see audit_sweep.rs. Exact inputs recorded per contract.",
        "contracts": records.len(),
        "byStatus": by_status,
        "byLint": by_lint,
        "byProtocol": by_protocol,
        "bySeverity": by_severity,
        "results": records,
    });
    serde_json::to_writer_pretty(std::io::stdout().lock(), &output)?;
    println!();
    Ok(())
}
