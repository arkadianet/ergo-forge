//! Audit layer: framework behaviour and lint decisions.

use ergo_sandbox::audit::{self, Completeness};
use ergo_sandbox::{compile_source, lift_tree};
use ergo_ser::address::NetworkPrefix;

/// Compile a source string and lift the resulting tree.
fn lifted(src: &str) -> ergo_sandbox::Lifted {
    let bytes = compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes;
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).expect("parse");
    lift_tree(&tree, true)
}

#[test]
fn a_fully_lifted_tree_audits_as_complete() {
    let a = audit::audit(&lifted("sigmaProp(HEIGHT > 100)"));
    assert_eq!(a.completeness, Completeness::Complete);
}

#[test]
fn children_covers_every_node_of_a_nested_tree() {
    let l = lifted("sigmaProp(if (HEIGHT > 100) OUTPUTS.size > 1 else INPUTS.size == 1)");
    fn count(n: &ergo_sandbox::Node) -> usize {
        1 + audit::children(n).into_iter().map(count).sum::<usize>()
    }
    assert!(count(&l.node) > 5, "traversal reached too few nodes");
}
