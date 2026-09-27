//! Re-key authenticated historical expectations without changing their answers.
#![allow(dead_code)]
mod request;
use ergo_sandbox::evidence::case::engine_revision;
use ergo_sandbox::evidence::validate::ValidationRequest;
pub use request::{fixture_revision, on_current_engine};

/// Re-key a frozen expected report for an explicitly different revision. This
/// changes only revision fields and hashes derived from their original JSON;
/// no verdict, cost, transaction byte, property or expected answer is inferred
/// from the new engine. Unknown/unexplained changed digests still fail equality.
#[derive(Default)]
pub struct RevisionMap(std::collections::BTreeMap<String, String>);
impl RevisionMap {
    pub fn seed(&mut self, historical: &serde_json::Value) {
        self.expected(historical);
    }
    pub fn expected(&mut self, historical: &serde_json::Value) -> serde_json::Value {
        let mut prior = historical.clone();
        loop {
            let count = self.0.len();
            let next = self.walk(historical, false);
            if next == prior && count == self.0.len() {
                return next;
            }
            prior = next;
        }
    }
    fn walk(&mut self, old: &serde_json::Value, revision: bool) -> serde_json::Value {
        use ergo_sandbox::evidence::case::json_digest;
        use serde_json::Value;
        use sha2::{Digest, Sha256};
        let new = match old {
            Value::String(s) if revision && s == fixture_revision() => {
                Value::String(engine_revision().into())
            }
            Value::String(s) => Value::String(self.0.get(s).unwrap_or(s).clone()),
            Value::Array(a) => Value::Array(a.iter().map(|v| self.walk(v, revision)).collect()),
            Value::Object(o) => Value::Object(
                o.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            if k.ends_with("Sha256") {
                                // These identify immutable source artifacts,
                                // not a fresh execution or relation identity.
                                v.clone()
                            } else {
                                self.walk(
                                    v,
                                    revision
                                        || matches!(
                                            k.as_str(),
                                            "engineRevision" | "nodeRevision" | "compilerRevision"
                                        ),
                                )
                            },
                        )
                    })
                    .collect(),
            ),
            _ => old.clone(),
        };
        if old != &new && (old.is_object() || old.is_array()) {
            self.0.insert(json_digest(old), json_digest(&new));
            self.0.insert(
                hex::encode(Sha256::digest(serde_json::to_vec_pretty(old).unwrap())),
                hex::encode(Sha256::digest(serde_json::to_vec_pretty(&new).unwrap())),
            );
            if let (Ok(old), Ok(new)) = (
                serde_json::from_value::<ergo_sandbox::map::relations::RelationProposal>(
                    old.clone(),
                ),
                serde_json::from_value::<ergo_sandbox::map::relations::RelationProposal>(
                    new.clone(),
                ),
            ) {
                self.0.insert(old.claim_digest(), new.claim_digest());
            }
        }
        new
    }
}

/// Block costs the pinned engine charges differently from the revision a historical record was
/// measured at (docs/reports/node-pin-a203cc02.md). Each new cost is the one the Scala 6.0.6
/// reference node charges for the same transaction, so each corrects the older engine rather than
/// taking a new measurement on trust. A correction applies only where the record holds exactly
/// the historical cost; any other change still fails.
pub const COST_CORRECTIONS: &[(&str, u64, u64)] = &[
    // P03: the rent path is charged `StorageContractCost` (50) as block cost, as Scala does.
    ("storage-rent-acceptance", 12_105, 12_150),
    // M05 stop executions: JIT-cost parity with Scala on the node's side.
    ("m05/original", 14_175, 14_176),
    ("m05/different-true-expression", 14_180, 14_185),
    // P05 USE incident: block 1,868,204's five preceding transactions (two now cost 1,260 and 180 more).
    ("use-incident/prior-block-cost", 194_317, 195_757),
];

/// The cost the record `key` holds at the historical revision, as the pinned engine charges it.
pub fn corrected_cost(key: &str, historical: u64) -> u64 {
    match COST_CORRECTIONS.iter().find(|(k, _, _)| *k == key) {
        Some((_, old, new)) if *old == historical => *new,
        Some((_, old, _)) => {
            panic!("{key}: the record holds {historical}; the correction is for {old}")
        }
        None => historical,
    }
}

/// A historical request whose prior block cost the pinned engine charges differently.
pub fn with_corrected_prior_cost(request: ValidationRequest, key: &str) -> ValidationRequest {
    let mut v = serde_json::to_value(&request).unwrap();
    let historical = v["priorBlockCost"]["value"].as_u64().unwrap();
    v["priorBlockCost"]["value"] = serde_json::json!(corrected_cost(key, historical));
    serde_json::from_value(v).unwrap()
}

pub fn mapping_revisions() -> RevisionMap {
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mapping/m03");
    let mut map = RevisionMap::default();
    for (file, hash) in [
        (
            "manifest.json",
            "cdf73504cc3d5f98141bac9e44b46621a76baf8ac5219826d03367d171383853",
        ),
        (
            "supplemental-v2.fixture",
            "25b20fb46cdd45ee199e3847321419fef765505a0db8d61b0bbc0499c40adfda",
        ),
    ] {
        let bytes = std::fs::read(root.join(file)).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)), hash);
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        for entry in manifest["vectors"].as_array().unwrap() {
            let bytes = std::fs::read(root.join(entry["path"].as_str().unwrap())).unwrap();
            assert_eq!(hex::encode(Sha256::digest(&bytes)), entry["sha256"]);
            map.seed(&serde_json::from_slice::<Value>(&bytes).unwrap());
        }
    }
    for (file, hash) in [
        (
            "m03-mapping-results.json",
            "7c64e6d0a26c05720128d2dc4891e408d76bdbd6a7d5ff226e0bf4d9b9087ba6",
        ),
        (
            "m03-omission-results.json",
            "2b4dd32bcdc5d41c0974e3718383744e0f6c903eae4ce5e4048695ad40c42ae1",
        ),
    ] {
        let bytes = std::fs::read(root.join("../../../../../docs/mapping").join(file)).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)), hash);
        map.seed(&serde_json::from_slice::<Value>(&bytes).unwrap());
    }
    map
}
