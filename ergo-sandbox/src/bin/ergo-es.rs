//! `ergo-es` — the workbench CLI: compile / eval / decompile.
//!
//! v1 human-text output; a JSON output flag can be added behind shells
//! without changing the engine.

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use ergo_sandbox::eval::Verdict;
use ergo_sandbox::{compile_source, eval_scenario, inspect, Scenario};
use ergo_ser::address::NetworkPrefix;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        usage();
        return ExitCode::from(2);
    };
    let rest = &args[1..];
    let result = match cmd.as_str() {
        "compile" => cmd_compile(rest),
        "params" => cmd_params(rest),
        "eval" => cmd_eval(rest),
        "decompile" => cmd_decompile(rest),
        "roundtrip" => cmd_roundtrip(rest),
        "audit" => cmd_audit(rest),
        "hunt" => cmd_hunt(rest),
        "test" => cmd_test(rest),
        "validate-tx" => cmd_validate_tx(rest),
        "compose" => cmd_compose(rest),
        "help" | "--help" | "-h" => {
            usage();
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("unknown command `{other}`");
            usage();
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    println!(
        "ergo-es — ErgoScript workbench CLI

USAGE:
  ergo-es compile <source-file> [--tree-version N] [--network mainnet|testnet]
                  [--params params.json]
      Compile ErgoScript source to ErgoTree bytes + P2S/P2SH addresses.
      --params supplies compile-time constants (JSON: name -> {{type, value}}).
  ergo-es test <contract.test.json>
      Run a contract test suite (source or tree + named scenarios with
      expected verdicts); one line per case, non-zero exit on any failure.
  ergo-es validate-tx <request.json>
      Will this unsigned transaction validate? {{tx, boxes, height?}}: every
      input's script runs in the real context; ERG/token conservation is
      checked. Non-zero exit when the transaction would be rejected.
  ergo-es compose <spec.json> [--params p.json] [--suite out.test.json]
      Assemble ErgoScript from spending paths (who + conditions); with
      --params also generate a test suite whose expectations come from the
      composer's model (write it with --suite, then `ergo-es test` it).
  ergo-es params <source-file>
      List the $parameters a source needs (with // $name: Type hints).
  ergo-es eval <scenario.json> [--hot-spots]
      Evaluate a scenario: contract (source or tree hex) + spending context
      → verdict, cost, trace. See README scenario schema. --hot-spots ranks
      the operations by cost (needs a --features cost-trace build).
  ergo-es decompile <hex | file | --seed | --mainnet [N]>
      Print the structural view of ErgoTree bytes (--seed / --mainnet run
      the bundled corpora instead).
  ergo-es roundtrip <hex | file | [--network mainnet|testnet] | --seed |
                     --mainnet [N]> [-v]
      Decompile → recompile → byte-compare. Single trees accept --network
      (default mainnet); -v prints every failure reason. Corpora paths
      resolve against the ergo node checkout (sibling of this repo).
  ergo-es audit <hex | --seed | --mainnet | --trees file.json>
      Static lints over the lifted tree. Single trees print findings;
      corpora print a summary tally.
  ergo-es hunt <hex | --mainnet [N] | --trees file.json> [--height H] [--self-box file.json]
               [--data-inputs file.json]
      Spend hunt: can anyone spend this box with no key? Six probes (three
      heights x attacker/preserve output) on the consensus reducer.
      --mainnet tallies the corpus; hits go to stderr for hand checks.
"
    );
}

// ── flags ────────────────────────────────────────────────────────────────────

fn flag_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == name {
            return Ok(Some(
                it.next()
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| format!("flag {name} needs a value"))?
                    .clone(),
            ));
        }
    }
    Ok(None)
}

fn parse_network(name: &str) -> Result<NetworkPrefix, String> {
    match name {
        "mainnet" => Ok(NetworkPrefix::Mainnet),
        "testnet" => Ok(NetworkPrefix::Testnet),
        other => Err(format!(
            "unknown network `{other}` (expected mainnet|testnet)"
        )),
    }
}

fn read_input(arg: &str) -> Result<String, String> {
    if arg == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("stdin: {e}"))?;
        return Ok(buf);
    }
    let path = Path::new(arg);
    if path.is_file() {
        return std::fs::read_to_string(path).map_err(|e| format!("{arg}: {e}"));
    }
    Ok(arg.to_string())
}

// ── compile ──────────────────────────────────────────────────────────────────

fn cmd_compile(args: &[String]) -> Result<(), String> {
    let Some(src_ref) = args.first() else {
        return Err("compile needs a source file (or - for stdin)".into());
    };
    let source = read_input(src_ref)?;
    let tree_version: u8 = match flag_value(args, "--tree-version")? {
        Some(v) => v.parse().map_err(|_| format!("bad --tree-version `{v}`"))?,
        None => 0,
    };
    let network = match flag_value(args, "--network")? {
        Some(n) => parse_network(&n)?,
        None => NetworkPrefix::Mainnet,
    };
    let out = match flag_value(args, "--params")? {
        Some(path) => {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            let params: std::collections::BTreeMap<String, ergo_sandbox::TypedValue> =
                serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            ergo_sandbox::compile::compile_with_params(&source, &params, tree_version, network)
                .map_err(|e| e.to_string())?
        }
        None => compile_source(&source, tree_version, network).map_err(|e| e.to_string())?,
    };
    println!("tree:     {}", hex::encode(&out.tree_bytes));
    println!("p2s:      {}", out.p2s_address);
    println!("p2sh:     {}", out.p2sh_address);
    Ok(())
}

/// `ergo-es params <source-file>` — the $parameters a source needs, as JSON
/// (name → type hint or null), ready to fill in and pass to `--params`.
fn cmd_params(args: &[String]) -> Result<(), String> {
    let Some(src_ref) = args.first() else {
        return Err("params needs a source file (or - for stdin)".into());
    };
    let source = read_input(src_ref)?;
    let needs = ergo_sandbox::compile::scan_params(&source);
    let map: serde_json::Map<String, serde_json::Value> = needs
        .into_iter()
        .map(|n| {
            (
                n.name,
                n.type_hint
                    .map(Into::into)
                    .unwrap_or(serde_json::Value::Null),
            )
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?
    );
    Ok(())
}

// ── eval ─────────────────────────────────────────────────────────────────────

fn cmd_eval(args: &[String]) -> Result<(), String> {
    let want_hot_spots = args.iter().any(|a| a == "--hot-spots");
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        return Err("eval needs a scenario JSON file (or - for stdin)".into());
    };
    if want_hot_spots && !cfg!(feature = "cost-trace") {
        return Err("--hot-spots needs a `--features cost-trace` build".into());
    }
    let text = read_input(path)?;
    let scenario: Scenario =
        serde_json::from_str(&text).map_err(|e| format!("scenario JSON: {e}"))?;
    let outcome = eval_scenario(&scenario).map_err(|e| e.to_string())?;
    println!("verdict:  {}", verdict_str(outcome.verdict));
    if let Some(err) = &outcome.error {
        println!("error:    {err}");
    }
    if let Some(red) = &outcome.reduced_to {
        println!("reducedTo: {red}");
    }
    println!(
        "cost:     {} / {} (block units)",
        outcome.cost, outcome.cost_limit
    );
    println!("tree:     {}", outcome.tree_hex);
    println!("p2s:      {}", outcome.p2s_address);
    for t in &outcome.trace {
        println!("  trace: {} = {}", t.label, t.value);
    }
    #[cfg(feature = "cost-trace")]
    {
        if want_hot_spots {
            print_hot_spots(&outcome.cost_breakdown);
        } else {
            for c in &outcome.cost_breakdown {
                println!("  cost {:>6} {} (total {})", c.delta, c.label, c.total);
            }
        }
    }
    Ok(())
}

/// Ranked cost view: one row per operation, highest first.
#[cfg(feature = "cost-trace")]
fn print_hot_spots(lines: &[ergo_sandbox::eval::CostLine]) {
    let rows = ergo_sandbox::hot_spots::hot_spots(lines);
    let jit_total: u64 = lines.iter().map(|l| l.delta).sum();
    println!(
        "hot spots ({} JIT units over {} steps):",
        jit_total,
        lines.len()
    );
    println!("  {:>6}  {:>5}  {:>5}  operation", "jit", "steps", "share");
    for r in &rows {
        println!(
            "  {:>6}  {:>5}  {:>4.0}%  {}",
            r.jit,
            r.count,
            r.share * 100.0,
            r.label
        );
    }
}

fn verdict_str(v: Verdict) -> &'static str {
    match v {
        Verdict::Pass => "PASS",
        Verdict::Fail => "FAIL",
        Verdict::Error => "ERROR",
        Verdict::NeedsProof => "NEEDS-PROOF",
        Verdict::ProofAccepted => "PROOF-ACCEPTED",
        Verdict::ProofRejected => "PROOF-REJECTED",
    }
}

// ── decompile ────────────────────────────────────────────────────────────────

fn cmd_decompile(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("--seed") => decompile_seed(),
        Some("--mainnet") => decompile_mainnet(args.get(1)),
        Some(arg) => {
            let bytes = resolve_bytes(arg)?;
            let report = inspect::tree_report(&bytes).map_err(|e| e.to_string())?;
            print!("{report}");
            Ok(())
        }
        None => Err("decompile needs tree hex, a file path, --seed, or --mainnet [N]".into()),
    }
}

fn resolve_bytes(arg: &str) -> Result<Vec<u8>, String> {
    let trimmed = arg.trim();
    // Prefer literal hex; fall back to a file path.
    if let Ok(bytes) = hex::decode(trimmed) {
        return Ok(bytes);
    }
    let text = read_input(arg)?;
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    hex::decode(cleaned.trim()).map_err(|e| {
        format!(
            "`{}` is neither valid hex nor a readable file (hex error: {e})",
            truncate(arg, 40)
        )
    })
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

fn decompile_seed() -> Result<(), String> {
    let root = workspace_root();
    let path = root.join("test-vectors/ergoscript/compile/compile_seed.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| corpus_err(&path, e))?)
            .map_err(|e| e.to_string())?;
    for v in doc["vectors"].as_array().ok_or("vectors array")? {
        if v["oracle"].as_str() != Some("ACCEPT") {
            continue;
        }
        let Some(hex_str) = v["tree_hex"].as_str() else {
            continue;
        };
        let bytes = hex::decode(hex_str).map_err(|e| e.to_string())?;
        let src = v["source"].as_str().unwrap_or("?").replace('\n', " ");
        println!("--- {}", truncate(&src, 70));
        let report = inspect::tree_report(&bytes).map_err(|e| e.to_string())?;
        print!("{report}");
    }
    Ok(())
}

fn decompile_mainnet(limit: Option<&String>) -> Result<(), String> {
    let trees = mainnet_trees(limit)?;
    for (i, t) in trees.iter().enumerate() {
        let bytes = hex::decode(t).map_err(|e| e.to_string())?;
        println!("--- mainnet #{i}");
        let report = inspect::tree_report(&bytes).map_err(|e| e.to_string())?;
        print!("{report}");
    }
    Ok(())
}

/// `ergo-es roundtrip` — decompile → recompile → byte-compare over a tree or
/// a corpus. Prints the tally plus the first few failure reasons (pass `-v`
/// for every reason). Corpus paths resolve against the ergo node checkout
/// (sibling of this repo).
fn cmd_roundtrip(args: &[String]) -> Result<(), String> {
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let mut tally = Tally {
        exact: 0,
        diff: 0,
        raw: 0,
        err: 0,
        reasons: Vec::new(),
    };
    match args.first().map(String::as_str) {
        Some("--seed") | Some("--mainnet") => {
            let is_seed = args[0] == "--seed";
            // Seed vectors were captured at treeVersion 3 on testnet; mainnet
            // trees are pre-v3 under mainnet.
            let (tree_version, network) = if is_seed {
                (3, NetworkPrefix::Testnet)
            } else {
                (0, NetworkPrefix::Mainnet)
            };
            let vectors: Vec<(String, Option<String>)> = if is_seed {
                seed_vectors()?
                    .into_iter()
                    .map(|(t, s)| (t, Some(s)))
                    .collect()
            } else {
                mainnet_trees(first_positional(args))?
                    .into_iter()
                    .map(|t| (t, None))
                    .collect()
            };
            let mut processed = 0usize;
            for (h, source) in &vectors {
                // Skip env-dependent sources (oracle demo env unavailable).
                if let Some(orig) = source {
                    if ergo_sandbox::compile_source(orig, tree_version, network).is_err() {
                        continue;
                    }
                }
                let bytes = hex::decode(h).map_err(|e| e.to_string())?;
                classify_tree(&bytes, tree_version, network, &mut tally);
                processed += 1;
            }
            println!(
                "trees: {} (of {}) exact={} diff={} raw={} err={}",
                processed,
                vectors.len(),
                tally.exact,
                tally.diff,
                tally.raw,
                tally.err
            );
            print_reasons(&tally, verbose);
            Ok(())
        }
        Some(arg) if arg == "-v" || arg == "--verbose" => {
            Err("roundtrip needs a tree, a file, --seed, or --mainnet [N]".into())
        }
        Some(arg) => {
            let bytes = resolve_bytes(arg)?;
            classify_tree(
                &bytes,
                tree_version_of(&bytes),
                NetworkPrefix::Mainnet,
                &mut tally,
            );
            println!(
                "{}",
                if tally.exact == 1 {
                    "EXACT"
                } else if tally.raw == 1 {
                    "RAW"
                } else if tally.err == 1 {
                    "ERR"
                } else {
                    "DIFF"
                }
            );
            print_reasons(&tally, verbose);
            Ok(())
        }
        None => Err("roundtrip needs a tree, a file, --seed, or --mainnet [N]".into()),
    }
}

fn first_positional(args: &[String]) -> Option<&String> {
    args.iter().skip(1).find(|a| !a.starts_with('-'))
}

fn print_reasons(tally: &Tally, verbose: bool) {
    if tally.reasons.is_empty() {
        return;
    }
    let limit = if verbose {
        tally.reasons.len()
    } else {
        tally.reasons.len().min(5)
    };
    for r in tally.reasons.iter().take(limit) {
        println!("  - {r}");
    }
    if !verbose && tally.reasons.len() > limit {
        println!(
            "  …and {} more (pass -v for all)",
            tally.reasons.len() - limit
        );
    }
}

/// Per-corpus round-trip totals plus the failure reasons seen.
struct Tally {
    exact: usize,
    diff: usize,
    raw: usize,
    err: usize,
    reasons: Vec<String>,
}

/// Classify one tree: decompile, recompile, compare.
///
/// Decompiles with the SAME network the source will be compiled against (PK
/// address constants are network-tagged), and records the failure reason
/// instead of swallowing it.
fn classify_tree(bytes: &[u8], tree_version: u8, network: NetworkPrefix, tally: &mut Tally) {
    let owned = bytes.to_vec();
    let out = ergo_sandbox::decompile::with_large_stack(move || {
        ergo_sandbox::decompile::decompile_report(&owned, network == NetworkPrefix::Testnet)
    });
    let decompiled = match out {
        Ok(d) => d,
        Err(e) => {
            tally.err += 1;
            tally.reasons.push(e.to_string());
            return;
        }
    };
    if decompiled.raw_placeholders > 0 {
        tally.raw += 1;
        if decompiled.truncated {
            tally.reasons.push(format!(
                "nesting deeper than the lift ceiling ({} placeholders, {} levels max)",
                decompiled.raw_placeholders,
                ergo_sandbox::decompile::MAX_LIFT_DEPTH
            ));
        } else {
            tally.reasons.push(format!(
                "{} construct(s) have no source-like lift yet",
                decompiled.raw_placeholders
            ));
        }
        return;
    }
    // The compiler's parse recursion is unbounded upstream — recompile on the
    // large stack for the same reason the decompile runs there.
    let src = decompiled.source;
    let compile_out = ergo_sandbox::decompile::with_large_stack(move || {
        ergo_sandbox::compile_source(&src, tree_version, network)
            .map(|o| o.tree_bytes)
            .map_err(|e| e.to_string())
    });
    match compile_out {
        Ok(tree_bytes) if tree_bytes == bytes => tally.exact += 1,
        Ok(tree_bytes) => {
            tally.diff += 1;
            tally.reasons.push(format!(
                "recompiled to different bytes (want {}, got {})",
                hex::encode(bytes),
                hex::encode(&tree_bytes)
            ));
        }
        Err(e) => {
            tally.err += 1;
            tally.reasons.push(e);
        }
    }
}

fn tree_version_of(bytes: &[u8]) -> u8 {
    // ErgoTree header: bits 0..2 are the tree version, bit 3 = has_size,
    // bit 4 = constant segregation. Recompilation needs the header version
    // (it selects v5 vs v6 method visibility).
    match bytes.first() {
        Some(h) => h & 0x07,
        None => 0,
    }
}

fn seed_vectors() -> Result<Vec<(String, String)>, String> {
    // Returns (tree_hex, source) — the source lets the roundtrip skip
    // oracle demo-env-dependent vectors (not compilable with an empty env).
    let root = workspace_root();
    let path = root.join("test-vectors/ergoscript/compile/compile_seed.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| corpus_err(&path, e))?)
            .map_err(|e| e.to_string())?;
    Ok(doc["vectors"]
        .as_array()
        .ok_or("vectors array")?
        .iter()
        .filter(|v| v["oracle"].as_str() == Some("ACCEPT") && v["tree_version"].as_u64() == Some(3))
        .filter_map(|v| {
            Some((
                v["tree_hex"].as_str()?.to_string(),
                v["source"].as_str().unwrap_or("").to_string(),
            ))
        })
        .collect())
}

fn mainnet_trees(limit: Option<&String>) -> Result<Vec<String>, String> {
    let root = workspace_root();
    let path = root.join("test-vectors/mainnet/scala_tx_json/diff_corpus.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| corpus_err(&path, e))?)
            .map_err(|e| e.to_string())?;
    let mut trees: Vec<String> = Vec::new();
    for tx in doc.as_array().ok_or("array")? {
        for out in tx["scalaJson"]["outputs"].as_array().unwrap_or(&vec![]) {
            if let Some(t) = out["ergoTree"].as_str() {
                if !trees.iter().any(|t0| t0 == t) {
                    trees.push(t.to_string());
                }
            }
        }
    }
    let limit: usize = limit.and_then(|s| s.parse().ok()).unwrap_or(trees.len());
    trees.truncate(limit);
    Ok(trees)
}

/// A JSON array of tree hex strings — any ad-hoc corpus, e.g. trees pulled
/// from the explorer.
fn trees_file(path: Option<&String>) -> Result<Vec<String>, String> {
    let path = path.ok_or("--trees needs a file path")?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn workspace_root() -> std::path::PathBuf {
    // Corpus recon modes expect the ergo node checkout sibling to this repo
    // (arkadianet/ergo · test-vectors/). Only used by `--seed` / `--mainnet`.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ergo")
}

fn corpus_err(path: &std::path::Path, e: std::io::Error) -> String {
    format!(
        "corpus file {} not readable ({e}) — the bundled corpora live in the \
         ergo node checkout (arkadianet/ergo); clone it as a sibling of this repo, \
         or pass tree hex directly",
        path.display()
    )
}

/// `ergo-es audit <tree-hex> | --seed | --mainnet [N]` — static lints over
/// the lifted tree.
fn cmd_audit(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("--seed") | Some("--mainnet") | Some("--trees") => {
            let is_seed = args[0] == "--seed";
            let (testnet, trees): (bool, Vec<String>) = if is_seed {
                (true, seed_vectors()?.into_iter().map(|(t, _)| t).collect())
            } else if args[0] == "--trees" {
                (false, trees_file(args.get(1))?)
            } else {
                (false, mainnet_trees(first_positional(args))?)
            };
            let mut flagged = 0usize;
            let mut findings_total = 0usize;
            let mut by_severity = std::collections::BTreeMap::<&str, usize>::new();
            let mut partial = 0usize;
            let mut parse_errors = 0usize;
            for h in &trees {
                let bytes = hex::decode(h).map_err(|e| e.to_string())?;
                let tree = match ergo_sandbox::inspect::parse_tree(&bytes) {
                    Ok(t) => t,
                    Err(_) => {
                        parse_errors += 1;
                        continue;
                    }
                };
                let lifted = ergo_sandbox::decompile::with_large_stack(move || {
                    ergo_sandbox::lift_tree(&tree, testnet)
                });
                let report = ergo_sandbox::audit::audit(&lifted);
                let n = report.findings.len();
                if n > 0 {
                    flagged += 1;
                    // Verification aid on stderr: the tally above stays the
                    // only stdout output, but the acceptance gate requires
                    // decompiling flagged trees by hand.
                    eprintln!("flagged {h}");
                }
                findings_total += n;
                for f in &report.findings {
                    *by_severity.entry(f.severity.label()).or_default() += 1;
                }
                if !matches!(
                    report.completeness,
                    ergo_sandbox::audit::Completeness::Complete
                ) {
                    partial += 1;
                }
            }
            let audited = trees.len() - parse_errors;
            let pct = if audited > 0 {
                100.0 * flagged as f64 / audited as f64
            } else {
                0.0
            };
            println!("audited: {audited} trees");
            println!("  flagged: {flagged} ({pct:.1}%)");
            println!("  findings: {findings_total}");
            for (sev, n) in &by_severity {
                println!("    {sev}: {n}");
            }
            println!("  partial: {partial}");
            if parse_errors > 0 {
                println!("  parse-errors: {parse_errors}");
            }
            Ok(())
        }
        Some(hex_arg) => {
            let bytes = hex::decode(hex_arg.trim()).map_err(|e| format!("bad hex: {e}"))?;
            let tree = ergo_sandbox::inspect::parse_tree(&bytes).map_err(|e| e.to_string())?;
            let lifted = ergo_sandbox::decompile::with_large_stack(move || {
                ergo_sandbox::lift_tree(&tree, false)
            });
            let report = ergo_sandbox::audit::audit(&lifted);

            match report.completeness {
                ergo_sandbox::audit::Completeness::Complete => {
                    println!("audit: {} finding(s)  [complete]", report.findings.len());
                }
                ergo_sandbox::audit::Completeness::Partial {
                    raw_placeholders,
                    truncated,
                } => {
                    println!(
                        "audit: {} finding(s)  [PARTIAL: {raw_placeholders} raw placeholder(s){}]",
                        report.findings.len(),
                        if truncated { ", truncated" } else { "" }
                    );
                    println!(
                        "  part of this contract was not analysed — no findings does not mean clean"
                    );
                }
            }
            for f in &report.findings {
                println!("\n{}  {}  node {}", f.severity.label(), f.lint, f.node_id);
                println!("  {}", f.message);
                println!("  {}", f.snippet);
            }
            Ok(())
        }
        None => Err("audit needs tree hex, --seed, or --mainnet [N]".into()),
    }
}

/// `ergo-es hunt <tree-hex> [--height H] [--self-box file] | --mainnet [N]`
/// — the spend hunt over the sandbox (P3b).
fn cmd_hunt(args: &[String]) -> Result<(), String> {
    use ergo_sandbox::hunt::{hunt, HuntOptions, HuntVerdict};

    let height = flag_value(args, "--height")?
        .map(|h| h.parse::<u32>().map_err(|e| format!("bad --height: {e}")))
        .transpose()?;
    let self_box = flag_value(args, "--self-box")?
        .map(|path| -> Result<ergo_sandbox::ScenarioBox, String> {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
        })
        .transpose()?;
    let data_inputs = flag_value(args, "--data-inputs")?
        .map(|path| -> Result<Vec<ergo_sandbox::ScenarioBox>, String> {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
        })
        .transpose()?
        .unwrap_or_default();
    let opts = HuntOptions {
        height,
        self_box,
        network: None,
        data_inputs,
    };

    match args.first().map(String::as_str) {
        Some("--mainnet") | Some("--trees") => {
            let trees = if args[0] == "--trees" {
                trees_file(args.get(1))?
            } else {
                // The corpus limit is the first positional AFTER --mainnet that
                // is not a flag or a flag's value (`--height 123` ≠ limit 123).
                let limit = positional_after_flags(
                    &args[1..],
                    &["--height", "--self-box", "--data-inputs"],
                );
                mainnet_trees(limit)?
            };
            let mut tally = std::collections::BTreeMap::<&str, usize>::new();
            let mut all_errored = 0usize;
            let mut parse_errors = 0usize;
            for h in &trees {
                let bytes = hex::decode(h).map_err(|e| e.to_string())?;
                let result = ergo_sandbox::decompile::with_large_stack({
                    let opts = opts.clone();
                    move || hunt(&bytes, &opts)
                });
                let r = match result {
                    Ok(r) => r,
                    Err(_) => {
                        parse_errors += 1;
                        continue;
                    }
                };
                let key = hunt_verdict_str(r.verdict);
                *tally.entry(key).or_default() += 1;
                if r.self_synthetic
                    && r.verdict == HuntVerdict::NotUnderProbes
                    && r.probes.iter().all(|p| p.error.is_some())
                {
                    // Every probe raised — with a synthetic SELF that is
                    // almost always a register read, i.e. undetermined, not
                    // guarded. Reported separately so the tally is honest.
                    all_errored += 1;
                }
                if matches!(
                    r.verdict,
                    HuntVerdict::SpendableByAnyone | HuntVerdict::MovableByAnyone
                ) {
                    eprintln!("{key}: {h}");
                }
            }
            let hunted = trees.len() - parse_errors;
            println!("hunted: {hunted} trees");
            for (k, n) in &tally {
                println!(
                    "  {k}: {n} ({:.1}%)",
                    100.0 * *n as f64 / hunted.max(1) as f64
                );
                if *k == hunt_verdict_str(HuntVerdict::NotUnderProbes) && all_errored > 0 {
                    println!("    of which every probe errored (synthetic SELF): {all_errored}");
                }
            }
            if parse_errors > 0 {
                println!("  parse-errors: {parse_errors}");
            }
            Ok(())
        }
        Some(hex_arg) if !hex_arg.starts_with("--") => {
            let bytes = hex::decode(hex_arg.trim()).map_err(|e| format!("bad hex: {e}"))?;
            let r = ergo_sandbox::decompile::with_large_stack(move || hunt(&bytes, &opts))
                .map_err(|e| e.to_string())?;
            println!("hunt: {}", hunt_verdict_str(r.verdict));
            if r.self_synthetic {
                println!(
                    "  SELF is synthetic (no registers, value 0) — pass --self-box for a real box"
                );
            }
            for res in &r.residuals {
                println!("  requires: {res}");
            }
            println!("  probes:");
            for p in &r.probes {
                let shape = match p.output {
                    ergo_sandbox::hunt::OutputShape::Attacker => "attacker",
                    ergo_sandbox::hunt::OutputShape::Preserve => "preserve",
                };
                let detail = match (&p.error, &p.reduced_to, p.verdict) {
                    (Some(e), _, _) => format!("  error: {e}"),
                    (None, Some(r), Verdict::NeedsProof) => format!("  -> {r}"),
                    _ => String::new(),
                };
                println!(
                    "    height {:>8}  {shape:<8}  {:<11}cost {}{detail}",
                    p.height,
                    verdict_str(p.verdict),
                    p.cost
                );
            }
            Ok(())
        }
        _ => Err("hunt needs tree hex or --mainnet [N]".into()),
    }
}

/// First positional argument, skipping `--flag value` pairs for the named
/// value-taking flags.
fn positional_after_flags<'a>(args: &'a [String], value_flags: &[&str]) -> Option<&'a String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if value_flags.contains(&a.as_str()) {
            it.next();
        } else if !a.starts_with("--") {
            return Some(a);
        }
    }
    None
}

fn hunt_verdict_str(v: ergo_sandbox::hunt::HuntVerdict) -> &'static str {
    use ergo_sandbox::hunt::HuntVerdict::*;
    match v {
        SpendableByAnyone => "spendable by anyone",
        MovableByAnyone => "movable by anyone",
        RequiresProof => "requires proof",
        NotUnderProbes => "not under probes",
    }
}

/// `ergo-es test <suite.json>` — the CI entry point for contract tests.
fn cmd_test(args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        return Err("test needs a suite JSON file (or - for stdin)".into());
    };
    let text = read_input(path)?;
    let suite: ergo_sandbox::testsuite::Suite =
        serde_json::from_str(&text).map_err(|e| format!("suite JSON: {e}"))?;
    let r = ergo_sandbox::decompile::with_large_stack(move || ergo_sandbox::testsuite::run(&suite))
        .map_err(|e| e.to_string())?;
    println!("contract: {}", r.address);
    for c in &r.cases {
        let mark = if c.passed { "ok  " } else { "FAIL" };
        let detail = match (&c.error, &c.reduced_to) {
            (Some(e), _) => format!("  ({e})"),
            (None, Some(red)) if c.actual == "needsProof" => format!("  -> {red}"),
            _ => String::new(),
        };
        println!(
            "  {mark}  {:<40} expected {:<13} got {:<13} cost {}{detail}",
            truncate(&c.name, 40),
            c.expected,
            c.actual,
            c.cost
        );
    }
    println!("{} passed, {} failed", r.passed, r.failed);
    if r.failed > 0 {
        Err(format!("{} case(s) failed", r.failed))
    } else {
        Ok(())
    }
}

/// `ergo-es validate-tx <request.json>` — the check right before signing.
fn cmd_validate_tx(args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        return Err("validate-tx needs a request JSON file (or - for stdin)".into());
    };
    let text = read_input(path)?;
    let req: ergo_sandbox::txcheck::TxRequest =
        serde_json::from_str(&text).map_err(|e| format!("request JSON: {e}"))?;
    let r = ergo_sandbox::decompile::with_large_stack(move || ergo_sandbox::txcheck::check(&req))
        .map_err(|e| e.to_string())?;
    println!("height:   {}", r.height);
    println!("ERG:      in {}  out {}", r.erg_in, r.erg_out);
    for i in &r.inputs {
        let detail = match (&i.error, &i.reduced_to) {
            (Some(e), _) => format!("  ({e})"),
            (None, Some(red)) if i.verdict == "needsProof" => format!("  -> {red}"),
            _ => String::new(),
        };
        println!(
            "  input {}  {:<11} {}{detail}",
            i.index,
            i.verdict,
            i.address.as_deref().unwrap_or(&i.box_id)
        );
    }
    for p in &r.problems {
        println!("  problem: {p}");
    }
    if r.valid {
        println!("valid: yes ({} signature(s) needed)", r.signatures_needed);
        Ok(())
    } else {
        println!("valid: NO");
        Err(format!("{} problem(s)", r.problems.len()))
    }
}

/// `ergo-es compose <spec.json> [--params p.json] [--suite out.json]`.
fn cmd_compose(args: &[String]) -> Result<(), String> {
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        return Err("compose needs a spec JSON file".into());
    };
    let text = read_input(path)?;
    let spec: ergo_sandbox::compose::Spec =
        serde_json::from_str(&text).map_err(|e| format!("spec JSON: {e}"))?;
    let params: std::collections::BTreeMap<String, ergo_sandbox::TypedValue> =
        match flag_value(args, "--params")? {
            Some(p) => {
                serde_json::from_str(&std::fs::read_to_string(&p).map_err(|e| format!("{p}: {e}"))?)
                    .map_err(|e| format!("{p}: {e}"))?
            }
            None => Default::default(),
        };
    let out = ergo_sandbox::compose::compose(&spec, &params).map_err(|e| e.to_string())?;
    print!("{}", out.source);
    if let Some(suite_path) = flag_value(args, "--suite")? {
        let Some(suite) = out.suite else {
            return Err("--suite needs --params (values make the suite)".into());
        };
        let json = serde_json::to_string_pretty(&suite).map_err(|e| e.to_string())?;
        std::fs::write(&suite_path, json + "\n").map_err(|e| format!("{suite_path}: {e}"))?;
        eprintln!("wrote {suite_path} ({} cases)", suite.scenarios.len());
    }
    Ok(())
}
