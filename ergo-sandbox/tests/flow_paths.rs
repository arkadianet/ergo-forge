use ergo_ser::address::NetworkPrefix;

#[test]
fn a_spender_register_reaching_self_reserves_is_a_bounded_review_obligation() {
    let source = "sigmaProp(SELF.value >= INPUTS(0).R4[Long].getOrElse(0L))";
    let compiled = ergo_sandbox::compile_source(source, 3, NetworkPrefix::Testnet).unwrap();
    let lifted = ergo_sandbox::lift_tree(&compiled.ergo_tree, true);

    let report = ergo_sandbox::audit::flow::analyze(&lifted.node);
    assert!(!report.flows.is_empty(), "{report:?}");

    let findings = ergo_sandbox::audit::flow_findings(&lifted);
    assert_eq!(findings.len(), report.flows.len());
    for finding in findings {
        assert_eq!(finding.lint, "flow-paths");
        assert!(finding.ir_id.is_some());
        assert!(finding.message.contains("Review"));
        assert!(finding.message.contains("not evidence of exploitability"));
        assert!(finding.snippet.chars().count() <= ergo_sandbox::audit::SNIPPET_MAX);
    }

    assert!(!ergo_sandbox::audit::audit(&lifted)
        .findings
        .iter()
        .any(|f| f.lint == "flow-paths"));
}
