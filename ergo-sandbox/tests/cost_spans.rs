use ergo_sandbox::{
    compile::compile_with_params_and_map,
    cost_spans::{cost_spans, Rule},
    eval::CostLine,
};
use ergo_ser::address::NetworkPrefix;
fn compile(source: &str) -> (ergo_sandbox::CompileOutput, ergo_compiler::SourceMap) {
    let (tree, map) =
        compile_with_params_and_map(source, &Default::default(), 3, NetworkPrefix::Mainnet)
            .unwrap();
    (tree, map.unwrap())
}
#[test]
fn mismatch_missing_maps_and_unlabelled_gaps_never_gain_attribution() {
    let source = "sigmaProp(HEIGHT > 100)";
    let (tree, map) = compile(source);
    let (other, _) = compile("sigmaProp(SELF.value > 100L)");
    let lines = [
        CostLine {
            label: "OP:0xA3".into(),
            delta: 26,
            total: 31,
        },
        CostLine {
            label: "Crypto:100".into(),
            delta: 100,
            total: 131,
        },
    ];
    for (root, map) in [
        (&tree.ergo_tree.body, None),
        (&other.ergo_tree.body, Some(&map)),
    ] {
        let r = cost_spans(source, root, map, "no-map", &lines);
        assert_eq!(r.total_jit, 131);
        assert_eq!(r.unattributed_jit, 131);
        assert_eq!(r.attributed_jit, 0);
        assert!(r
            .rows
            .iter()
            .all(|r| r.rule == Rule::Unattributed && r.candidates.is_empty()));
        if map.is_some() {
            assert_eq!(r.map_status, "misaligned");
        }
    }
}
#[test]
fn repeated_opcodes_remain_ambiguous_even_with_detail_or_uncited_candidates() {
    let source="sigmaProp(OUTPUTS.exists { (o: Box) => o.value > 1L } && INPUTS.exists { (i: Box) => i.value > 2L })";
    let (tree, map) = compile(source);
    let lines = [CostLine {
        label: "OP:0xAE:n=8".into(),
        delta: 20,
        total: 20,
    }];
    let r = cost_spans(source, &tree.ergo_tree.body, Some(&map), "aligned", &lines);
    assert_eq!(r.rows[0].rule, Rule::Ambiguous);
    assert_eq!(r.rows[0].candidates.len(), 2);
    assert_eq!(r.ambiguous_jit, 20);
    assert_eq!(r.exact_share, 0.0);
}
#[cfg(feature = "cost-trace")]
#[test]
fn substitutions_errors_proofs_and_dynamic_trees_keep_the_boundary() {
    for payload in [
        serde_json::json!({"source":"{ val bytes = fromBase16(\"$data\"); sigmaProp(HEIGHT > bytes.size) }","height":100,"params":{"data":{"type":"String","value":"abcdef"}}}),
        serde_json::json!({"source":"sigmaProp(SELF.R4[Int].get > HEIGHT)","height":100}),
        serde_json::json!({"source":"sigmaProp(HEIGHT > 1)","height":100,"proof":""}),
        serde_json::json!({"source":"{ sigmaProp(HEIGHT > 1) && executeFromVar[SigmaProp](0) }","height":100}),
    ] {
        let sc: ergo_sandbox::Scenario = serde_json::from_value(payload).unwrap();
        let out = ergo_sandbox::eval_scenario(&sc).unwrap();
        let p = ergo_sandbox::source_positions::SourcePositions::for_run(&sc, &out).unwrap();
        let r = cost_spans(
            sc.source.as_ref().unwrap(),
            &p.tree.body,
            p.map.as_ref(),
            p.status,
            &out.cost_breakdown,
        );
        assert_eq!(
            r.total_jit,
            r.attributed_jit + r.ambiguous_jit + r.unattributed_jit
        );
        assert_eq!(
            r.total_jit,
            out.cost_breakdown.last().map_or(0, |l| l.total)
        );
        if !sc.params.is_empty() {
            assert_eq!(p.status, "substituted-source");
            assert_eq!(r.attributed_jit, 0);
        }
        if sc.source.as_ref().unwrap().contains("executeFromVar") {
            assert_eq!(r.attributed_jit, 0);
        }
    }
}
