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

/// Guard collection is conjunctive-only: an `isDefined` under `||` proves
/// nothing (`x.isDefined || y.isDefined` can hold with `x` empty), so the
/// `get` must still be flagged.
#[test]
fn is_defined_under_or_does_not_guard_the_get() {
    assert_eq!(
        lints_of(
            "sigmaProp((SELF.R4[Int].isDefined || SELF.R5[Int].isDefined) && SELF.R4[Int].get > 5)"
        ),
        vec!["unchecked-get"]
    );
}

/// `!x.isDefined` asserts the OPPOSITE — walking into it must not guard.
#[test]
fn negated_is_defined_does_not_guard_the_get() {
    assert_eq!(
        lints_of("sigmaProp(!(SELF.R4[Int].isDefined) && SELF.R4[Int].get > 5)"),
        vec!["unchecked-get"]
    );
}

/// The conjunctive descent that replaced the arbitrary walk still guards
/// through nested `&&` chains.
#[test]
fn is_defined_nested_in_conjunction_still_guards() {
    assert!(lints_of(
        "sigmaProp((HEIGHT > 100 && SELF.R4[Int].isDefined) && SELF.R4[Int].get > 5)"
    )
    .is_empty());
}

// ── severity tiers: who controls the receiver ──────────────────────────────

fn findings_of(src: &str) -> Vec<ergo_sandbox::Finding> {
    audit::audit(&lifted(src)).findings
}

/// A context variable is supplied by the spender with the proof; a missing
/// one fails the spend but cannot lock the box — the spender just supplies it.
#[test]
fn a_get_on_a_context_variable_is_low() {
    let f = findings_of("sigmaProp(getVar[Int](0).get > 5)");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].lint, "unchecked-get");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::Low);
    assert!(
        f[0].message.contains("context variable 0"),
        "{}",
        f[0].message
    );
}

/// Inside a lambda over a collection the receiver is an element the spender
/// chose (inputs, outputs, data inputs) — fragile, not a lock.
#[test]
fn a_get_on_a_lambda_element_is_medium() {
    let f = findings_of("sigmaProp(OUTPUTS.exists { (b: Box) => b.R4[Int].get > 0 })");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::Medium);
    assert!(f[0].message.contains("element"), "{}", f[0].message);
}

/// A guard inside the lambda body still clears the element read.
#[test]
fn a_guarded_lambda_element_get_is_not_flagged() {
    assert!(findings_of(
        "sigmaProp(OUTPUTS.exists { (b: Box) => b.R4[Int].isDefined && b.R4[Int].get > 0 })"
    )
    .is_empty());
}

/// SELF inside a lambda body is still SELF — the tier follows the receiver's
/// root, not the lexical position.
#[test]
fn a_self_get_inside_a_lambda_stays_high() {
    let f = findings_of("sigmaProp(OUTPUTS.exists { (b: Box) => b.value > SELF.R4[Long].get })");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::High);
}

/// A get whose receiver is indexed off a lambda element is still the element's.
#[test]
fn a_get_through_an_element_property_chain_is_medium() {
    let f = findings_of(
        "sigmaProp(INPUTS.forall { (b: Box) => b.tokens(0)._1 == SELF.id && b.R5[Long].get > 0L })",
    );
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, ergo_sandbox::Severity::Medium);
}

// ── guards held in a val ───────────────────────────────────────────────────
//
// A single-use `val` is inlined by the compiler, so only multi-use guards
// survive to the tree (as `val v2 = v.isDefined; … v2 && … && v2`). The
// lint must follow the binding.

#[test]
fn a_guard_bound_to_a_val_clears_the_get_under_and() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined; sigmaProp(ok && SELF.R4[Int].get > 5 && ok) }"
    )
    .is_empty());
}

#[test]
fn a_guard_bound_to_a_val_clears_the_get_in_the_then_branch() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined && HEIGHT > 10; sigmaProp(if (ok) SELF.R4[Int].get > 5 else ok) }"
    )
    .is_empty());
}

#[test]
fn a_val_guard_under_or_still_proves_nothing() {
    assert_eq!(
        findings_of(
            "{ val ok = SELF.R4[Int].isDefined || SELF.R5[Int].isDefined; sigmaProp(ok && SELF.R4[Int].get > 5 && ok) }"
        )
        .len(),
        1
    );
}

#[test]
fn a_val_guard_does_not_clear_a_get_outside_its_scope() {
    assert_eq!(
        findings_of(
            "{ val ok = SELF.R4[Int].isDefined; sigmaProp(SELF.R4[Int].get > 5 && ok && ok) }"
        )
        .len(),
        1
    );
}

#[test]
fn a_val_guard_composed_of_two_guards_clears_both_gets() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined && SELF.R5[Long].isDefined; sigmaProp(ok && SELF.R4[Int].get > 5 && SELF.R5[Long].get > 1L && ok) }"
    )
    .is_empty());
}

// ── guards through || and negation ────────────────────────────────────────
//
// `!x.isDefined || x.get > 5` runs the right operand only when x is defined;
// `if (!x.isDefined) a else x.get` reaches the else branch the same way.

#[test]
fn a_negated_guard_under_or_clears_the_right_operand() {
    assert!(findings_of("sigmaProp(!(SELF.R4[Int].isDefined) || SELF.R4[Int].get > 5)").is_empty());
}

#[test]
fn a_negated_guard_condition_clears_the_else_branch() {
    assert!(findings_of(
        "sigmaProp(if (!(SELF.R4[Int].isDefined)) false else SELF.R4[Int].get > 5)"
    )
    .is_empty());
}

#[test]
fn a_negated_guard_condition_does_not_clear_the_then_branch() {
    assert_eq!(
        findings_of("sigmaProp(if (!(SELF.R4[Int].isDefined)) SELF.R4[Int].get > 5 else false)")
            .len(),
        1
    );
}

#[test]
fn an_or_chain_of_negated_guards_clears_the_tail() {
    assert!(findings_of(
        "sigmaProp(!(SELF.R4[Int].isDefined) || !(SELF.R5[Long].isDefined) || SELF.R4[Int].get > 5 && SELF.R5[Long].get > 1L)"
    )
    .is_empty());
}

#[test]
fn a_positive_guard_under_or_still_proves_nothing_for_the_right_operand() {
    assert_eq!(
        findings_of("sigmaProp(SELF.R4[Int].isDefined || SELF.R4[Int].get > 5)").len(),
        1
    );
}

#[test]
fn a_negated_val_guard_under_or_clears_the_right_operand() {
    assert!(findings_of(
        "{ val ok = SELF.R4[Int].isDefined; sigmaProp(!ok || SELF.R4[Int].get > 5 && ok) }"
    )
    .is_empty());
}
