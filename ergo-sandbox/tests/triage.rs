use ergo_sandbox::audit::triage::{triage, TriageRequest, BOUNDED_WARNING};
use serde_json::Value;

const DEPLOYED: &str = include_str!("../../examples/incidents/use-lp.triage.json");
const FIXED: &str = include_str!("../../examples/incidents/fixed/use-lp.triage.json");

fn run(req: TriageRequest) -> ergo_sandbox::Finding {
    ergo_sandbox::decompile::with_large_stack(move || triage(&req)).unwrap()
}

#[test]
fn incident_pair_records_scenario_reproduction_and_bounded_miss() {
    let deployed: TriageRequest = serde_json::from_str(DEPLOYED).unwrap();
    let fixed: TriageRequest = serde_json::from_str(FIXED).unwrap();
    assert_eq!(deployed.lint, fixed.lint);
    assert_eq!(deployed.node_id, fixed.node_id);
    // The same pool finding, with only its companion's implementation changed.
    let mut before: Value = serde_json::from_str(DEPLOYED).unwrap();
    let after: Value = serde_json::from_str(FIXED).unwrap();
    before["drain"]["inputs"][1]["ergoTree"] = after["drain"]["inputs"][1]["ergoTree"].clone();
    assert_eq!(before, after);

    for (req, state) in [
        (deployed, "reproduced-in-scenario"),
        (fixed, "not-reproduced"),
    ] {
        let finding = run(req);
        assert_eq!(finding.triage.state(), state);
        let evidence = finding.triage.evidence().unwrap();
        assert!(evidence.hunt.oracle_calls > 0);
        assert!(evidence.hunt.probes_run <= evidence.hunt.synthesis.caps.max_probes);
        assert_eq!(
            evidence
                .hunt
                .synthesis
                .shapes
                .iter()
                .map(|s| s.run)
                .sum::<usize>(),
            evidence.hunt.probes_run
        );
        let record = serde_json::to_value(&finding).unwrap();
        assert_eq!(record["triage"]["consensusReducerConsulted"], true);
        // Evidence retains a directly replayable input, including the finding anchor.
        let replay: TriageRequest =
            serde_json::from_value(record["triage"]["evidence"]["request"].clone()).unwrap();
        assert_eq!(replay.node_id, finding.node_id);
        if state == "reproduced-in-scenario" {
            let hit = evidence.hunt.best.as_ref().unwrap();
            assert!(!hit.shape.is_empty());
            assert!(evidence.reducer_verdict.as_ref().unwrap().valid);
            let replayed = ergo_sandbox::txcheck::check(&hit.witness.tx_request).unwrap();
            assert!(replayed.valid);
            assert!(!hit.extracted.is_empty());
        } else {
            assert!(evidence.hunt.best.is_none());
            assert!(evidence.reducer_verdict.is_none());
            assert_eq!(record["triage"]["explanation"], BOUNDED_WARNING);
        }
        println!(
            "{state}: probes={}, oracle_calls={}, hits={}, capped={}, shapes={}",
            evidence.hunt.probes_run,
            evidence.hunt.oracle_calls,
            evidence.hunt.hits,
            evidence.hunt.capped,
            serde_json::to_string(&evidence.hunt.synthesis.shapes).unwrap()
        );
    }
}

#[test]
fn static_and_unusable_hunts_cannot_claim_confirmation() {
    let req: TriageRequest = serde_json::from_str(DEPLOYED).unwrap();
    let bytes = hex::decode(req.drain.inputs[0].box_.ergo_tree.as_ref().unwrap()).unwrap();
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).unwrap();
    for finding in ergo_sandbox::audit::audit(&ergo_sandbox::lift_tree(&tree, false)).findings {
        let record = serde_json::to_value(finding).unwrap();
        assert_eq!(record["triage"]["state"], "unconfirmed");
        assert_eq!(record["triage"]["evidence"]["kind"], "static-only");
        assert_eq!(record["triage"]["consensusReducerConsulted"], false);
        assert!(record["triage"]["explanation"]
            .as_str()
            .unwrap()
            .contains("Static"));
    }
    let mut missing = req.clone();
    missing.drain.objective = None;
    missing.drain.max_probes = Some(1);
    let finding = run(missing);
    assert_eq!(finding.triage.state(), "unconfirmed");
    assert!(finding.triage.evidence().is_some());

    let mut invalid = req.clone();
    invalid.drain.protocol_nfts.clear();
    let finding = run(invalid);
    assert_eq!(finding.triage.state(), "unconfirmed");
    assert_eq!(finding.triage.evidence().unwrap().hunt.oracle_calls, 0);

    let mut stale = req;
    stale.node_id = u64::MAX;
    assert!(triage(&stale).is_err());
}

#[test]
fn fixed_request_uses_the_committed_fixed_source() {
    let suite: Value = serde_json::from_str(include_str!(
        "../../examples/incidents/use-lp-drain.fixed-swap.test.json"
    ))
    .unwrap();
    let compiled = ergo_sandbox::compile::compile_with_params(
        include_str!("../../examples/incidents/fixed/use-lp-swap.es"),
        &serde_json::from_value(suite["params"].clone()).unwrap(),
        3,
        ergo_ser::address::NetworkPrefix::Mainnet,
    )
    .unwrap();
    let req: TriageRequest = serde_json::from_str(FIXED).unwrap();
    assert_eq!(
        req.drain.inputs[1].box_.ergo_tree.as_deref(),
        Some(hex::encode(compiled.tree_bytes).as_str())
    );
}

#[test]
fn cli_emits_a_recorded_finding() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args([
            "triage",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../examples/incidents/fixed/use-lp.triage.json"
            ),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(record["triage"]["state"], "not-reproduced");
    assert_eq!(record["triage"]["consensusReducerConsulted"], true);
    assert!(
        record["triage"]["evidence"]["hunt"]["probesRun"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn a_truncated_miss_keeps_its_budget_and_warning() {
    let mut req: TriageRequest = serde_json::from_str(DEPLOYED).unwrap();
    req.drain.max_probes = Some(1);
    let finding = run(req);
    assert_eq!(finding.triage.state(), "not-reproduced");
    let evidence = finding.triage.evidence().unwrap();
    assert_eq!(evidence.hunt.probes_run, 1);
    assert_eq!(evidence.hunt.oracle_calls, 1);
    assert!(evidence.hunt.capped);
    assert_eq!(evidence.hunt.synthesis.caps.max_probes, 1);
    assert_eq!(evidence.request.drain.max_probes, Some(1));
    assert_eq!(
        serde_json::to_value(finding).unwrap()["triage"]["explanation"],
        BOUNDED_WARNING
    );
}
