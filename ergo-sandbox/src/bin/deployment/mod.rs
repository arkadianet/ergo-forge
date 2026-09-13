use ergo_sandbox::{decompile::with_large_stack, TypedValue};
use ergo_ser::address::NetworkPrefix;
use std::collections::BTreeMap;

// Strict option parsing shared by the deployment commands. Values beginning
// with `--` are refused, so a missing value cannot consume another option.
fn options<'a>(
    args: &'a [String],
    allowed: &[&str],
) -> Result<(&'a str, BTreeMap<&'a str, &'a str>), String> {
    let input = args
        .first()
        .filter(|s| !s.starts_with("--"))
        .ok_or("missing input")?;
    let mut result = BTreeMap::new();
    let mut flags = args[1..].iter();
    while let Some(flag) = flags.next() {
        if !allowed.contains(&flag.as_str()) || result.contains_key(flag.as_str()) {
            return Err(format!("unknown or repeated option: {flag}"));
        }
        let value = if flag == "--json" {
            "true"
        } else {
            flags
                .next()
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| format!("{flag} needs a value"))?
                .as_str()
        };
        result.insert(flag.as_str(), value);
    }
    Ok((input, result))
}
fn params(path: Option<&&str>) -> Result<BTreeMap<String, TypedValue>, String> {
    path.map(|path| {
        serde_json::from_str(&super::read_input(path)?).map_err(|e| format!("params: {e}"))
    })
    .unwrap_or_else(|| Ok(BTreeMap::new()))
}

pub fn verify(args: &[String]) -> Result<u8, String> {
    let (target, opts) = options(args, &["--source", "--params", "--network", "--json"])?;
    let source = super::read_input(
        opts.get("--source")
            .ok_or("verify requires --source f.es")?,
    )?;
    let params = params(opts.get("--params"))?;
    let network = opts
        .get("--network")
        .map(|n| super::parse_network(n))
        .transpose()?
        .unwrap_or(NetworkPrefix::Mainnet);
    let target = target.to_string();
    let report = with_large_stack(move || {
        ergo_sandbox::verify::verify(&target, &source, &params, 3, network)
    })
    .map_err(|e| e.to_string())?;
    if opts.contains_key("--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "{}",
            serde_json::to_value(report.outcome)
                .map_err(|e| e.to_string())?
                .as_str()
                .unwrap()
        );
        for diff in &report.constant_differences {
            println!(
                "Constant at {:?}: left={:?}; right={:?}",
                diff.path, diff.left, diff.right
            );
        }
        println!("{}", report.limitation);
    }
    Ok(report.outcome.exit_code())
}

fn compile_options(opts: &BTreeMap<&str, &str>) -> Result<(u8, NetworkPrefix), String> {
    let version = opts
        .get("--tree-version")
        .map(|v| v.parse::<u8>().map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or(3);
    let network = opts
        .get("--network")
        .map(|n| super::parse_network(n))
        .transpose()?
        .unwrap_or(NetworkPrefix::Mainnet);
    Ok((version, network))
}

pub fn lock(args: &[String]) -> Result<u8, String> {
    let (input, opts) = options(args, &["--params", "--network", "--tree-version", "--out"])?;
    let source = super::read_input(input)?;
    let params = params(opts.get("--params"))?;
    let (version, network) = compile_options(&opts)?;
    let lock = with_large_stack(move || {
        ergo_sandbox::lockfile::create(&source, &params, version, network)
    })
    .map_err(|e| e.to_string())?;
    let path = opts.get("--out").copied().unwrap_or("contract.lock.json");
    let text = serde_json::to_string_pretty(&lock).map_err(|e| e.to_string())? + "\n";
    std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))?;
    println!("Wrote {path}\n{}", lock.limitation);
    Ok(0)
}

pub fn verify_lock(args: &[String]) -> Result<u8, String> {
    use ergo_sandbox::lockfile::{compare_live, LiveComparison, LiveStatus, Lockfile};
    let (input, opts) = options(
        args,
        &[
            "--source",
            "--params",
            "--network",
            "--tree-version",
            "--explorer",
            "--nft",
            "--json",
        ],
    )?;
    let locked: Lockfile =
        serde_json::from_str(&super::read_input(input)?).map_err(|e| format!("lockfile: {e}"))?;
    let source = super::read_input(
        opts.get("--source")
            .ok_or("verify-lock requires --source f.es")?,
    )?;
    let params = params(opts.get("--params"))?;
    let (version, network) = compile_options(&opts)?;
    let mut report = with_large_stack({
        let locked = locked.clone();
        move || ergo_sandbox::lockfile::verify_lock(&locked, &source, &params, version, network)
    })
    .map_err(|e| e.to_string())?;
    let explorer = opts
        .get("--explorer")
        .map(|s| s.to_string())
        .or_else(|| std::env::var("EXPLORER_URL").ok())
        .filter(|s| !s.trim().is_empty());
    let nft = opts.get("--nft").copied();
    if opts.contains_key("--explorer") && nft.is_none() {
        return Err("--explorer requires --nft".into());
    }
    report.live = match (nft, explorer) {
        (Some(_), Some(url)) => match super::tree_explorer(&url) {
            Ok(source) => compare_live(&locked, nft, Some(source.as_ref())),
            Err(e) => LiveComparison::unverified(e),
        },
        _ => compare_live(&locked, nft, None),
    };
    let code = if !report.drifts.is_empty() || report.live.status == LiveStatus::BytesDiffer {
        5
    } else if nft.is_some() && report.live.status == LiveStatus::Unverified {
        6
    } else {
        0
    };
    if opts.contains_key("--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "Local lock: {}",
            if report.drifts.is_empty() {
                "current"
            } else {
                "drift"
            }
        );
        for drift in &report.drifts {
            println!(
                "Drift {:?} ({}): locked={}; current={}",
                drift.kind, drift.field, drift.locked, drift.current
            );
        }
        println!(
            "Live: {} — {}",
            serde_json::to_value(report.live.status)
                .map_err(|e| e.to_string())?
                .as_str()
                .unwrap(),
            report.live.reason
        );
        println!("{}", report.limitation);
    }
    Ok(code)
}
