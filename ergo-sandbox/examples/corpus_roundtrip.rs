//! Decompile→recompile→byte-compare over the node's oracle-graded corpora.
fn main() {
    let mut exact = 0usize;
    let mut diff = 0usize;
    let mut raw = 0usize;
    let mut raw_cases: Vec<String> = Vec::new();
    let mut recompile_err = 0usize;
    let mut env_skip = 0usize;

    // 1. compile corpus (source + tree pairs) — bar: decompile recompiles to the SAME bytes
    let doc: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            "/home/rkadias/coding/ergo/test-vectors/ergoscript/compile/compile_seed.json",
        )
        .unwrap(),
    )
    .unwrap();
    for v in doc["vectors"].as_array().unwrap() {
        if v["oracle"].as_str() != Some("ACCEPT") {
            continue;
        }
        let Some(h) = v["tree_hex"].as_str() else {
            continue;
        };
        let bytes = hex::decode(h).unwrap();
        // The oracle captured some vectors with a demo env (x, height1, b1,
        // col1, g1...). Those don't compile with the sandbox's empty env —
        // skip them (the ergo repo's compile_semantic_parity covers them).
        let orig = v["source"].as_str().unwrap_or("");
        if ergo_sandbox::compile_source(
            orig,
            v["tree_version"].as_u64().unwrap_or(3) as u8,
            ergo_ser::address::NetworkPrefix::Testnet,
        )
        .is_err()
        {
            env_skip += 1;
            continue;
        }
        // Recompile-parity requires the decompiled source to compile under the
        // SAME tree_version the oracle used (cast lowering differs by version).
        if v["tree_version"].as_u64() != Some(3) {
            env_skip += 1;
            continue;
        }
        let src = ergo_sandbox::decompile::decompile_bytes(&bytes).unwrap();
        if src.contains("<unparsed")
            || src.contains("<op ")
            || src.contains("<method ")
            || src.contains("<const ")
        {
            raw += 1;
            raw_cases.push(format!(
                "RAW  {}",
                v["source"].as_str().unwrap_or("?").replace('\n', " ")
            ));
            continue;
        }
        match ergo_sandbox::compile_source(
            &src,
            v["tree_version"].as_u64().unwrap_or(3) as u8,
            ergo_ser::address::NetworkPrefix::Testnet,
        ) {
            Ok(out) if out.tree_bytes == bytes => exact += 1,
            Ok(out) => {
                diff += 1;
                if diff <= 8 {
                    println!(
                        "DIFF  {}\n  want {}\n  got  {}",
                        v["source"].as_str().unwrap_or("?").replace('\n', " "),
                        h,
                        hex::encode(&out.tree_bytes)
                    );
                }
            }
            Err(e) => {
                recompile_err += 1;
                if recompile_err <= 8 {
                    println!(
                        "ERR   {}\n  src: {}\n  {}",
                        v["source"].as_str().unwrap_or("?").replace('\n', " "),
                        src.replace('\n', " "),
                        e
                    );
                }
            }
        }
    }
    println!("compile corpus: exact={exact} diff={diff} raw={raw} err={recompile_err} env-skip={env_skip}");
    for r in raw_cases.iter().take(5) {
        println!("  {r}");
    }

    // 2. mainnet trees — bar: parses, prints, no panic (byte-exact round-trip
    //    not expected for hand-built trees; raw placeholders are honest)
    let doc: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            "/home/rkadias/coding/ergo/test-vectors/mainnet/scala_tx_json/diff_corpus.json",
        )
        .unwrap(),
    )
    .unwrap();
    let mut trees: Vec<String> = Vec::new();
    for tx in doc.as_array().unwrap() {
        for out in tx["scalaJson"]["outputs"].as_array().unwrap_or(&vec![]) {
            if let Some(t) = out["ergoTree"].as_str() {
                if !trees.contains(&t.to_string()) {
                    trees.push(t.to_string());
                }
            }
        }
    }
    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut m_exact = 0usize;
    let mut m_raw = 0usize;
    for t in &trees {
        let bytes = hex::decode(t).unwrap();
        match ergo_sandbox::decompile::decompile_bytes(&bytes) {
            Ok(src) => {
                ok += 1;
                if src.contains("<unparsed")
                    || src.contains("<op ")
                    || src.contains("<method ")
                    || src.contains("<const ")
                {
                    m_raw += 1;
                }
                if let Ok(out) =
                    ergo_sandbox::compile_source(&src, 0, ergo_ser::address::NetworkPrefix::Mainnet)
                {
                    if out.tree_bytes == bytes {
                        m_exact += 1;
                    }
                }
            }
            Err(e) => {
                fail += 1;
                println!("mainnet FAIL {e}");
            }
        }
    }
    println!("mainnet trees: printed={ok} failed={fail} recompiled-exact={m_exact} with-raw={m_raw} (of {})", trees.len());
}
