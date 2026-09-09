//! Delegation is a review obligation, including on an identity-bound vault.

use std::collections::BTreeMap;

use ergo_sandbox::audit::{self, Completeness};
use ergo_sandbox::{compile_source, lift_tree, Finding, Lifted, Severity, TypedValue};
use ergo_ser::address::NetworkPrefix;

fn findings(lifted: &Lifted) -> Vec<Finding> {
    let audit = audit::audit(lifted);
    assert_eq!(
        audit::lints::delegated_reserves(&lifted.node).len(),
        audit
            .findings
            .iter()
            .filter(|f| f.lint == "delegated-reserves")
            .count(),
        "lint must be registered"
    );
    assert_eq!(audit.completeness, Completeness::Complete);
    audit
        .findings
        .into_iter()
        .filter(|f| f.lint == "delegated-reserves")
        .collect()
}

fn source(src: &str) -> Vec<Finding> {
    let compiled = compile_source(src, 3, NetworkPrefix::Testnet).expect("compile");
    findings(&lift_tree(&compiled.ergo_tree, false))
}

fn preserved(extra: &str) -> Vec<Finding> {
    source(&format!(
        "{{ val out = OUTPUTS(0); sigmaProp(out.propositionBytes == SELF.propositionBytes && ({extra})) }}"
    ))
}

#[test]
fn deployed_use_vault_delegates_erg_and_treasury_amount() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../examples/incidents/use-bank-vault.json")).unwrap();
    assert!(fixture["lint"]["note"].as_str().unwrap().contains("AMOUNT"));
    let bytes = hex::decode(fixture["vault"]["ergoTree"].as_str().unwrap()).unwrap();
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).unwrap();
    // Both printer modes must follow the lift's hoisted aliases.
    for inline in [false, true] {
        let lifted = lift_tree(&tree, inline);
        let audit = audit::audit(&lifted);
        assert!(!audit
            .findings
            .iter()
            .any(|f| f.lint == "unbound-box-reserves"));
        let f = findings(&lifted);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].severity, Severity::Medium);
        assert_eq!(f[0].snippet, "OUTPUTS(1)");
        assert!(f[0].message.contains("ERG value"), "{f:?}");
        assert!(f[0].message.contains("tokens(1)._2"), "{f:?}");
        assert!(!f[0].message.contains("tokens(0)._2"), "{f:?}");
        assert!(f[0].message.contains("delegated"), "{f:?}");
        assert!(f[0].message.contains("not evidence of exploitability"));
        assert!(
            f[0].ir_id.is_some(),
            "finding must map to the identity equality"
        );
    }
}

#[test]
fn identity_alone_is_enough_and_only_outputs_are_successors() {
    for identity in [
        "OUTPUTS(2).propositionBytes == SELF.propositionBytes",
        "SELF.propositionBytes == OUTPUTS(2).propositionBytes",
        "OUTPUTS(2).tokens(3) == SELF.tokens(3)",
        "SELF.tokens(3)._1 == OUTPUTS(2).tokens(3)._1",
        "OUTPUTS(2).tokens == SELF.tokens",
    ] {
        let f = source(&format!("sigmaProp({identity})"));
        assert_eq!(f.len(), 1, "{identity}: {f:?}");
        assert_eq!(f[0].snippet, "OUTPUTS(2)");
    }
    for identity in [
        "INPUTS(0).propositionBytes == SELF.propositionBytes",
        "CONTEXT.dataInputs(0).tokens(0) == SELF.tokens(0)",
        "OUTPUTS(0).tokens(1)._1 == SELF.tokens(2)._1",
        "OUTPUTS(0).tokens(1)._2 == SELF.tokens(1)._2",
        "OUTPUTS(0).propositionBytes == INPUTS(0).propositionBytes",
    ] {
        assert!(
            source(&format!("sigmaProp({identity})")).is_empty(),
            "{identity}"
        );
    }
}

#[test]
fn direct_and_arithmetic_value_bounds_clear_erg_delegation() {
    for bound in [
        "out.value >= SELF.value",
        "out.value == SELF.value",
        "SELF.value == out.value",
        "SELF.value <= out.value",
        "out.value > SELF.value",
        "SELF.value < out.value",
        "out.value >= SELF.value - 1000000L",
        "out.value - SELF.value >= -1000000L",
        "SELF.value - out.value <= 1000000L",
        "out.value.toBigInt * out.tokens(1)._2.toBigInt >= SELF.value.toBigInt * SELF.tokens(1)._2.toBigInt",
    ] {
        assert!(preserved(bound).is_empty(), "{bound}");
    }
}

#[test]
fn dust_floors_upper_bounds_and_unrelated_reads_do_not_clear_erg_delegation() {
    for bound in [
        "out.value >= 1000000L",
        "out.value <= SELF.value",
        "SELF.value >= out.value",
        "out.value.toBigInt < SELF.value.toBigInt",
        "out.value != SELF.value",
        "out.value >= INPUTS(0).value && SELF.value > 0L",
        "OUTPUTS(1).value >= SELF.value",
    ] {
        let f = preserved(bound);
        assert_eq!(f.len(), 1, "{bound}: {f:?}");
        assert!(f[0].message.contains("ERG value"));
    }
}

#[test]
fn token_delegation_is_independent_of_erg_and_specific_to_the_slot() {
    let id = "out.value >= SELF.value && out.tokens(1)._1 == SELF.tokens(1)._1";
    for companion in [
        "HEIGHT > 0",
        "out.tokens(2)._2 >= SELF.tokens(2)._2",
        "OUTPUTS(1).tokens(1)._2 >= SELF.tokens(1)._2",
    ] {
        let f = preserved(&format!("{id} && {companion}"));
        assert_eq!(f.len(), 1, "{companion}: {f:?}");
        assert!(!f[0].message.contains("ERG value"));
        assert!(f[0].message.contains("tokens(1)._2"));
    }
    for companion in [
        "out.tokens(1)._2 >= SELF.tokens(1)._2",
        "out.tokens(1)._2 == 100L",
        "SELF.tokens(1)._2 - out.tokens(1)._2 <= SELF.R4[Long].get",
        "out.tokens(1) == SELF.tokens(1)",
        "SELF.tokens == out.tokens",
    ] {
        assert!(
            preserved(&format!("{id} && {companion}")).is_empty(),
            "{companion}"
        );
    }
}

#[test]
fn findings_are_deduplicated_per_successor_and_keep_other_successors_separate() {
    let f = source(
        "sigmaProp(OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
         OUTPUTS(0).tokens(0) == SELF.tokens(0) &&
         OUTPUTS(1).propositionBytes == SELF.propositionBytes &&
         OUTPUTS(1).value >= SELF.value &&
         OUTPUTS(2).tokens(0) == SELF.tokens(0))",
    );
    assert_eq!(f.len(), 2, "{f:?}");
    assert_eq!(f[0].snippet, "OUTPUTS(0)");
    assert_eq!(f[1].snippet, "OUTPUTS(2)");
    assert_ne!(f[0].node_id, f[1].node_id);
}

#[test]
fn gallery_amm_pool_has_local_reserve_constraints() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../examples/tests/amm-pool.test.json")).unwrap();
    let params: BTreeMap<String, TypedValue> =
        serde_json::from_value(fixture["params"].clone()).unwrap();
    let compiled = ergo_sandbox::compile::compile_with_params(
        include_str!("../../examples/contracts/protocols/amm/pool.es"),
        &params,
        3,
        NetworkPrefix::Testnet,
    )
    .expect("compile gallery AMM");
    let f = findings(&lift_tree(&compiled.ergo_tree, false));
    assert!(
        f.is_empty(),
        "constant-product, deposit and redeem bounds: {f:?}"
    );
}

fn dexy(src: &str) -> Vec<Finding> {
    // These corpus sources use string parameters for base64 NFT ids. Distinct
    // 32-byte values suffice for a structural audit; no transaction is run.
    let params = ergo_sandbox::compile::scan_params(src)
        .into_iter()
        .enumerate()
        .map(|(i, need)| {
            if need.name == "initialDexyTokens" {
                return (
                    need.name,
                    TypedValue {
                        r#type: "Long".into(),
                        value: serde_json::json!(1_000_000_000_000_000_000i64),
                    },
                );
            }
            let prefix = ["AA", "AQ", "Ag", "Aw", "BA", "BQ", "Bg", "Bw"][i];
            (
                need.name,
                TypedValue {
                    r#type: "String".into(),
                    value: serde_json::json!(format!("{}{prefix}=", "A".repeat(41))),
                },
            )
        })
        .collect();
    let compiled =
        ergo_sandbox::compile::compile_with_params(src, &params, 3, NetworkPrefix::Testnet)
            .expect("compile Dexy corpus source");
    findings(&lift_tree(&compiled.ergo_tree, false))
}

#[test]
fn dexy_mint_and_payout_successors_preserve_their_own_reserves() {
    for src in [
        include_str!("../../examples/contracts/dexy/bank/freemint.es"),
        include_str!("../../examples/contracts/dexy/bank/arbmint.es"),
        include_str!("../../examples/contracts/dexy/bank/payout.es"),
    ] {
        let f = dexy(src);
        assert!(f.is_empty(), "{f:?}");
    }
}

#[test]
fn dexy_bank_deliberately_delegates_reserves() {
    let f = dexy(include_str!("../../examples/contracts/dexy/bank/bank.es"));
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].message.contains("ERG value"));
    assert!(f[0].message.contains("tokens(1)._2"));
}

/// Recorded limit: a bound in one spending branch suppresses this whole-tree
/// absence lint. This test records its scope, not a reserve-safety verdict.
#[test]
fn branch_specific_delegation_is_not_decided() {
    assert!(preserved("out.value >= SELF.value || HEIGHT > 100").is_empty());
}
