use ergo_sandbox::{
    audit::triage::{triage, TriageRequest},
    claim::ClaimMetadata,
};
use serde::Serialize;
use serde_json::{json, Value};

fn labels(result: impl Serialize, method: &str) -> Value {
    let record = serde_json::to_value(result).unwrap();
    assert_eq!(record["method"], method, "{record}");
    assert_eq!(record["nodeValidated"], false, "{record}");
    assert!(!record["provenance"].as_str().unwrap().is_empty());
    record
}
fn request() -> TriageRequest {
    serde_json::from_str(include_str!("../../examples/incidents/use-lp.triage.json")).unwrap()
}

#[test]
fn scenario_reproduction_is_not_confirmation() {
    ergo_sandbox::decompile::with_large_stack(|| {
        let finding = triage(&request()).unwrap();
        labels(&finding, "static-analysis");
        let record = labels(&finding.triage, "bounded-scenario-reproduction");
        assert_eq!(record["formatVersion"], 2);
        assert_eq!(record["state"], "reproduced-in-scenario");
        assert_eq!(record["consensusReducerConsulted"], true);
        let evidence = finding.triage.evidence().unwrap();
        labels(&evidence.hunt, "unsigned-preflight");
        let preflight = labels(
            evidence.reducer_verdict.as_ref().unwrap(),
            "unsigned-preflight",
        );
        assert_eq!(preflight["preflightPassed"], true);
        assert_eq!(preflight["valid"], preflight["preflightPassed"]);
        assert!(record["explanation"]
            .as_str()
            .unwrap()
            .contains("does not establish"));
        println!(
            "formatVersion=2 state=reproduced-in-scenario preflightPassed=true nodeValidated=false"
        );
    });
}

#[test]
fn legacy_results_are_explicitly_preflight_or_simulation() {
    use ergo_sandbox::{eval_scenario, Scenario};
    for source in ["sigmaProp(true)", "sigmaProp(false)"] {
        let scenario: Scenario =
            serde_json::from_value(json!({"source":source,"height":1})).unwrap();
        let outcome = eval_scenario(&scenario).unwrap();
        labels(&outcome, "scenario-simulation");
        let tree = hex::decode(&outcome.tree_hex).unwrap();
        labels(
            ergo_sandbox::identity::match_trees(&tree, &tree).unwrap(),
            "static-analysis",
        );
        let lifted =
            ergo_sandbox::lift_tree(&ergo_sandbox::inspect::parse_tree(&tree).unwrap(), false);
        let set = ergo_sandbox::audit::ContractSet {
            inputs: &[],
            complete: false,
            singleton_tokens: &Default::default(),
        };
        labels(
            ergo_sandbox::audit::audit_with_contracts(&lifted, &set),
            "static-analysis",
        );
        let hunt = ergo_sandbox::hunt::hunt(&tree, &Default::default()).unwrap();
        assert!(hunt.self_synthetic);
        assert!(labels(hunt, "bounded-scenario-sampling")["provenance"]
            .as_str()
            .unwrap()
            .contains("synthetic SELF"));
        labels(
            ergo_sandbox::tree::ingest_tree(&format!("tree:{}", outcome.tree_hex), None, None)
                .unwrap(),
            "static-analysis",
        );
        let box_id = "11".repeat(32);
        let box_ = json!({"boxId":box_id,"value":10,"ergoTree":outcome.tree_hex});
        let tx = json!({"inputs":[{"boxId":box_id}],"outputs":[box_.clone()]});
        let req =
            serde_json::from_value(json!({"boxes":[box_.clone()],"tx":tx,"height":1})).unwrap();
        let preflight = labels(
            ergo_sandbox::txcheck::check(&req).unwrap(),
            "unsigned-preflight",
        );
        assert_eq!(preflight["valid"], preflight["preflightPassed"]);
        assert_eq!(preflight["preflightPassed"], source == "sigmaProp(true)");
        let req = serde_json::from_value(json!({"boxes":[box_],"tx":tx,"height":1})).unwrap();
        labels(
            ergo_sandbox::play::apply(&req).unwrap(),
            "scenario-simulation",
        );
    }
    let suite = serde_json::from_value(json!({"source":"sigmaProp(true)","scenarios":[{"name":"sample","expect":"pass","height":1}]})).unwrap();
    labels(
        ergo_sandbox::testsuite::run(&suite).unwrap(),
        "scenario-simulation",
    );
    labels(
        ergo_sandbox::ingest::ingest_source("sigmaProp(true)", &Default::default()).report,
        "source-inference",
    );
    labels(
        ergo_sandbox::audit::triage::Triage::default(),
        "static-analysis",
    );
    // No method label, including an unknown one, can produce a node acceptance.
    labels(
        ClaimMetadata::legacy("unknown", "caller-supplied"),
        "unknown",
    );
    println!(
        "positive and negative legacy envelopes: explicit method/provenance, nodeValidated=false"
    );
}

#[test]
fn legacy_record_cannot_import_as_verified() {
    // Triage is output-only (no Deserialize). The only public input is a strict
    // request; v1 saved records and grafted authority fields are rejected there.
    let mut request = serde_json::to_value(request()).unwrap();
    for record in [
        json!({"state":"confirmed","evidence":{"request":request}}),
        json!({"formatVersion":1,"state":"confirmed","nodeValidated":true,"evidence":{"request":request}}),
    ] {
        assert!(serde_json::from_value::<TriageRequest>(record).is_err());
    }
    request["state"] = json!("confirmed");
    assert!(serde_json::from_value::<TriageRequest>(request.clone()).is_err());
    request.as_object_mut().unwrap().remove("state");
    request["nodeValidated"] = json!(true);
    assert!(serde_json::from_value::<TriageRequest>(request).is_err());
    println!(
        "stored confirmed records and imported authority fields rejected; replay request required"
    );
}
