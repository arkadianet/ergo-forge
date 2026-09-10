use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Provenance is distinct from validity. A source recording is not a UTXO proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    Hypothetical,
    CallerSupplied,
    SourceRecorded,
    IndependentlyChecked,
}

/// Missing and explicitly empty are different inputs. No serde defaults are used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Premise<T> {
    Missing { reason: String },
    Present { value: T, origin: Origin },
}
impl<T> Premise<T> {
    pub fn missing(reason: impl Into<String>) -> Self {
        Self::Missing {
            reason: reason.into(),
        }
    }
    pub fn supplied(value: T) -> Self {
        Self::Present {
            value,
            origin: Origin::CallerSupplied,
        }
    }
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Present { value, .. } => Some(value),
            _ => None,
        }
    }
    fn validate_origin(&self) -> Result<(), String> {
        match self {
            Self::Missing { reason } if reason.is_empty() => Err("missing premise needs a reason".into()),
            Self::Present { origin: Origin::IndependentlyChecked, .. } => Err("independent state/binding checks are not implemented; record the supplied/source provenance instead".into()),
            _ => Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRecord {
    pub locator: String,
    pub revision: Option<String>,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceIdentity {
    pub record: SourceRecord,
    pub text: Premise<String>,
}
impl SourceIdentity {
    pub fn supplied(text: &str) -> Self {
        Self {
            record: SourceRecord {
                locator: "inline-source".into(),
                revision: None,
                sha256: digest(text.as_bytes()),
            },
            text: Premise::supplied(text.into()),
        }
    }
    fn validate(&self) -> Result<(), String> {
        if self.record.locator.is_empty() || !valid_digest(&self.record.sha256) {
            return Err("source requires a locator and SHA-256 identity".into());
        }
        self.text.validate_origin()?;
        if let Some(text) = self.text.value() {
            if digest(text.as_bytes()) != self.record.sha256 {
                return Err("attached source does not match its identity".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstantBinding {
    pub name: String,
    pub typed_value: Value,
    pub origin: Origin,
    pub mechanism: String,
    pub source_lines: Vec<usize>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingSet {
    pub bindings: Vec<ConstantBinding>,
    /// False preserves the partial binding record from a failed compilation.
    pub complete: bool,
}

/// Original source JSON, before ChainBox/ScenarioBox defaulting. No wire codec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordedBox {
    document: Value,
    origin: Origin,
    record: Option<SourceRecord>,
}
impl RecordedBox {
    pub fn new(
        document: Value,
        origin: Origin,
        record: Option<SourceRecord>,
    ) -> Result<Self, String> {
        let result = Self {
            document,
            origin,
            record,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), String> {
        if !self.document.is_object() {
            return Err("box recording must be a JSON object".into());
        }
        if self.origin == Origin::IndependentlyChecked {
            return Err("box state has not been independently checked".into());
        }
        if let Some(record) = &self.record {
            if record.locator.is_empty() || record.sha256 != json_digest(&self.document) {
                return Err("box recording identity mismatch".into());
            }
        }
        if self.origin == Origin::SourceRecorded && self.record.is_none() {
            return Err("source-recorded box requires a record identity".into());
        }
        Ok(())
    }
    pub fn document(&self) -> &Value {
        &self.document
    }
    /// Presence is read from the retained document, never a defaulted ChainBox.
    pub fn registers(&self) -> Premise<Value> {
        let field = self
            .document
            .get("additionalRegisters")
            .or_else(|| self.document.get("registers"));
        match field {
            Some(v) if v.is_object() => Premise::Present {
                value: v.clone(),
                origin: self.origin,
            },
            _ => Premise::missing(
                "registers omitted, null, or not represented as an object in source document",
            ),
        }
    }
}

/// All premises participate in identity, including unknowns, defaults, roles,
/// source locators, caller policy and budgets. Their JSON is retained verbatim
/// in value (object ordering is immaterial), not marshalled to legacy boxes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CasePremises {
    pub engine_revision: String,
    pub target_bytes: Premise<String>,
    pub source: Premise<SourceIdentity>,
    pub constants: Premise<BindingSet>,
    pub boxes: Premise<Vec<RecordedBox>>,
    pub context: Premise<Value>,
    pub assumptions: BTreeMap<String, Premise<Value>>,
}
impl CasePremises {
    pub fn unspecified() -> Self {
        Self {
            engine_revision: engine_revision().into(),
            target_bytes: Premise::missing("target bytes not supplied"),
            source: Premise::missing("source identity not supplied"),
            constants: Premise::missing("binding origins not supplied"),
            boxes: Premise::missing("state not supplied; no unspent-state assertion"),
            context: Premise::missing("execution context not supplied"),
            assumptions: BTreeMap::new(),
        }
    }
    fn validate(&self) -> Result<(), String> {
        if self.engine_revision.len() != 40 || hex::decode(&self.engine_revision).is_err() {
            return Err("engine revision must be a full git identity".into());
        }
        self.target_bytes.validate_origin()?;
        if let Some(bytes) = self.target_bytes.value() {
            if bytes.is_empty() || hex::decode(bytes).is_err() {
                return Err(
                    "target bytes must be nonempty hex; no placeholder substitution".into(),
                );
            }
        }
        self.source.validate_origin()?;
        if let Some(source) = self.source.value() {
            source.validate()?;
        }
        self.constants.validate_origin()?;
        if let Some(set) = self.constants.value() {
            let mut names = std::collections::BTreeSet::new();
            for b in &set.bindings {
                if b.name.is_empty()
                    || b.mechanism.is_empty()
                    || !names.insert(&b.name)
                    || b.origin == Origin::IndependentlyChecked
                {
                    return Err(
                        "invalid, duplicate, or unsupported independently-checked binding".into(),
                    );
                }
            }
        }
        self.boxes.validate_origin()?;
        if let Some(boxes) = self.boxes.value() {
            for b in boxes {
                b.validate()?;
            }
        }
        self.context.validate_origin()?;
        for p in self.assumptions.values() {
            p.validate_origin()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DeploymentIdentity {
    Unknown,
}

/// Output claims cannot be imported into this input record. Independently
/// checked source hashing is recomputed on import; it proves only attached bytes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceCase {
    format_version: u32,
    deployment_identity: DeploymentIdentity,
    node_validated: bool,
    premises: CasePremises,
}
impl<'de> Deserialize<'de> for EvidenceCase {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Record {
            format_version: u32,
            deployment_identity: DeploymentIdentity,
            node_validated: bool,
            premises: CasePremises,
        }
        let r = Record::deserialize(de)?;
        if r.format_version != 1
            || r.node_validated
            || r.deployment_identity != DeploymentIdentity::Unknown
        {
            return Err(serde::de::Error::custom(
                "unsupported evidence-case version or acceptance claim",
            ));
        }
        Self::new(r.premises).map_err(serde::de::Error::custom)
    }
}
impl EvidenceCase {
    pub fn new(premises: CasePremises) -> Result<Self, String> {
        premises.validate()?;
        Ok(Self {
            format_version: 1,
            deployment_identity: DeploymentIdentity::Unknown,
            node_validated: false,
            premises,
        })
    }
    pub fn premises(&self) -> &CasePremises {
        &self.premises
    }
    /// A changed case is a new cache key, never an in-place evidence repair.
    pub fn with_premises(&self, premises: CasePremises) -> Result<Self, String> {
        Self::new(premises)
    }
    pub fn fingerprint(&self) -> String {
        json_digest(&serde_json::to_value(self).expect("case serializes"))
    }
    pub fn target_bytes(&self) -> Result<Vec<u8>, String> {
        if self.premises.engine_revision != engine_revision() {
            return Err("case targets a different engine revision; analysis unavailable".into());
        }
        hex::decode(
            self.premises
                .target_bytes
                .value()
                .ok_or("target bytes missing; analysis unavailable")?,
        )
        .map_err(|e| e.to_string())
    }
    pub fn analyze(&self) -> Result<Analysis<StaticAnalysis>, String> {
        let tree = crate::inspect::parse_tree(&self.target_bytes()?).map_err(|e| e.to_string())?;
        let lifted = crate::lift_tree(&tree, false);
        let audit = crate::audit::audit(&lifted);
        Ok(Analysis::new(
            self,
            &[],
            StaticAnalysis {
                obligations: audit.obligations,
                source: crate::decompile::print(&lifted.node),
                completeness: audit.completeness,
                findings: audit.findings,
            },
        ))
    }
    pub fn compare(&self, other: &Self) -> Result<Analysis<crate::identity::MatchReport>, String> {
        let result = crate::identity::match_trees(&self.target_bytes()?, &other.target_bytes()?)
            .map_err(|e| e.to_string())?;
        Ok(Analysis::new(self, &[other], result))
    }
}

#[derive(Debug, Serialize)]
pub struct StaticAnalysis {
    pub obligations: Vec<crate::audit::obligation::Obligation>,
    pub source: String,
    pub completeness: crate::audit::Completeness,
    pub findings: Vec<crate::Finding>,
}

/// Cache entry constructed only by analysis. It retains every input snapshot;
/// imported JSON is not a reusable analysis result (no Deserialize).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis<T> {
    method: &'static str,
    provenance: &'static str,
    case: EvidenceCase,
    dependencies: Vec<EvidenceCase>,
    input_fingerprint: String,
    node_validated: bool,
    deployment_identity: DeploymentIdentity,
    source_digest_check: Option<DigestCheck>,
    result: T,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DigestCheck {
    origin: Origin,
    method: &'static str,
    sha256: String,
}
impl<T> Analysis<T> {
    fn new(case: &EvidenceCase, dependencies: &[&EvidenceCase], result: T) -> Self {
        Self {
            method: "static-analysis",
            provenance: "case-bound inputs; deployment identity not established",
            case: case.clone(),
            dependencies: dependencies.iter().map(|c| (*c).clone()).collect(),
            input_fingerprint: input_fingerprint(case, dependencies),
            node_validated: false,
            deployment_identity: DeploymentIdentity::Unknown,
            source_digest_check: case.premises.source.value().and_then(|s| {
                s.text.value().map(|text| DigestCheck {
                    origin: Origin::IndependentlyChecked,
                    method: "sha256-of-attached-source-only",
                    sha256: digest(text.as_bytes()),
                })
            }),
            result,
        }
    }
    /// Returns no cached result if any premise of either side has changed.
    pub fn result_for(&self, case: &EvidenceCase, dependencies: &[&EvidenceCase]) -> Option<&T> {
        (self.input_fingerprint == input_fingerprint(case, dependencies)).then_some(&self.result)
    }
}
fn input_fingerprint(case: &EvidenceCase, dependencies: &[&EvidenceCase]) -> String {
    json_digest(&serde_json::json!({"analysisVersion":1,"case":case,"dependencies":dependencies}))
}
pub fn engine_revision() -> &'static str {
    include_str!("../../../Cargo.toml")
        .lines()
        .find(|line| line.starts_with("ergo-compiler "))
        .and_then(|line| line.split("rev").nth(1))
        .and_then(|s| s.split('"').nth(1))
        .expect("workspace pins ergo-compiler")
}
fn valid_digest(s: &str) -> bool {
    s.len() == 64 && hex::decode(s).is_ok()
}
fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
/// Canonical JSON identity of a record, NOT Ergo wire serialization or a box ID.
pub fn json_digest(value: &Value) -> String {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(m) => {
                let ordered: BTreeMap<_, _> =
                    m.iter().map(|(k, v)| (k.clone(), sorted(v))).collect();
                Value::Object(ordered.into_iter().collect())
            }
            Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
            v => v.clone(),
        }
    }
    digest(&serde_json::to_vec(&sorted(value)).expect("JSON serializes"))
}
