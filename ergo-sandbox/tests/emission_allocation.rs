//! Rule 123's emission-box allocation check, reached through the `emission` premise of the
//! enabled network rules. The vectors are separate from the frozen P03 inventory
//! (`manifest.json`, whose case IDs the roadmap policy names): they have their own manifest,
//! authenticated here before any request runs. Their expected results come from the node's
//! `validate_transaction` called directly (`generate_emission_vectors.rs`).
// The revision migration the P03 tests use, without the rest of `engine_support`.
#[path = "engine_support/request.rs"]
mod migrate;
use ergo_sandbox::evidence::{
    emission::EmissionConstants,
    validate::{validate, ValidationRequest},
};
use ergo_validation::tx::reemission::EmissionRuleInputs;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

const MANIFEST: &str = "emission-manifest.json";

/// Every vector, authenticated against the manifest: hash, revision and import round trip.
fn cases() -> Vec<(Value, ValidationRequest)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evidence");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(manifest["formatVersion"], 1);
    let recorded = manifest["nodeRevision"].as_str().unwrap().to_owned();
    let mut loaded = vec![];
    for row in manifest["cases"].as_array().unwrap() {
        assert_eq!(row["nodeRevision"], manifest["nodeRevision"]);
        assert_eq!(row["claimStatus"], "no-property-claim");
        assert_eq!(row["publicationEligibility"], "public-authored");
        assert_eq!(row["sourceKind"], "authored-full-node-vector");
        let files = row["files"].as_array().unwrap();
        assert_eq!(files.len(), 1, "one self-contained request per vector");
        let bytes = std::fs::read(root.join(files[0]["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            files[0]["sha256"],
            "{} hash changed",
            row["id"]
        );
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let request: ValidationRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            value,
            "no premise disappears during import"
        );
        loaded.push((
            row.clone(),
            migrate::on_current_engine_from(request, &recorded),
        ));
    }
    loaded
}

fn case(id: &str) -> (Value, ValidationRequest) {
    cases().into_iter().find(|(r, _)| r["id"] == id).unwrap()
}

#[test]
fn the_node_checks_the_emission_allocation_when_given_the_constants() {
    let all = cases();
    assert_eq!(all.len(), 2);
    for (row, request) in all {
        let expected = &row["expected"];
        match validate(&request) {
            Ok(accepted) => {
                let report = accepted.report();
                assert_eq!(row["id"], "emission-allocation-acceptance");
                assert_eq!(report["status"], expected["status"]);
                assert_eq!(report["stage"], expected["stage"]);
                assert_eq!(report["transactionId"], expected["transactionId"]);
                assert_eq!(
                    report["totalBlockCost"].as_u64(),
                    expected["totalBlockCost"].as_u64()
                );
                assert_eq!(report["request"], serde_json::to_value(&request).unwrap());
                assert_eq!(report["requestFingerprint"], request.fingerprint());
            }
            Err(failure) => {
                assert_eq!(row["id"], "emission-allocation-rejection");
                assert_eq!(failure.status, expected["status"].as_str().unwrap());
                assert_eq!(failure.stage, expected["stage"].as_str().unwrap());
                assert!(failure.pipeline_invoked);
                assert!(!failure.node_validated);
                assert!(
                    failure
                        .detail
                        .starts_with(&format!("{}: ", expected["detail"].as_str().unwrap())),
                    "node diagnostic differs from the full-pipeline result: {}",
                    failure.detail
                );
                assert!(failure
                    .detail
                    .contains("emission token allocation is invalid"));
            }
        }
    }
}

/// The gap the premise closes, kept visible: the rejected transaction is accepted when the
/// request carries no `emission`, so such a result does not cover the allocation rule. The
/// request without the field is the request as it was before the premise existed.
#[test]
fn without_the_constants_the_allocation_is_not_checked() {
    let (_, rejected) = case("emission-allocation-rejection");
    assert!(validate(&rejected).is_err());
    let mut value = serde_json::to_value(&rejected).unwrap();
    let rules = value["networkRules"]["value"].as_object_mut().unwrap();
    assert!(rules.remove("emission").is_some());
    let without: ValidationRequest = serde_json::from_value(value.clone()).unwrap();
    let accepted = validate(&without).expect("allocation is unchecked without the constants");
    assert_eq!(accepted.report()["status"], "node-accepted");
    // An absent premise is not serialized, so a request that never had it keeps its fingerprint.
    assert_eq!(serde_json::to_value(&without).unwrap(), value);
    assert!(
        serde_json::to_value(&without).unwrap()["networkRules"]["value"]
            .get("emission")
            .is_none()
    );
    assert_ne!(without.fingerprint(), rejected.fingerprint());
}

#[test]
fn the_constants_are_validated_before_the_node_runs() {
    let (_, base) = case("emission-allocation-acceptance");
    assert!(validate(&base).is_ok());
    let original = serde_json::to_value(&base).unwrap();
    let mutate = |f: &dyn Fn(&mut Value)| {
        let mut value = original.clone();
        f(&mut value["networkRules"]["value"]["emission"]);
        value
    };
    // The node divides by the epoch length.
    for (name, value) in [
        (
            "zero epoch length",
            mutate(&|e| e["epochLength"] = json!(0)),
        ),
        (
            "short NFT id",
            mutate(&|e| e["emissionNftId"] = json!("22".repeat(31))),
        ),
        (
            "non-hex NFT id",
            mutate(&|e| e["emissionNftId"] = json!("zz".repeat(32))),
        ),
        (
            "trailing tree bytes",
            mutate(&|e| e["emissionTree"] = json!("10010101d1730000")),
        ),
        (
            "unparsable tree",
            mutate(&|e| e["emissionTree"] = json!("ff")),
        ),
        (
            "non-SigmaProp sizeless tree",
            mutate(&|e| e["emissionTree"] = json!("100101d17300")),
        ),
    ] {
        let request: ValidationRequest = serde_json::from_value(value).unwrap();
        let failure = validate(&request).expect_err(name);
        assert_eq!(failure.status, "incomplete-or-invalid-premises", "{name}");
        assert!(!failure.pipeline_invoked, "{name}");
        assert!(!failure.node_validated, "{name}");
    }
    // Unknown and missing constants are import errors, never defaults.
    let unknown = mutate(&|e| e["note"] = json!("x"));
    assert!(serde_json::from_value::<ValidationRequest>(unknown).is_err());
    let missing = mutate(&|e| {
        e.as_object_mut().unwrap().remove("fixedRate");
    });
    assert!(serde_json::from_value::<ValidationRequest>(missing).is_err());
    // `emission` belongs to enabled rules only.
    let mut disabled = original.clone();
    disabled["networkRules"]["value"] = json!({"mode":"disabled","description":"d","reason":"r",
        "emission": original["networkRules"]["value"]["emission"]});
    assert!(serde_json::from_value::<ValidationRequest>(disabled).is_err());
}

/// Each field reaches its place in the node's inputs: the node's own mainnet constants, written
/// as the premise, come back as the node's own `EmissionRuleInputs`, so the real emission tree
/// also passes the gates the premise applies.
#[test]
fn the_nodes_mainnet_constants_round_trip_through_the_premise() {
    let spec = ergo_chain_spec::ChainSpec::mainnet();
    let trees = spec.emission_script_trees().unwrap();
    let nft = spec.reemission.as_ref().unwrap().emission_nft_id;
    let monetary = spec.monetary;
    let constants = EmissionConstants {
        fixed_rate: monetary.fixed_rate,
        fixed_rate_period: monetary.fixed_rate_period,
        epoch_length: monetary.epoch_length,
        one_epoch_reduction: monetary.one_epoch_reduction,
        founders_initial_reward: monetary.founders_initial_reward,
        miner_reward_delay: monetary.miner_reward_delay,
        emission_nft_id: hex::encode(nft.as_bytes()),
        emission_tree: hex::encode(&trees.emission),
    };
    // Script version 3 is the one mainnet activates today.
    assert_eq!(
        constants.node(3).unwrap(),
        EmissionRuleInputs {
            monetary,
            emission_nft_id: *nft.as_bytes(),
            emission_tree: trees.emission,
        }
    );
}
