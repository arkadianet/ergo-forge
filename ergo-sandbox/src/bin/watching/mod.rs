use ergo_sandbox::{
    map::source::ChainBox,
    watch::{self, SourceInfo, WatchInput},
};
use std::collections::BTreeMap;

pub fn run(args: &[String]) -> Result<u8, String> {
    let mut files = Vec::new();
    let mut nfts = Vec::new();
    let mut registers = Vec::new();
    let mut baseline = None;
    let mut explorer = None;
    let mut json = false;
    let mut args = args.iter().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" if !json => json = true,
            "--nft" | "--register" | "--baseline" | "--explorer" => {
                let value = args
                    .next()
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| format!("{arg} needs a value"))?;
                match arg.as_str() {
                    "--nft" => nfts.push(value.clone()),
                    "--register" => {
                        registers.push(value.clone());
                        while args.peek().is_some_and(|v| {
                            matches!(v.as_str(), "R4" | "R5" | "R6" | "R7" | "R8" | "R9")
                        }) {
                            registers.push(args.next().unwrap().clone());
                        }
                    }
                    "--baseline" if baseline.is_none() => {
                        baseline = Some(
                            serde_json::from_str::<ChainBox>(&super::read_input(value)?)
                                .map_err(|e| format!("baseline box: {e}"))?,
                        );
                    }
                    "--explorer" if explorer.is_none() => explorer = Some(value.clone()),
                    _ => return Err(format!("repeated option: {arg}")),
                }
            }
            value if !value.starts_with('-') => files.push(value),
            _ => return Err(format!("unknown or repeated option: {arg}")),
        }
    }
    let inputs: Vec<_> = files
        .iter()
        .map(|file| {
            Ok(WatchInput {
                lockfile: serde_json::from_str(&super::read_input(file)?)
                    .map_err(|e| format!("lockfile: {e}"))?,
                nfts: nfts.clone(),
                registers: registers.clone(),
                baseline_boxes: baseline
                    .as_ref()
                    .map(|b| nfts.iter().map(|n| (n.clone(), b.clone())).collect())
                    .unwrap_or_else(BTreeMap::new),
            })
        })
        .collect::<Result<_, String>>()?;
    watch::validate(&inputs)?;
    // An explicit URL or environment configuration is required; no public default.
    let explorer = explorer
        .or_else(|| std::env::var("EXPLORER_URL").ok())
        .filter(|s| !s.trim().is_empty());
    let reports = match explorer {
        Some(url) => match super::tree_explorer(&url) {
            Ok(source) => watch::observe(&inputs, Some(source.as_ref())),
            Err(reason) => watch::unavailable(
                &inputs,
                SourceInfo {
                    kind: "explorer".into(),
                    url: Some(url),
                },
                &reason,
            ),
        },
        None => watch::observe(&inputs, None),
    }?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&reports).map_err(|e| e.to_string())?
        );
    } else {
        for report in &reports {
            println!("NFT {} · lock {}", report.nft, report.lockfile_fingerprint);
            println!(
                "Source {} {:?} · height {:?} · holder {:?}",
                report.chain_source.kind,
                report.chain_source.url,
                report.height,
                report.live.box_id
            );
            println!(
                "{}: {}",
                serde_json::to_value(report.live.status)
                    .unwrap()
                    .as_str()
                    .unwrap(),
                report.live.reason
            );
            for register in &report.registers {
                println!(
                    "{} {}: current={:?}, expected={:?}; {}",
                    register.name,
                    serde_json::to_value(register.status)
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    register.current,
                    register.expected,
                    register.reason
                );
            }
            println!("{}\n{}", report.limitation, report.observation);
        }
    }
    Ok(watch::exit_code(&reports))
}
