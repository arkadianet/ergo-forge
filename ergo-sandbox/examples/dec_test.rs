fn main() {
    let cases: &[(&str, &str)] = &[
        ("100104c801d191a37300", "HEIGHT > 100"),
        ("1001040ad191a37300", "{ val x = HEIGHT; x > 5 }"),
        ("1002040a0412d1ed91a373008fa37301", "HEIGHT>5 && HEIGHT<9"),
        (
            "0008cd0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            "P2PK",
        ),
        ("10010100d17300", "!true"),
        ("100204020404d19373007301", "1 == 2"),
    ];
    for (hexs, label) in cases {
        let bytes = hex::decode(hexs).unwrap();
        let src = ergo_sandbox::decompile::decompile_bytes(&bytes).unwrap();
        println!("{label}: {src}");
        // recompile and compare
        match ergo_sandbox::compile_source(&src, 3, ergo_ser::address::NetworkPrefix::Testnet) {
            Ok(out) => {
                let same = hex::encode(&out.tree_bytes) == *hexs;
                println!(
                    "  roundtrip: {}",
                    if same {
                        "EXACT".to_string()
                    } else {
                        format!("DIFF {}", hex::encode(&out.tree_bytes))
                    }
                );
            }
            Err(e) => println!("  recompile FAILED: {e}"),
        }
    }
}
