//! Declared extraction property evaluated only on a fresh AcceptedExecution.
use super::validate::AcceptedExecution;
use crate::drain::{
    accounting, DrainInput, DrainOutput, DrainRequest, DrainRole, ObjectivePolicy, Payee, Synthesis,
};
use crate::scenario::ScenarioBox;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROPERTY_VERSION: &str = "recognized-attacker-receipts-v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputBinding {
    pub box_id: String,
    pub role: DrainRole,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Property {
    pub version: String,
    pub inputs: Vec<InputBinding>,
    pub attacker_public_keys: Vec<String>,
    pub objective: Option<ObjectivePolicy>,
}
/// Not deserializable or publicly constructible; bound to complete bundle identity.
///
/// ```compile_fail
/// use ergo_sandbox::evidence::claim::ConfirmedViolation;
/// let claim: ConfirmedViolation = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use ergo_sandbox::evidence::claim::ConfirmedViolation;
/// let claim = ConfirmedViolation {};
/// ```
#[derive(Debug)]
pub struct ConfirmedViolation {
    fingerprint: String,
    accounting: Value,
}
impl ConfirmedViolation {
    pub fn report(&self) -> Value {
        json!({"status":"confirmed-violation","bundleFingerprint":self.fingerprint,"accounting":self.accounting,"propertyVersion":PROPERTY_VERSION,"scope":"node-accepted under supplied premises and violates the declared property; not a historical exploit or inclusion claim"})
    }
}
pub enum PropertyResult {
    Violation(ConfirmedViolation),
    Nonviolating(Value),
}
/// Projection only: these ScenarioBox values never enter a reducer or serializer.
/// Accounting v1 reads only exact value, tokens and script bytes.
fn project(b: &ergo_ser::ergo_box::ErgoBoxCandidate) -> Result<ScenarioBox, String> {
    let value =
        i64::try_from(b.value).map_err(|_| "accounting v1 value outside i64 representation")?;
    serde_json::from_value(json!({"value":value,"ergoTree":hex::encode(b.ergo_tree_bytes()),"tokens":b.tokens.iter().map(|t|json!({"id":hex::encode(t.token_id.as_bytes()),"amount":t.amount})).collect::<Vec<_>>()})).map_err(|e|e.to_string())
}
pub(crate) fn evaluate(
    accepted: &AcceptedExecution,
    property: &Property,
    fingerprint: String,
) -> Result<PropertyResult, String> {
    if property.version != PROPERTY_VERSION {
        return Err("unknown extraction policy version".into());
    }
    let resolved = accepted.checked().resolved_inputs();
    if property.inputs.len() != resolved.len() {
        return Err("property must bind every spending input in transaction order".into());
    }
    let mut inputs = vec![];
    for (binding, b) in property.inputs.iter().zip(resolved) {
        if binding.role == DrainRole::Unknown
            || binding.box_id != hex::encode(b.box_id().map_err(|e| e.to_string())?)
        {
            return Err("wrong companion/input identity or unknown role".into());
        }
        inputs.push(DrainInput {
            role: binding.role,
            box_: project(&b.candidate)?,
        });
    }
    if !inputs.iter().any(|i| accounting::is_victim(i.role)) {
        return Err("property needs a protected or companion input".into());
    }
    let outputs = accepted
        .checked()
        .transaction()
        .output_candidates
        .iter()
        .map(project)
        .collect::<Result<Vec<_>, _>>()?;
    let request = DrainRequest {
        inputs,
        data_inputs: vec![],
        outputs: outputs
            .iter()
            .cloned()
            .map(|box_| DrainOutput {
                box_,
                payee: Payee::Fixed,
            })
            .collect(),
        protocol_nfts: vec![],
        height: 0,
        network: None,
        attacker_tree: None,
        attacker_public_keys: property.attacker_public_keys.clone(),
        objective: property.objective.clone(),
        max_permutations: None,
        max_probes: None,
        synthesis: Synthesis::default(),
    };
    let errors = accounting::objective_errors(&request);
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    let recognized = accounting::recognized_trees(&request).map_err(|e| e.to_string())?;
    let victim = accounting::sum_boxes(
        request
            .inputs
            .iter()
            .filter(|i| accounting::is_victim(i.role))
            .map(|i| &i.box_),
    );
    let roles = request.inputs.iter().map(|i| i.role).collect::<Vec<_>>();
    let boxes = request
        .inputs
        .iter()
        .map(|i| i.box_.clone())
        .collect::<Vec<_>>();
    let accounting = accounting::leak(&request, &victim, &roles, &boxes, &outputs, &recognized);
    let violation = !accounting.extracted.is_empty();
    let accounting = serde_json::to_value(accounting).map_err(|e| e.to_string())?;
    if violation {
        Ok(PropertyResult::Violation(ConfirmedViolation {
            fingerprint,
            accounting,
        }))
    } else {
        Ok(PropertyResult::Nonviolating(accounting))
    }
}
