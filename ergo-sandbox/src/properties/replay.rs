//! Offline, opt-in replay. Serialized reports are inspection data, never authority.
use super::{
    evaluate::{evaluate, Evaluation},
    schema::Declaration,
    trace::{digest, SuppliedTrace},
};
use crate::evidence::{
    validate::{node_revision, ValidationRequest},
    RecordedBox,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERSION: &str = "property-replay:v1";
/// Complete supplied experiment. No producer-supplied verdict is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayEnvelope {
    pub schema_version: String,
    pub declaration: Value,
    pub root: Vec<RecordedBox>,
    pub steps: Vec<ValidationRequest>,
    pub schedule: Vec<Value>,
    pub node_revision: String,
    pub evaluator_revision: String,
}
impl ReplayEnvelope {
    pub fn identity(&self) -> String {
        digest(self)
    }
}
/// Output only; even a valid-looking imported report cannot construct this type.
/// ```compile_fail
/// use ergo_sandbox::properties::replay::ReplayResult;
/// let _: ReplayResult = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug)]
pub struct ReplayResult {
    identity: String,
    evaluation: Evaluation,
}
impl ReplayResult {
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn report(&self) -> &Value {
        self.evaluation.report()
    }
}
pub fn replay(envelope: &ReplayEnvelope) -> Result<ReplayResult, String> {
    if envelope.schema_version != VERSION
        || envelope.node_revision != node_revision()
        || envelope.evaluator_revision != "author-property-evaluator:v1"
    {
        return Err("unsupported replay/node/evaluator version".into());
    }
    let declaration =
        Declaration::parse(&envelope.declaration.to_string()).map_err(|e| e.to_string())?;
    let evaluation = evaluate(
        &declaration,
        &SuppliedTrace {
            root: envelope.root.clone(),
            steps: envelope.steps.clone(),
            schedule: envelope.schedule.clone(),
        },
    )?;
    Ok(ReplayResult {
        identity: envelope.identity(),
        evaluation,
    })
}
/// Bind an import to the experiment identity retained by its consumer, then run
/// fresh validation. There is no import path for a saved evaluation result.
pub fn import(bytes: &str, expected_identity: &str) -> Result<ReplayResult, String> {
    let envelope: ReplayEnvelope = serde_json::from_str(bytes).map_err(|e| e.to_string())?;
    if envelope.identity() != expected_identity {
        return Err("experiment identity mismatch".into());
    }
    replay(&envelope)
}
