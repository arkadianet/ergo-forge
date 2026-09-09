//! Independent forge ground truth: run the audit layer over the Lithos
//! contracts myself, so a "forge found this" claim is my own tool output.
use std::collections::BTreeMap;
use ergo_sandbox::{audit, compile::compile_with_params, lift_tree, TypedValue};
use ergo_ser::address::NetworkPrefix;

fn main() {
    let root = std::env::args().nth(1).expect("usage: <resources dir>");
    let mut files: Vec<_> = walk(&root);
    files.sort();
    let (mut ok, mut failed) = (0, 0);
    for f in &files {
        let src = std::fs::read_to_string(f).unwrap();
        // Their build substitutes bare CONST_* identifiers; ours needs them
        // bound. Anything still unbound is reported, never silently skipped.
        let params: BTreeMap<String, TypedValue> = infer_params(&src);
        match compile_with_params(&src, &params, 3, NetworkPrefix::Mainnet) {
            Ok(o) => {
                ok += 1;
                let tree = ergo_sandbox::inspect::parse_tree(&o.tree_bytes).unwrap();
                let lifted = lift_tree(&tree, false);
                let a = audit::audit(&lifted);
                let name = f.rsplit('/').next().unwrap();
                if a.findings.is_empty() {
                    println!("CLEAN    {name}");
                } else {
                    for x in &a.findings {
                        println!("FINDING  {name}  {}  {:?}  {}", x.lint, x.severity,
                            x.message.chars().take(150).collect::<String>());
                    }
                }
            }
            Err(e) => {
                failed += 1;
                println!("NOCOMPILE {}  {}", f.rsplit('/').next().unwrap(),
                    format!("{e}").chars().take(110).collect::<String>());
            }
        }
    }
    println!("\nSUMMARY compiled={ok} not_compiled={failed} total={}", files.len());
}

/// Their build binds bare `CONST_*` identifiers via ErgoScript's named-constant
/// substitution. For static review the VALUES do not matter — only the shapes —
/// so supply typed placeholders inferred from the name. Anything mis-inferred
/// shows up as a compile error naming the constant, never as a silent wrong tree.
fn infer_params(src: &str) -> BTreeMap<String, TypedValue> {
    let mut out = BTreeMap::new();
    let mut names: Vec<String> = vec![];
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if src.is_char_boundary(i) && src[i..].starts_with("CONST_") {
            let end = src[i..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .map(|e| i + e)
                .unwrap_or(bytes.len());
            names.push(src[i..end].to_string());
            i = end;
        } else {
            i += 1;
            while i < bytes.len() && !src.is_char_boundary(i) {
                i += 1;
            }
        }
    }
    names.sort();
    names.dedup();
    for n in names {
        // A declared `val CONST_X = ...` is bound in-source; only free ones are params.
        if src.contains(&format!("val {n} ")) || src.contains(&format!("val {n}=")) {
            continue;
        }
        let coll = n.ends_with("_NFT") || n.ends_with("_HASH") || n.ends_with("_ID")
            || n.ends_with("_TOKEN") || n.ends_with("_ERGOTREE") || n.ends_with("_PK")
            || n.contains("SCRIPT");
        let tv = if coll {
            TypedValue { r#type: "Coll[Byte]".into(),
                value: serde_json::json!("11".repeat(32)) }
        } else {
            TypedValue { r#type: "Long".into(), value: serde_json::json!(1000i64) }
        };
        out.insert(n, tv);
    }
    out
}

fn walk(d: &str) -> Vec<String> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { out.extend(walk(p.to_str().unwrap())); }
            else if p.extension().map(|x| x == "ergo").unwrap_or(false) {
                out.push(p.to_str().unwrap().to_string());
            }
        }
    }
    out
}
