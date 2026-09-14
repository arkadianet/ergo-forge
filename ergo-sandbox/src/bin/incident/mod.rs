use std::collections::BTreeMap;
use std::path::Path;

pub fn run(args: &[String]) -> Result<(), String> {
    let tx_id = args
        .first()
        .filter(|s| !s.starts_with("--"))
        .ok_or("incident requires a txid and --explorer URL")?;
    let mut opts = BTreeMap::new();
    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        if !matches!(flag.as_str(), "--explorer" | "--out" | "--network")
            || opts.contains_key(flag.as_str())
        {
            return Err(format!("unknown or repeated incident option: {flag}"));
        }
        let value = rest
            .next()
            .filter(|s| !s.starts_with("--"))
            .ok_or_else(|| format!("{flag} needs a value"))?;
        opts.insert(flag.as_str(), value.as_str());
    }
    let url = opts
        .get("--explorer")
        .ok_or("incident requires --explorer URL")?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("--explorer must be an http(s) URL".into());
    }
    let network = opts.get("--network").copied().unwrap_or("mainnet");
    let default_out = format!("incident-{tx_id}");
    let out = Path::new(opts.get("--out").copied().unwrap_or(&default_out));
    if out.exists() {
        return Err(
            "incident output directory already exists; choose a new --out directory".into(),
        );
    }
    #[cfg(feature = "explorer")]
    {
        let source = ergo_sandbox::map::explorer::ExplorerSource::new(url);
        let draft = ergo_sandbox::incident::scaffold(&source, tx_id, network)?;
        draft.write_to(out)?;
        println!("Wrote {} script suites to {}. Expectations require an author; no evaluation or broadcast occurred.", draft.suites.len(), out.display());
        Ok(())
    }
    #[cfg(not(feature = "explorer"))]
    {
        let _ = (url, network, out);
        Err("incident requires a build with the explorer feature".into())
    }
}
