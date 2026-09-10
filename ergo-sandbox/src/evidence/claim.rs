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
        json!({"status":"confirmed-violation","bundleFingerprint":self.fingerprint,"accounting":self.accounting,"impact":{"severity":null,"reason":"impact beyond the declared extraction accounting has not been assessed"},"propertyVersion":PROPERTY_VERSION,"scope":"node-accepted under supplied premises and violates the declared property; not a historical exploit or inclusion claim"})
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

/// A review decision is a caller assertion, never execution evidence. A saved
/// decision remains visible even after its dependencies invalidate it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewDecision {
    pub obligation_key: String,
    pub premise_fingerprint: String,
    pub reason: String,
}

/// Assemble the obligation queue and freshly replayed property results in
/// separate fields. No stored claim/status is accepted as authority.
pub fn obligation_report(
    case: &super::EvidenceCase,
    context: &crate::audit::ContextAudit,
    decisions: &[ReviewDecision],
    bundles: &[super::replay::ReplayBundle],
) -> Result<Value, String> {
    let bytes = case.target_bytes()?;
    let tree = crate::inspect::parse_tree(&bytes).map_err(|e| e.to_string())?;
    // Address rendering depends on the caller's display network, not semantics.
    let recovery_matches = [false, true].into_iter().any(|testnet| {
        crate::decompile::print(&crate::lift_tree(&tree, testnet).node)
            == context.target_recovered_code
    });
    if !recovery_matches {
        return Err("context audit does not match case target recovery".into());
    }
    let premises = json!({"case":case,"conditionalContext":context.premises,
        "targetRecovery":context.target_recovered_code,"completeness":context.completeness,
        "propertyBundles":bundles});
    let fingerprint = super::case::json_digest(&json!({"reportVersion":1,"premises":premises,
        "observations":context.findings,"obligations":context.obligations}));
    let obligations = context.obligations.iter().map(|g| {
        let discharges = &g.discharges;
        let matching = decisions.iter().filter(|d| d.obligation_key == g.key).collect::<Vec<_>>();
        let suppressed = matching.iter().any(|d| d.premise_fingerprint == fingerprint && !d.reason.trim().is_empty());
        json!({"key":g.key,"category":g.category,"reviewPriority":g.review_priority,
            "anchors":g.anchors,"status":if suppressed {"suppressed-under-premises"}
                else if discharges.len() == g.anchors.len() {"conditionally-discharged"} else {"unresolved"},
            "discharges":discharges,"premiseFingerprint":fingerprint})
    }).collect::<Vec<_>>();
    let decision_records = decisions
        .iter()
        .map(|d| {
            json!({"decision":d,
        "valid":d.premise_fingerprint == fingerprint && !d.reason.trim().is_empty()
            && context.obligations.iter().any(|g|g.key == d.obligation_key)})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"formatVersion":1,"method":"obligation-report","premises":premises,
        "premiseFingerprint":fingerprint,"obligations":obligations,"reviewDecisions":decision_records,
        "propertyResults":bundles.iter().map(super::replay::replay).collect::<Vec<_>>(),
        "completeness":context.completeness}),
    )
}
