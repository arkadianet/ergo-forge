//! Pins the P2.5 AST split: the tree-shaped API and the text-shaped API must
//! agree, and lift diagnostics must be readable off the tree rather than by
//! scanning rendered text.

use ergo_sandbox::{compile_source, decompile};
use ergo_ser::address::NetworkPrefix;
use ergo_ser::ergo_tree::ErgoTree;
use ergo_ser::opcode::Expr;
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::SigmaValue;

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

/// A constant whose Tuple value nests deeper than MAX_LIFT_DEPTH must
/// truncate to the Raw placeholder, not overflow the stack. The parser caps
/// constant-value nesting at 110 (< 128), so this is only reachable through
/// a hand-built tree — exactly the input class the lift ceiling exists for.
#[test]
fn deeply_nested_constant_truncates_instead_of_overflowing() {
    let mut tpe = SigmaType::SInt;
    let mut val = SigmaValue::Int(1);
    for _ in 0..200 {
        val = SigmaValue::Tuple(vec![val]);
        tpe = SigmaType::STuple(vec![tpe]);
    }
    let tree = ErgoTree {
        version: 3,
        has_size: false,
        constant_segregation: false,
        constants: vec![],
        body: Expr::Const { tpe, val },
    };
    let lifted = decompile::lift_tree(&tree, true);
    assert!(lifted.truncated, "deep constant must set truncated");
    assert!(
        lifted.raw_placeholders > 0,
        "deep constant must degrade to a Raw placeholder"
    );
    let rendered = decompile::print(&lifted.node);
    assert!(
        rendered.contains("<nesting deeper than 128 levels>"),
        "rendered: {rendered}"
    );
}

// ----- shared IR ids (P5-B consumer contract) -----

/// Every lifted node that stands for an IR node carries that node's id from
/// `ergo_ser::opcode::preorder` — the same walk the compiler's source map is
/// keyed by — in `Lifted::ir_ids` (lift id → IR id).
#[test]
fn lifted_nodes_carry_shared_preorder_ir_ids() {
    let bytes = ergo_sandbox::compile_source("sigmaProp(HEIGHT > 100)", 3, NetworkPrefix::Testnet)
        .unwrap()
        .tree_bytes;
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).unwrap();
    let lifted = ergo_sandbox::lift_tree(&tree, true);
    let n = ergo_ser::opcode::preorder(&tree.body).count() as u64;
    // The root D1 wrapper is stripped by the lift, so the lifted root stands
    // for IR node 1 (the comparison); its children are 2 (HEIGHT) and 3.
    assert_eq!(lifted.ir_ids.get(&lifted.node.id), Some(&1));
    let child_ir: Vec<u64> = ergo_sandbox::audit::children(&lifted.node)
        .iter()
        .filter_map(|c| lifted.ir_ids.get(&c.id).copied())
        .collect();
    assert_eq!(child_ir, vec![2, 3]);
    assert!(lifted.ir_ids.values().all(|&i| i < n));
}
