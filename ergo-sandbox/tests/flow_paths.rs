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

#[test]
fn repeated_val_dependencies_finish_quickly_and_keep_taint() {
    let mut source = String::from("{ val x0 = getVar[Long](0).get; ");
    for i in 1..=40 {
        source.push_str(&format!("val x{i} = x{} + x{}; ", i - 1, i - 1));
    }
    source.push_str("sigmaProp(SELF.value >= x40) }");
    let compiled = ergo_sandbox::compile_source(&source, 3, NetworkPrefix::Testnet).unwrap();
    let lifted = ergo_sandbox::lift_tree(&compiled.ergo_tree, true);
    assert!(compiled.tree_bytes.len() < 1024);
    let start = std::time::Instant::now();
    let report = ergo_sandbox::audit::flow::analyze_with(
        &lifted.node,
        ergo_sandbox::audit::flow::Bounds {
            max_depth: 128,
            ..Default::default()
        },
    );
    assert!(
        report.flows.iter().any(|f| f.source.contains("getVar[0]")),
        "{report:?}"
    );
    assert!(!ergo_sandbox::audit::flow_findings(&lifted).is_empty());
    let checklist = ergo_sandbox::checklist::checklist(
        &compiled.tree_bytes,
        NetworkPrefix::Testnet,
        &Default::default(),
    )
    .unwrap();
    assert!(!checklist.flow.flows.is_empty());
    assert!(checklist
        .flow
        .flows
        .iter()
        .any(|f| f.source.contains("getVar[0]")));
    assert!(start.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn a_trace_alternatives_cut_is_reported_even_with_repeated_source_names() {
    use ergo_sandbox::{Node, NodeKind};
    let mut id = 0;
    let mut node = |kind| {
        id += 1;
        Node { id, kind }
    };
    let mut sum = node(NodeKind::GetVar(0, "Long".into()));
    for var in [0, 0, 0, 1] {
        let rhs = node(NodeKind::GetVar(var, "Long".into()));
        sum = node(NodeKind::Infix("+", Box::new(sum), Box::new(rhs)));
    }
    let anchor = node(NodeKind::Leaf("SELF"));
    let value = node(NodeKind::Prop(Box::new(anchor), "value".into()));
    let root = node(NodeKind::Infix(">=", Box::new(value), Box::new(sum)));
    let report = ergo_sandbox::audit::flow::analyze(&root);
    assert!(report.limits.alternatives, "{report:?}");
    assert!(ergo_sandbox::audit::lints::flow_paths(&root)
        .iter()
        .any(|f| f.message.contains("Bounded")));
}
