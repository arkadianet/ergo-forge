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

/// Lint ids present in an audit of `src`.
fn lints_of(src: &str) -> Vec<&'static str> {
    audit::audit(&lifted(src))
        .findings
        .iter()
        .map(|f| f.lint)
        .collect()
}

#[test]
fn bare_register_get_is_flagged() {
    assert_eq!(
        lints_of("sigmaProp(SELF.R4[Int].get > 5)"),
        vec!["unchecked-get"]
    );
}

#[test]
fn is_defined_conjunction_guards_the_get() {
    assert!(lints_of("sigmaProp(SELF.R4[Int].isDefined && SELF.R4[Int].get > 5)").is_empty());
}

#[test]
fn is_defined_conditional_guards_the_get() {
    assert!(
        lints_of("sigmaProp(if (SELF.R4[Int].isDefined) SELF.R4[Int].get > 5 else false)")
            .is_empty()
    );
}

#[test]
fn get_or_else_is_never_flagged() {
    assert!(lints_of("sigmaProp(OUTPUTS(0).R4[Long].getOrElse(0L) > 5L)").is_empty());
}

#[test]
fn two_unguarded_gets_produce_two_findings_with_distinct_nodes() {
    let a = audit::audit(&lifted(
        "sigmaProp(SELF.R4[Int].get > 5 && SELF.R5[Int].get > 6)",
    ));
    assert_eq!(a.findings.len(), 2, "{:?}", a.findings);
    assert_ne!(a.findings[0].node_id, a.findings[1].node_id);
}

#[test]
fn findings_carry_a_readable_snippet() {
    let a = audit::audit(&lifted("sigmaProp(SELF.R4[Int].get > 5)"));
    assert!(
        a.findings[0].snippet.contains("get"),
        "snippet: {}",
        a.findings[0].snippet
    );
    assert_eq!(a.findings[0].severity, ergo_sandbox::Severity::High);
}
