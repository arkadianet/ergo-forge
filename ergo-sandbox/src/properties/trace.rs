//! Supplied UTXO linkage only. The pinned node remains the step interpreter.
use super::schema::{check_collection_limits, check_trace_limit};
use crate::evidence::{
    validate::{validate, AcceptedExecution, ValidationRequest},
    wire::{WireBox, WireTransaction},
    RecordedBox,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct SuppliedTrace {
    pub root: Vec<RecordedBox>,
    pub steps: Vec<ValidationRequest>,
    /// Exact action bytes and environmental premises, in execution order.
    pub schedule: Vec<Value>,
}
pub fn schedule_step(r: &ValidationRequest) -> Value {
    json!({"transactionBytes":r.transaction_bytes,"parameters":r.parameters,"networkRules":r.network_rules,"blockContext":r.block_context,"headers":r.headers,"localPolicy":r.local_policy,"priorBlockCost":r.prior_block_cost})
}
pub fn digest<T: serde::Serialize>(v: &T) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(v).expect("serializable premise"),
    ))
}
pub(crate) struct Step {
    pub accepted: AcceptedExecution,
    pub inputs: Vec<WireBox>,
    pub outputs: Vec<WireBox>,
    pub data: Vec<WireBox>,
    pub prefix: String,
}
pub(crate) fn check(trace: &SuppliedTrace) -> Result<Vec<Step>, String> {
    check_trace_limit(trace.steps.len()).map_err(|e| e.to_string())?;
    if trace.schedule.len() != trace.steps.len() {
        return Err("unresolved: missing schedule steps".into());
    }
    let mut live = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for r in &trace.root {
        let b = WireBox::from_record(r.clone())?;
        let id = b.id()?;
        if !seen.insert(id.clone()) {
            return Err("duplicate root box".into());
        }
        live.insert(id, b);
    }
    let mut result: Vec<Step> = vec![];
    let mut prefix = digest(&trace.root);
    for (index, r) in trace.steps.iter().enumerate() {
        if schedule_step(r) != trace.schedule[index] {
            return Err("unresolved: schedule mismatch".into());
        }
        let context = r.block_context.value().ok_or("missing context")?;
        if let Some(previous) = result.last() {
            let old = previous.accepted.request();
            let before = old
                .block_context
                .value()
                .ok_or("missing previous context")?;
            if context.height < before.height
                || context.pre_header_timestamp < before.pre_header_timestamp
            {
                return Err("unresolved: reversed context schedule".into());
            }
            if context.height == before.height {
                let mut a = schedule_step(old);
                let mut b = schedule_step(r);
                for v in [&mut a, &mut b] {
                    v.as_object_mut().unwrap().remove("transactionBytes");
                    v.as_object_mut().unwrap().remove("priorBlockCost");
                }
                if a != b
                    || r.prior_block_cost.value().copied()
                        != previous.accepted.report()["totalBlockCost"].as_u64()
                {
                    return Err(
                        "unresolved: same-block environment or cumulative cost mismatch".into(),
                    );
                }
            }
        }
        for record in r.case.premises().boxes.value().ok_or("missing snapshot")? {
            let b = WireBox::from_record(record.clone())?;
            if live.get(&b.id()?).is_none_or(|old| old.node() != b.node()) {
                return Err("unresolved: unavailable or unexplained snapshot box".into());
            }
        }
        // No caller-supplied acceptance is consumed; even the trigger is validated.
        let accepted =
            validate(r).map_err(|e| format!("execution-rejected at {index}: {}", e.detail))?;
        let tx = WireTransaction::from_bytes(
            &hex::decode(&r.transaction_bytes).map_err(|e| e.to_string())?,
            &hex::encode(accepted.checked().tx_id()),
        )?;
        let lookup = |id: String| {
            live.get(&id)
                .cloned()
                .ok_or("unresolved: unavailable input/data input".to_owned())
        };
        let inputs = tx
            .node()
            .inputs
            .iter()
            .map(|i| lookup(hex::encode(i.box_id.as_bytes())))
            .collect::<Result<Vec<_>, _>>()?;
        let data = tx
            .node()
            .data_inputs
            .iter()
            .map(|i| lookup(hex::encode(i.box_id.as_bytes())))
            .collect::<Result<Vec<_>, _>>()?;
        let outputs = tx.output_boxes()?;
        for boxes in [&inputs, &outputs, &data] {
            check_collection_limits(
                boxes.len(),
                boxes.iter().map(|b| b.node().candidate.tokens.len()),
            )
            .map_err(|e| e.to_string())?;
        }
        for b in &inputs {
            live.remove(&b.id()?);
        }
        for b in &outputs {
            let id = b.id()?;
            if !seen.insert(id.clone()) {
                return Err("reused output identity".into());
            }
            live.insert(id, b.clone());
        }
        prefix = digest(&(prefix, r.fingerprint()));
        result.push(Step {
            accepted,
            inputs,
            outputs,
            data,
            prefix: prefix.clone(),
        });
    }
    Ok(result)
}
