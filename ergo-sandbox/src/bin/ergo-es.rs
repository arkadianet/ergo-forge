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
        "eval" => cmd_eval(rest),
        "decompile" => cmd_decompile(rest),
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
      Compile ErgoScript source to ErgoTree bytes + P2S/P2SH addresses.
  ergo-es eval <scenario.json>
      Evaluate a scenario: contract (source or tree hex) + spending context
      → verdict, cost, trace. See README scenario schema.
  ergo-es decompile <hex | file | --seed | --mainnet [N]>
      Print the structural view of ErgoTree bytes (--seed / --mainnet run
      the bundled corpora instead).
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
    let out = compile_source(&source, tree_version, network).map_err(|e| e.to_string())?;
    println!("tree:     {}", hex::encode(&out.tree_bytes));
    println!("p2s:      {}", out.p2s_address);
    println!("p2sh:     {}", out.p2sh_address);
    Ok(())
}

// ── eval ─────────────────────────────────────────────────────────────────────

fn cmd_eval(args: &[String]) -> Result<(), String> {
    let Some(path) = args.first() else {
        return Err("eval needs a scenario JSON file (or - for stdin)".into());
    };
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
    for c in &outcome.cost_breakdown {
        println!("  cost {:>6} {} (total {})", c.delta, c.label, c.total);
    }
    Ok(())
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
    for (i, t) in trees.iter().enumerate() {
        let bytes = hex::decode(t).map_err(|e| e.to_string())?;
        println!("--- mainnet #{i}");
        let report = inspect::tree_report(&bytes).map_err(|e| e.to_string())?;
        print!("{report}");
    }
    Ok(())
}

fn workspace_root() -> std::path::PathBuf {
    // Corpus recon modes expect the ergo node checkout sibling to this repo
    // (arkadianet/ergo · test-vectors/). Only used by `--seed` / `--mainnet`.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ergo")
}

fn corpus_err(path: &std::path::Path, e: std::io::Error) -> String {
    format!(
        "corpus file {} not readable ({e}) — the bundled corpora live in the \
         ergo node checkout (arkadianet/ergo); clone it as a sibling of this repo, \
         or pass tree hex directly",
        path.display()
    )
}
