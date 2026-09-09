//! Explicit promotion of a freshly generated candidate. No search changes and
//! no repair of missing identities, token holdings, proofs or state premises.
use super::{
    case::json_digest,
    claim::{InputBinding, Property, PROPERTY_VERSION},
    replay::{replay, ReplayBundle},
    sign::{sign_owned_p2pk, OwnedDlogSecret, SigningFailure},
    validate::ValidationRequest,
    wire::{CandidateSpec, WireBox, WireTransaction},
};
use crate::drain::{drain_hunt, DrainReport, DrainRequest};
use ergo_primitives::reader::VlqReader;
use ergo_ser::transaction::read_transaction;
use serde::Serialize;
use serde_json::{json, Value};

/// Output-only association. It is neither an imported claim nor lint causation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Promotion {
    pub status: String,
    pub request_fingerprint: String,
    pub material_fingerprint: String,
    pub mapping: Value,
    pub replay: Option<Value>,
    pub failure: Option<String>,
}
impl Promotion {
    pub fn claim_reference(&self) -> Option<Value> {
        let r = self.replay.as_ref()?;
        (r["status"] == "confirmed-violation").then(|| {
            json!({
                "kind":"declared-property-claim", "status":"confirmed-violation",
                "bundleFingerprint":r["bundleFingerprint"],
                "scope":r["scope"],"lintCausationEstablished":false
            })
        })
    }
}

/// `material` supplies complete canonical transaction/boxes and validation
/// context, not a previous report. Search runs fresh with the original request.
/// The supplied transaction must match the generated candidate exactly, including
/// output IDs; only a supported funding proof may subsequently be inserted.
pub fn promote(
    request: &DrainRequest,
    material: &ValidationRequest,
    funding: Option<(usize, &OwnedDlogSecret)>,
) -> Result<DrainReport, crate::SandboxError> {
    let mut report = drain_hunt(request)?;
    let mut promotion = Promotion {
        status: "promotion-failed".into(),
        request_fingerprint: json_digest(&json!(request)),
        material_fingerprint: material.fingerprint(),
        mapping: Value::Null,
        replay: None,
        failure: None,
    };
    match bind(request, &report, material) {
        Err(e) => promotion.failure = Some(e),
        Ok((property, mapping)) => {
            promotion.mapping = mapping;
            let execution = if let Some((index, key)) = funding {
                match sign_owned_p2pk(material, index, key) {
                    Ok(a) => Ok(a.request().clone()),
                    Err(SigningFailure::Material(e)) => {
                        Err(format!("funding-proof-unavailable: {e}"))
                    }
                    Err(SigningFailure::Validation(e)) => {
                        Err(format!("{}: {}", e.status, e.detail))
                    }
                }
            } else {
                Ok(material.clone())
            };
            match execution {
                Err(e) => promotion.failure = Some(e),
                Ok(execution) => {
                    let replayed = replay(&ReplayBundle {
                        format_version: 1,
                        execution,
                        property,
                    });
                    promotion.status = replayed["status"]
                        .as_str()
                        .unwrap_or("promotion-failed")
                        .into();
                    if replayed["nodeValidated"] != true {
                        promotion.failure =
                            Some(format!("{}: {}", promotion.status, replayed["execution"]));
                    }
                    promotion.replay = Some(replayed);
                }
            }
        }
    }
    report.promotion = Some(promotion);
    Ok(report)
}

/// Strict conversion of the generator's complete box JSON. No default registers,
/// height, value, token supply or tree. Output identity is checked separately.
pub fn candidate_spec(value: &Value) -> Result<CandidateSpec, String> {
    let registers = value["additionalRegisters"]
        .as_object()
        .ok_or("missing-register-material")?;
    let mut raw = format!("{:02x}", registers.len());
    for index in 0..registers.len() {
        let v = registers
            .get(&format!("R{}", index + 4))
            .and_then(Value::as_str)
            .ok_or("noncanonical-register-material")?;
        raw.push_str(v);
    }
    serde_json::from_value(json!({
        "value":value["value"],"ergoTree":value["ergoTree"],
        "creationHeight":value["creationHeight"],"tokens":value["assets"].as_array().ok_or("missing-token-material")?.iter().map(|t|json!({"id":t["tokenId"],"amount":t["amount"]})).collect::<Vec<_>>(),
        "registers":raw
    })).map_err(|e|format!("incomplete-box-material: {e}"))
}
fn bind(
    request: &DrainRequest,
    report: &DrainReport,
    material: &ValidationRequest,
) -> Result<(Property, Value), String> {
    let hit = report.best.as_ref().ok_or("no-generated-candidate")?;
    let candidate = &hit.witness.tx_request;
    let bytes = hex::decode(&material.transaction_bytes).map_err(|e| e.to_string())?;
    let mut reader = VlqReader::new(&bytes);
    let tx = read_transaction(&mut reader).map_err(|e| e.to_string())?;
    let wire =
        WireTransaction::from_node(tx).map_err(|e| format!("noncanonical-transaction: {e}"))?;
    if !reader.is_empty() || wire.bytes() != bytes {
        return Err("noncanonical-transaction".into());
    }
    let context = material.block_context.value().ok_or("incomplete-context")?;
    if candidate.height != Some(context.height) {
        return Err("context-height-mismatch".into());
    }
    let records = material
        .case
        .premises()
        .boxes
        .value()
        .ok_or("unavailable-box-material")?;
    let boxes = records
        .iter()
        .cloned()
        .map(WireBox::from_record)
        .collect::<Result<Vec<_>, _>>()?;
    if boxes.len() != candidate.boxes.len()
        || wire.node().inputs.len() != candidate.tx.inputs.len()
        || wire.node().data_inputs.len() != candidate.tx.data_inputs.len()
        || wire.node().output_candidates.len() != candidate.tx.outputs.len()
    {
        return Err("candidate-material-count-mismatch".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for b in &boxes {
        let id = b.id()?;
        if !ids.insert(id.clone()) {
            return Err("duplicate-box-material".into());
        }
        let generated = candidate
            .boxes
            .iter()
            .find(|v| v["boxId"] == id)
            .ok_or("label-derived-or-unbacked-box-id")?;
        if candidate_spec(generated)?.build()? != b.node().candidate {
            return Err("unbacked-box-holdings-or-fields".into());
        }
    }
    let mut mapping = vec![];
    let mut property_inputs = vec![];
    for (position, (generated, input)) in candidate
        .tx
        .inputs
        .iter()
        .zip(&wire.node().inputs)
        .enumerate()
    {
        let id = hex::encode(input.box_id.as_bytes());
        if generated.box_id != id {
            return Err("input-identity-mismatch".into());
        }
        // Current drain family emits empty extensions. Refuse unsupported material
        // explicitly instead of erasing a caller's extension during promotion.
        if !generated.extension.is_empty() || !input.spending_proof.extension().values.is_empty() {
            return Err("unsupported-input-extension".into());
        }
        let declared = request
            .inputs
            .iter()
            .position(|i| i.box_.box_id.as_deref() == Some(&id))
            .ok_or("label-derived-or-synthetic-input")?;
        if !request.inputs[declared].box_.extension.is_empty() {
            return Err("unsupported-input-extension".into());
        }
        if hit.witness.roles[position] != request.inputs[declared].role {
            return Err("role-binding-mismatch".into());
        }
        mapping
            .push(json!({"declaredInput":declared,"candidateInput":position,"canonicalBoxId":id}));
        property_inputs.push(InputBinding {
            box_id: id,
            role: hit.witness.roles[position],
        });
    }
    for (generated, input) in candidate
        .tx
        .data_inputs
        .iter()
        .zip(&wire.node().data_inputs)
    {
        if generated.box_id != hex::encode(input.box_id.as_bytes()) {
            return Err("unavailable-data-box".into());
        }
        if !request
            .data_inputs
            .iter()
            .any(|b| b.box_id.as_deref() == Some(&generated.box_id))
        {
            return Err("unavailable-data-box".into());
        }
    }
    let outputs = wire.output_boxes()?;
    // Output contents are compared field-for-field. Their ids are NOT: a
    // generated candidate carries the search's own deterministic simulation
    // ids, and a canonical output id is derived from the transaction that only
    // exists once the candidate is built. Requiring equality would reject every
    // candidate by construction. Both ids are recorded in the mapping instead,
    // so a reader can see which is which. Input ids stay strict above, because
    // those reference boxes that already exist.
    let mut output_mapping = vec![];
    for (position, (generated, output)) in candidate.tx.outputs.iter().zip(&outputs).enumerate() {
        if candidate_spec(generated)?.build()? != output.node().candidate {
            return Err("candidate-output-mismatch".into());
        }
        output_mapping.push(json!({
            "candidateOutput": position,
            "canonicalBoxId": output.id()?,
            "simulationBoxId": generated["boxId"],
        }));
    }
    let mut objective = request.objective.clone();
    if let Some(policy) = &mut objective {
        for term in &mut policy.terms {
            term.source_input = mapping
                .iter()
                .position(|m| m["declaredInput"] == term.source_input)
                .ok_or("unbound-objective-source")?;
        }
    }
    Ok((
        Property {
            version: PROPERTY_VERSION.into(),
            inputs: property_inputs,
            attacker_public_keys: request.attacker_public_keys.clone(),
            objective,
        },
        json!({
            "inputs":mapping,"outputs":output_mapping,
            "candidateFingerprint":json_digest(&json!(hit)),"transactionId":wire.id(),
            "scope":"exact generated candidate under supplied state; provenance is in replay bundle; no historical availability claim"
        }),
    ))
}
