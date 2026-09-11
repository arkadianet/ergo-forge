//! Parse-only vocabulary assessment over explicit local declarations.
//! This does not validate transactions, adjudicate meaning, or infer deployment.
//! Usage: real_property_probe <attempts.json>, from the repository root.
use ergo_sandbox::properties::schema::Declaration;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: real_property_probe <attempts.json>")?;
    if args.next().is_some() {
        return Err("usage: real_property_probe <attempts.json>".into());
    }
    let bytes = std::fs::read(path)?;
    let attempts: Vec<Value> = serde_json::from_slice(&bytes)?;
    let mut results = Vec::new();
    for attempt in attempts {
        let source = attempt["source"].as_str().ok_or("missing source")?;
        let source_bytes = std::fs::read(source)?;
        let digest = hex::encode(Sha256::digest(&source_bytes));
        if attempt["sourceSha256"] != digest {
            return Err(format!("source digest mismatch: {source}").into());
        }
        let result = Declaration::parse(&attempt["declaration"].to_string());
        results.push(json!({
            "id": attempt["id"], "sourceSha256": digest,
            "syntaxAccepted": result.is_ok(),
            "diagnostic": result.err().map(|e| e.to_string()),
            "semanticSupport": "not established by parsing",
            "transactionValidation": "not attempted"
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "attemptsSha256": hex::encode(Sha256::digest(&bytes)), "results": results
        }))?
    );
    Ok(())
}
