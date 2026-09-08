//! Record a protocol-map chain fixture from the live explorer, once.
//!
//! The map's tests run offline against committed fixtures; this is the only
//! thing that ever talks to a chain, and it is run by hand:
//!
//! ```text
//! cargo run -p ergo-sandbox --example record_map_fixture -- \
//!     <seed> <out.json> [--depth N] [--max-nodes N] [--explorer URL]
//! ```
//!
//! Read-only: it fetches, it never submits.

#[cfg(not(feature = "explorer"))]
fn main() {
    eprintln!("build with --features explorer to record a fixture");
    std::process::exit(2);
}

#[cfg(feature = "explorer")]
fn main() {
    use ergo_sandbox::map::explorer::{ExplorerSource, RecordingSource, DEFAULT_EXPLORER_URL};
    use ergo_sandbox::map::{map, MapOptions, Seed};

    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let positional: Vec<&String> = {
        let mut out = Vec::new();
        let mut skip = false;
        for a in &args {
            if skip {
                skip = false;
                continue;
            }
            if a.starts_with("--") {
                skip = true;
            } else {
                out.push(a);
            }
        }
        out
    };
    let (Some(seed_text), Some(out_path)) = (positional.first(), positional.get(1)) else {
        eprintln!("usage: record_map_fixture <seed> <out.json> [--depth N] [--max-nodes N]");
        std::process::exit(2);
    };

    let mut opts = MapOptions::default();
    if let Some(d) = flag("--depth") {
        opts.max_depth = d.parse().expect("--depth");
    }
    if let Some(n) = flag("--max-nodes") {
        opts.max_nodes = n.parse().expect("--max-nodes");
    }
    let base = flag("--explorer").unwrap_or_else(|| DEFAULT_EXPLORER_URL.to_string());
    let seed = match flag("--tx") {
        Some(tx) => Seed::TransactionId(tx),
        None => Seed::guess(seed_text),
    };

    let recorder = RecordingSource::new(ExplorerSource::new(&base)).expect("reach the explorer");
    let m = map(&recorder, &seed, &opts).expect("map");
    eprintln!(
        "recorded {} nodes, {} edges, {} findings at height {}",
        m.nodes.len(),
        m.edges.len(),
        m.findings.len(),
        m.height
    );
    let fixture = recorder.finish();
    std::fs::write(out_path, fixture.to_json().expect("serialize")).expect("write fixture");
}
