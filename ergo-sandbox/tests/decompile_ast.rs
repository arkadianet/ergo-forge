//! Pins the P2.5 AST split: the tree-shaped API and the text-shaped API must
//! agree, and lift diagnostics must be readable off the tree rather than by
//! scanning rendered text.

use ergo_sandbox::{compile_source, decompile};
use ergo_ser::address::NetworkPrefix;

/// Compile a source string to tree bytes, then parse it back to a tree.
fn tree_of(src: &str) -> ergo_ser::ergo_tree::ErgoTree {
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    ergo_sandbox::inspect::parse_tree(&bytes).expect("parse")
}

/// lift + print must reproduce exactly what the fused path renders. If these
/// diverge, the split introduced drift.
#[test]
fn lift_then_print_matches_the_fused_render() {
    for src in [
        "sigmaProp(HEIGHT > 100)",
        "(1 + 2 * 3 - 4) / 5 == 0",
        "sigmaProp(OUTPUTS.size > 1 && INPUTS.size == 1)",
    ] {
        let tree = tree_of(src);
        let fused = decompile::render_report_net(&tree, true);
        let split = decompile::print(&decompile::lift_tree(&tree, true).node);
        assert_eq!(split, fused.source, "source: {src}");
    }
}

/// The two return shapes must not be able to disagree about diagnostics.
#[test]
fn lifted_and_decompiled_report_the_same_placeholder_count() {
    for src in [
        "sigmaProp(HEIGHT > 100)",
        "sigmaProp(OUTPUTS.size > 1 && INPUTS.size == 1)",
    ] {
        let tree = tree_of(src);
        let lifted = decompile::lift_tree(&tree, true);
        let report = decompile::render_report_net(&tree, true);
        assert_eq!(
            lifted.raw_placeholders, report.raw_placeholders,
            "source: {src}"
        );
        assert_eq!(lifted.truncated, report.truncated, "source: {src}");
    }
}

/// Ids are unique within one decompilation — lints rely on this to dedupe and
/// cross-reference findings.
#[test]
fn lift_local_ids_are_unique_within_one_tree() {
    fn collect(n: &decompile::Node, out: &mut Vec<u64>) {
        out.push(n.id);
        match &n.kind {
            decompile::NodeKind::Unary(_, a) => collect(a, out),
            decompile::NodeKind::Infix(_, a, b) => {
                collect(a, out);
                collect(b, out);
            }
            decompile::NodeKind::If(c, t, e) => {
                collect(c, out);
                collect(t, out);
                collect(e, out);
            }
            _ => {}
        }
    }

    let tree = tree_of("sigmaProp(if (HEIGHT > 100) OUTPUTS.size > 1 else INPUTS.size == 1)");
    let lifted = decompile::lift_tree(&tree, true);
    let mut ids = Vec::new();
    collect(&lifted.node, &mut ids);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "duplicate lift ids: {ids:?}");
}
