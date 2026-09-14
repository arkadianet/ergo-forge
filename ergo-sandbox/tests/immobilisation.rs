use ergo_sandbox::{
    compile_source,
    eval::Verdict,
    hunt::{hunt, HuntOptions, HuntVerdict, ProbeKind},
    rent, ScenarioBox,
};
use ergo_ser::address::NetworkPrefix;

fn run(source: &str, options: HuntOptions) -> ergo_sandbox::hunt::Hunt {
    let c = compile_source(source, 3, NetworkPrefix::Mainnet).unwrap();
    hunt(&c.tree_bytes, &options).unwrap()
}

#[test]
fn immobilisation_probes_report_unspendable_not_safe() {
    for source in [
        "sigmaProp(SELF.R4[Int].get > 0)",
        "sigmaProp(INPUTS(2).R4[Int].get > 0)",
        "sigmaProp(CONTEXT.dataInputs(1).R4[Int].get > 0)",
    ] {
        let h = run(source, HuntOptions::default());
        assert_eq!(h.verdict, HuntVerdict::NotUnderProbes);
        assert!(h.observation.contains("not spendable under these probes"));
        assert!(h.probes.iter().all(|p| p.verdict == Verdict::Error));
        let absent: Vec<_> = h
            .probes
            .iter()
            .filter(|p| p.kind == ProbeKind::RegistersAbsent)
            .collect();
        assert!(absent
            .iter()
            .all(|p| p.observation == "unspendable-under-probe"));
        assert!(
            absent.iter().all(|p| !p.erroring_reads.is_empty()),
            "{absent:?}"
        );
        let json = serde_json::to_string(&h).unwrap();
        assert!(!json.to_lowercase().contains("safe"), "{json}");
        assert_eq!(serde_json::to_value(&h).unwrap()["nodeValidated"], false);
        assert_eq!(h.probes.len(), h.caps.max_probes);
        assert!(!h.truncated);
    }
    let control = run(
        "sigmaProp(SELF.R4[Int].getOrElse(1) > 0)",
        HuntOptions::default(),
    );
    assert!(control.probes.iter().all(|p| p.verdict == Verdict::Pass));
    let supplied = serde_json::from_str::<ScenarioBox>(
        r#"{"value":1000000,"registers":{"R4":{"type":"Int","value":1}}}"#,
    )
    .unwrap();
    let h = run(
        "sigmaProp(SELF.R4[Int].get > 0)",
        HuntOptions {
            self_box: Some(supplied),
            ..Default::default()
        },
    );
    assert!(h
        .probes
        .iter()
        .filter(|p| p.kind == ProbeKind::RegistersAbsent)
        .all(|p| p.verdict == Verdict::Error));
    assert!(h
        .probes
        .iter()
        .filter(|p| p.kind == ProbeKind::CostLimit)
        .all(|p| p.verdict == Verdict::Pass && !p.cost_exhausted));
}

#[test]
fn minimum_outputs_and_cost_budget_are_recorded() {
    let h = run(
        "sigmaProp(OUTPUTS(2).value > 0L && OUTPUTS(0).value < 1000000L)",
        HuntOptions::default(),
    );
    let min = h
        .probes
        .iter()
        .filter(|p| p.kind == ProbeKind::MinimumOutputValue);
    for p in min {
        assert_eq!(p.verdict, Verdict::Pass);
        assert_eq!(p.output_values.len(), 3);
        assert!(p.output_values.iter().all(|v| *v >= 360 && *v < 1000000));
        assert_eq!(p.cost_limit, ergo_sandbox::eval::DEFAULT_COST_LIMIT);
    }
    assert_eq!(
        h.caps.block_cost_limit,
        ergo_validation::ProtocolParams::mainnet_default().max_block_cost
    );
    assert!(h.caps.basis.contains("check_output_box"));
    assert!(run("sigmaProp(OUTPUTS(999).value > 0L)", HuntOptions::default()).truncated);
}

#[test]
fn storage_rent_line_is_static() {
    let tree = compile_source("sigmaProp(false)", 3, NetworkPrefix::Mainnet)
        .unwrap()
        .tree_bytes;
    let b: ScenarioBox = serde_json::from_str(
        r#"{"value":1000000,"creationHeight":100,"registers":{"R4":{"type":"Int","value":1}}}"#,
    )
    .unwrap();
    for context in [None, Some(&b)] {
        let line = rent::read_line(rent::estimate_for(&tree, context));
        let v = serde_json::to_value(line).unwrap();
        assert_eq!(v["provenance"], "static");
        assert_eq!(v["nodeValidated"], false);
        assert!(v["text"]
            .as_str()
            .unwrap()
            .contains("anyone may claim this box for the rent"));
        assert!(v["basis"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b == "P03:storage-rent-acceptance"));
        assert_eq!(v["estimate"]["periodBlocks"], rent::STORAGE_PERIOD);
        assert_eq!(v["estimate"]["feeFactor"], rent::STORAGE_FEE_FACTOR);
        assert_eq!(
            v["estimate"]["nextCollectionHeight"],
            serde_json::json!(context.map(|_| 100 + rent::STORAGE_PERIOD))
        );
    }
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/evidence/manifest.json")).unwrap();
    let basis = manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "storage-rent-acceptance")
        .unwrap();
    assert_eq!(basis["expected"]["status"], "node-accepted");
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/evidence/node-vectors/storage-rent-acceptance.fixture"
    ))
    .unwrap();
    assert_eq!(
        fixture["parameters"]["value"]["storagePeriod"],
        rent::STORAGE_PERIOD
    );
    assert_eq!(
        fixture["parameters"]["value"]["storageFeeFactor"],
        rent::STORAGE_FEE_FACTOR
    );
}

#[test]
fn minimum_value_matches_node_structural_rule_and_rejects_one_less() {
    use ergo_primitives::digest::Digest32;
    use ergo_ser::{
        input::{ContextExtension, Input, SpendingProof},
        transaction::Transaction,
    };
    let source = "sigmaProp(OUTPUTS(2).value > 0L)";
    let h = run(source, HuntOptions::default());
    let p = h
        .probes
        .iter()
        .find(|p| {
            p.kind == ProbeKind::MinimumOutputValue
                && p.output == ergo_sandbox::hunt::OutputShape::Attacker
        })
        .unwrap();
    let tree = hex::encode(
        compile_source("sigmaProp(true)", 3, NetworkPrefix::Mainnet)
            .unwrap()
            .tree_bytes,
    );
    let mut tx = Transaction {
        inputs: vec![Input {
            box_id: Digest32::from_bytes([1; 32]),
            spending_proof: SpendingProof::new(vec![], ContextExtension::empty()).unwrap(),
        }],
        data_inputs: vec![],
        output_candidates: p
            .output_values
            .iter()
            .map(|v| {
                ergo_sandbox::evidence::wire::CandidateSpec {
                    value: *v as u64,
                    ergo_tree: tree.clone(),
                    creation_height: p.height,
                    tokens: vec![],
                    registers: "00".into(),
                }
                .build()
                .unwrap()
            })
            .collect(),
    };
    let params = ergo_validation::ProtocolParams::mainnet_default();
    ergo_validation::tx::structural::validate_structural(&tx, &params).unwrap();
    tx.output_candidates[2].value -= 1;
    assert!(matches!(
        ergo_validation::tx::structural::validate_structural(&tx, &params),
        Err(ergo_validation::ValidationError::OutputValueTooLow { index: 2, .. })
    ));
}

#[test]
fn cost_probe_reports_engine_exhaustion_at_the_existing_block_budget() {
    let source = "{ val xs = SELF.R4[Coll[Int]].get; sigmaProp(xs.fold(0L, { (a: Long, b: Int) => { val subtotal = xs.fold(0L, { (c: Long, d: Int) => c + d.toLong }); a + subtotal + b.toLong } }) > 0L) }";
    let b: ScenarioBox = serde_json::from_value(serde_json::json!({"value":1000000,"registers":{"R4":{"type":"Coll[Int]","value":vec![1; 4000]}}})).unwrap();
    let h = run(
        source,
        HuntOptions {
            self_box: Some(b),
            ..Default::default()
        },
    );
    let probes: Vec<_> = h
        .probes
        .iter()
        .filter(|p| p.kind == ProbeKind::CostLimit)
        .collect();
    assert_eq!(probes.len(), 2);
    assert!(
        probes.iter().all(|p| p.cost_exhausted
            && p.verdict == Verdict::Error
            && p.cost_limit == ergo_sandbox::eval::DEFAULT_COST_LIMIT),
        "{probes:?}"
    );
}
