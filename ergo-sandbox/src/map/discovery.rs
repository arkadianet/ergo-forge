//! Discovery observations carry no necessity, accepted execution, or action authority.
use super::relations::Selector;
use crate::evidence::case::{Premise, RecordedBox};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoveryVersion {
    #[serde(rename = "discovery-map:v2")]
    V2,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchStatus {
    Hint,
    ObservedMatch,
    RejectedMatch,
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub source_box: String,
    pub site: String,
    pub reference: Selector,
    pub proposed_targets: Vec<String>,
    pub proposed_roles: Vec<String>,
    pub status: MatchStatus,
    pub reason: String,
}
/// No implicit conversion into relation proposals or a legacy action request.
///
/// ```compile_fail
/// use ergo_sandbox::map::{discovery::DiscoveryMap, relations::RequiredRelations};
/// fn conversion<T: From<DiscoveryMap>>() {}
/// conversion::<RequiredRelations>();
/// ```
/// ```compile_fail
/// use ergo_sandbox::{map::discovery::DiscoveryMap, DrainRequest};
/// fn conversion<T: From<DiscoveryMap>>() {}
/// conversion::<DrainRequest>();
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryMap {
    pub version: DiscoveryVersion,
    pub seeds: Vec<Value>,
    /// Raw input before legacy defaulting. No creation reference is manufactured.
    pub recorded_boxes: Vec<RecordedBox>,
    pub canonical_references: BTreeMap<String, String>,
    pub observed_code: BTreeMap<String, String>,
    pub observations: Vec<Observation>,
    pub source_queries: Vec<Value>,
    pub snapshot_scope: Premise<Value>,
    pub caps: BTreeMap<String, u64>,
    pub unresolved_reasons: Vec<String>,
}
/// Only a candidate identifier; resolving independent evidence is a separate operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nomination {
    discovery_id: String,
}
impl Nomination {
    pub fn discovery_id(&self) -> &str {
        &self.discovery_id
    }
}
impl DiscoveryMap {
    pub fn nominate(&self, id: &str) -> Result<Nomination, String> {
        if self.observations.iter().filter(|o| o.id == id).count() != 1 {
            return Err("nomination requires one unambiguous discovery ID".into());
        }
        Ok(Nomination {
            discovery_id: id.into(),
        })
    }
}
