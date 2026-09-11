//! Offline scan of every distinct mainnet contract script supplied on stdin
//! (`hash|ergoTree|boxCount|value`). Reads local data only; nothing is fetched
//! or broadcast. Reports structural lift coverage and unadjudicated lint counts.
//! Printed-text proxies measure prevalence, never property expressiveness.
//! Usage: cargo run -p ergo-sandbox --release --example mainnet_scan < export.txt
//! Rows must be distinct hash|ergoTree|boxCount|value records, without a header.
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut digest = Sha256::new();
    let mut hashes = BTreeSet::new();
    let mut trees = BTreeSet::new();
    let (mut n, mut ok, mut complete, mut parse_err) = (0u64, 0u64, 0u64, 0u64);
    let (mut findings, mut clean, mut truncated) = (0u64, 0u64, 0u64);
    let (mut mul, mut ctx, mut either) = (0u64, 0u64, 0u64);
    let (mut boxes_total, mut boxes_either) = (0u128, 0u128);
    let mut by_lint = BTreeMap::<String, u64>::new();
    let mut size_hist = BTreeMap::<u64, u64>::new();

    let mut input = std::io::stdin().lock();
    let mut line = String::new();
    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        digest.update(line.as_bytes());
        let f: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('|').collect();
        if f.len() != 4 {
            return Err(format!("row {}: expected four fields", n + 1).into());
        }
        let hash = hex::decode(f[0])?;
        if hash.is_empty() || !hashes.insert(hash) {
            return Err(format!("row {}: empty or duplicate export hash", n + 1).into());
        }
        let count: u128 = f[2].parse()?;
        let _value: u128 = f[3].parse()?;
        boxes_total = boxes_total.checked_add(count).ok_or("box count overflow")?;
        n += 1;
        let Ok(bytes) = hex::decode(f[1]) else {
            parse_err += 1;
            continue;
        };
        if !trees.insert(Sha256::digest(&bytes).to_vec()) {
            return Err(format!("row {n}: duplicate script bytes").into());
        }
        *size_hist
            .entry(((bytes.len() as u64) / 128) * 128)
            .or_default() += 1;
        let Ok(tree) = ergo_sandbox::inspect::parse_tree(&bytes) else {
            parse_err += 1;
            continue;
        };
        let lifted = ergo_sandbox::decompile::with_large_stack(move || {
            ergo_sandbox::lift_tree(&tree, false)
        });
        ok += 1;
        if lifted.raw_placeholders == 0 {
            complete += 1;
        }
        if lifted.truncated {
            truncated += 1;
        }

        let audit = ergo_sandbox::audit::audit(&lifted);
        if audit.findings.is_empty() {
            clean += 1;
        }
        findings += audit.findings.len() as u64;
        for a in &audit.findings {
            *by_lint.entry(a.lint.to_string()).or_default() += 1;
        }

        // These are printer-string proxies, not AST or semantic classifications.
        let ir = ergo_sandbox::decompile::print(&lifted.node);
        let m = ir.contains(" * ");
        let c = ir.contains("dataInputs");
        if m {
            mul += 1;
        }
        if c {
            ctx += 1;
        }
        if m || c {
            either += 1;
            boxes_either += count;
        }

        if n % 25000 == 0 {
            eprintln!("  ... {n} scanned");
        }
    }

    if n == 0 {
        return Err("empty export".into());
    }
    println!("export sha256: {}", hex::encode(digest.finalize()));
    println!("=== OFFLINE DISTINCT-SCRIPT CENSUS ===");
    println!("distinct scripts:   {n}");
    println!(
        "  lifted:           {ok}  ({:.2}%)",
        100.0 * ok as f64 / n as f64
    );
    println!(
        "  no raw placeholders:         {complete}  ({:.2}% of lifted)",
        percent(complete, ok)
    );
    println!("  truncated:        {truncated}");
    println!("  parse/lift error: {parse_err}");
    println!("  boxes covered:    {boxes_total}");
    println!("\n=== UNADJUDICATED FINDINGS (lint percentages use findings denominator) ===");
    println!("total findings:     {findings}");
    println!("clean contracts:    {clean}  ({:.1}%)", percent(clean, ok));
    println!(
        "mean per contract:  {:.1}",
        findings as f64 / ok.max(1) as f64
    );
    for (k, v) in &by_lint {
        println!("  {k:24} {v:8}  ({:.1}%)", percent(*v, findings));
    }
    println!("\n=== PRINTED-TEXT PREVALENCE (not expressiveness) ===");
    println!("printed multiplication: {mul}  ({:.1}%)", percent(mul, ok));
    println!("printed dataInputs:     {ctx}  ({:.1}%)", percent(ctx, ok));
    println!(
        "either printed proxy:   {either}  ({:.1}%)",
        percent(either, ok)
    );
    println!(
        "boxes under those:      {boxes_either}  ({:.1}%)",
        100.0 * boxes_either as f64 / boxes_total.max(1) as f64
    );
    println!("\n=== SIZE DISTRIBUTION (bytes) ===");
    for (k, v) in &size_hist {
        if *v > n / 200 {
            println!("  {k:5}-{:<5} {v}", k + 127);
        }
    }
    Ok(())
}

fn percent(numerator: u64, denominator: u64) -> f64 {
    100.0 * numerator as f64 / denominator.max(1) as f64
}
