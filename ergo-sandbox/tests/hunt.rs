//! The spend hunt: bounded scenario sampling for "spendable by anyone?".

use ergo_sandbox::eval::Verdict;
use ergo_sandbox::hunt::{hunt, Hunt, HuntOptions, HuntVerdict, OutputShape, DEFAULT_BASE_HEIGHT};
use ergo_sandbox::{compile_source, ScenarioBox};
use ergo_ser::address::NetworkPrefix;

fn tree(src: &str) -> Vec<u8> {
    compile_source(src, 3, NetworkPrefix::Testnet)
        .expect("compile")
        .tree_bytes
}

fn run(src: &str) -> Hunt {
    hunt(&tree(src), &HuntOptions::default()).expect("hunt")
}

#[test]
fn a_trivially_true_contract_is_spendable_by_anyone() {
    let h = run("sigmaProp(true)");
    assert_eq!(h.verdict, HuntVerdict::SpendableByAnyone, "{h:?}");
    assert!(h.probes.iter().all(|p| p.verdict == Verdict::Pass));
}

#[test]
fn every_hunt_runs_three_heights_times_two_output_shapes() {
    let h = run("sigmaProp(true)");
    assert_eq!(h.probes.len(), 6, "{h:?}");
    let attacker = h
        .probes
        .iter()
        .filter(|p| p.output == OutputShape::Attacker)
        .count();
    assert_eq!(attacker, 3);
    let heights: std::collections::BTreeSet<u32> = h.probes.iter().map(|p| p.height).collect();
    assert_eq!(
        heights,
        [1, DEFAULT_BASE_HEIGHT, DEFAULT_BASE_HEIGHT + 1_000_000]
            .into_iter()
            .collect()
    );
}

#[test]
fn a_future_height_lock_is_found_by_the_plus_one_million_probe() {
    let src = format!("sigmaProp(HEIGHT > {})", DEFAULT_BASE_HEIGHT + 10);
    let h = run(&src);
    assert_eq!(h.verdict, HuntVerdict::SpendableByAnyone, "{h:?}");
    for p in &h.probes {
        let expected = if p.height > DEFAULT_BASE_HEIGHT + 10 {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        assert_eq!(p.verdict, expected, "probe {p:?}");
    }
}

#[test]
fn a_caller_height_moves_the_base_probe() {
    let src = format!("sigmaProp(HEIGHT > {})", DEFAULT_BASE_HEIGHT + 10);
    let opts = HuntOptions {
        height: Some(DEFAULT_BASE_HEIGHT + 20),
        ..HuntOptions::default()
    };
    let h = hunt(&tree(&src), &opts).unwrap();
    let base = h
        .probes
        .iter()
        .find(|p| p.height == DEFAULT_BASE_HEIGHT + 20)
        .expect("base probe at the caller's height");
    assert_eq!(base.verdict, Verdict::Pass);
}

#[test]
fn a_p2pk_contract_requires_proof_and_names_the_key() {
    let h = run("PK(\"3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt\")");
    assert_eq!(h.verdict, HuntVerdict::RequiresProof, "{h:?}");
    assert_eq!(h.residuals.len(), 1, "{:?}", h.residuals);
    assert!(h.residuals[0].contains("ProveDlog"), "{:?}", h.residuals);
}

#[test]
fn a_contract_that_only_needs_self_preserved_is_movable_not_stealable() {
    let h = run("sigmaProp(OUTPUTS(0).propositionBytes == SELF.propositionBytes)");
    assert_eq!(h.verdict, HuntVerdict::MovableByAnyone, "{h:?}");
    for p in &h.probes {
        let expected = match p.output {
            OutputShape::Attacker => Verdict::Fail,
            OutputShape::Preserve => Verdict::Pass,
        };
        assert_eq!(p.verdict, expected, "probe {p:?}");
    }
}

#[test]
fn a_register_read_on_a_synthetic_self_is_not_under_probes_and_says_why() {
    let h = run("sigmaProp(SELF.R4[Int].get > 0)");
    assert_eq!(h.verdict, HuntVerdict::NotUnderProbes, "{h:?}");
    assert!(h.self_synthetic);
    assert!(h.probes.iter().all(|p| p.verdict == Verdict::Error));
    assert!(
        h.probes.iter().all(|p| p.error.is_some()),
        "error text must be kept per probe"
    );
}

#[test]
fn a_supplied_self_box_makes_the_register_read_real() {
    let json = r#"{"value": 1000000, "registers": {"R4": {"type": "Int", "value": 5}}}"#;
    let self_box: ScenarioBox = serde_json::from_str(json).unwrap();
    let opts = HuntOptions {
        self_box: Some(self_box),
        ..HuntOptions::default()
    };
    let h = hunt(&tree("sigmaProp(SELF.R4[Int].get > 0)"), &opts).unwrap();
    assert!(!h.self_synthetic);
    assert_eq!(h.verdict, HuntVerdict::SpendableByAnyone, "{h:?}");
}

#[test]
fn unparseable_bytes_are_a_marshalling_error() {
    assert!(hunt(&[0x10, 0x01], &HuntOptions::default()).is_err());
}
