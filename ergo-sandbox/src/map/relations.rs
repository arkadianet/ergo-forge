//! Unresolved relationship proposals, deliberately without proof or execution authority.
//! Imports here reject proof states. M03 establishment requires a fresh call to
//! `check_relation::check`; no proposal constructor certifies a relation or action.
use crate::evidence::case::{json_digest, Premise, RecordedBox};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selector {
    BoxId {
        hex: String,
    },
    PropositionBytes {
        hex: String,
    },
    PropositionHash {
        hex: String,
    },
    /// Amount is a minimum. M03 proves minimum 1 under positive-input-token state.
    TokenAt {
        index: u32,
        id: String,
        amount: u64,
    },
    TokenMember {
        id: String,
        amount: u64,
    },
    And {
        predicates: Vec<Selector>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Subject {
    BoxId { hex: String },
    Script { hex: String },
}

/// Field scope and literal type are explicit; these are syntax, not evaluated guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Guard {
    True,
    /// Narrow ordered guard required by the pinned action_alternatives family.
    HeightAtLeast {
        value: i32,
    },
    Equals {
        field: GuardField,
        literal: Literal,
    },
    And {
        guards: Vec<Guard>,
    },
    Or {
        guards: Vec<Guard>,
    },
    Not {
        guard: Box<Guard>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "kebab-case", deny_unknown_fields)]
pub enum GuardField {
    Context { name: String },
    SelfBox { name: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum Literal {
    Boolean(bool),
    Int(i32),
    Long(i64),
    Bytes(String),
}

/// The sole guard lives in P. Opaque state constraints are retained, not interpreted.
/// Missing bytes/references remain missing; imports never resolve them from a map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Premises {
    pub root_bytes: Premise<String>,
    pub self_box: Premise<RecordedBox>,
    pub node_revision: Premise<String>,
    pub compiler_revision: Premise<String>,
    pub network: Premise<String>,
    pub activation_rules: Premise<Value>,
    pub state_constraints: Premise<Value>,
    pub guard: Guard,
    pub authentication_roots: Vec<RecordedBox>,
    pub provenance: Premise<Value>,
    pub analysis_caps: BTreeMap<String, u64>,
}
impl Premises {
    pub fn digest(&self) -> String {
        json_digest(&serde_json::to_value(self).expect("premises serialize"))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Relation {
    Spend {
        selector: Selector,
    },
    Authenticates {
        field: String,
        expected: Literal,
    },
    Execute {
        input: String,
        variable: u8,
        code_digest: String,
    },
}

/// There is intentionally no deserializable established/refuted variant in M01.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalStatus {
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationProposal {
    pub subject: Subject,
    pub premises: Premises,
    pub target: Relation,
    pub status: ProposalStatus,
    pub reason: String,
    /// Unchecked legacy proposal material, never a certificate. M03 takes its own
    /// typed Derivation and rechecks it against the exact root.
    pub proposed_derivation: Value,
    pub dependency_ids: Vec<String>,
    pub exact_anchors: Vec<String>,
    pub authenticated_bytes: Vec<String>,
}
impl RelationProposal {
    pub fn claim_digest(&self) -> String {
        json_digest(
            &serde_json::json!({"version":"required-relations:v1", "subject":self.subject, "premises":self.premises, "target":self.target}),
        )
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    version: Version,
    proposals: Vec<BoundProposal>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Version {
    #[serde(rename = "required-relations:v1")]
    V1,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundProposal {
    claim_digest: String,
    premise_digest: String,
    proposal: RelationProposal,
}

/// Serializable proposals only. Deserialize always checks binding and rejects proof states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Envelope", into = "Envelope")]
pub struct RequiredRelations {
    proposals: Vec<RelationProposal>,
}
impl RequiredRelations {
    pub fn new(proposals: Vec<RelationProposal>) -> Result<Self, String> {
        if proposals.iter().any(|p| p.reason.trim().is_empty()) {
            return Err("unresolved proposal requires a reason".into());
        }
        Ok(Self { proposals })
    }
    pub fn proposals(&self) -> &[RelationProposal] {
        &self.proposals
    }
}
impl From<RequiredRelations> for Envelope {
    fn from(value: RequiredRelations) -> Self {
        Self {
            version: Version::V1,
            proposals: value
                .proposals
                .into_iter()
                .map(|proposal| BoundProposal {
                    claim_digest: proposal.claim_digest(),
                    premise_digest: proposal.premises.digest(),
                    proposal,
                })
                .collect(),
        }
    }
}
impl TryFrom<Envelope> for RequiredRelations {
    type Error = String;
    fn try_from(value: Envelope) -> Result<Self, String> {
        for p in &value.proposals {
            if p.claim_digest != p.proposal.claim_digest()
                || p.premise_digest != p.proposal.premises.digest()
            {
                return Err(
                    "claim/premise binding changed; imported material requires rechecking".into(),
                );
            }
        }
        Self::new(value.proposals.into_iter().map(|p| p.proposal).collect())
    }
}
