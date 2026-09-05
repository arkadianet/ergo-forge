//! Play: the one operation of a sandbox chain — apply a transaction to a
//! set of boxes. Every input's script is evaluated in the full transaction
//! context (`selfIndex`, all inputs, outputs, data inputs, that input's
//! context variables and secrets), ERG and tokens must balance (one new
//! token named after the first input may be minted), and the outputs come
//! back with the ids the chain would give them. The state itself lives
//! with the caller: the request carries the boxes, the response the new
//! ones. Nothing here touches a network.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::prove::{PartySpec, SecretSpec};
use crate::scenario::{ScenarioBox, TypedValue};
use crate::{eval_scenario, SandboxError, Scenario, Verdict};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayRequest {
    pub height: u32,
    /// The unspent boxes the transaction may use, each with its `boxId`.
    pub boxes: Vec<ScenarioBox>,
    pub tx: PlayTx,
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayTx {
    pub inputs: Vec<PlayInput>,
    #[serde(default)]
    pub data_inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<ScenarioBox>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayInput {
    pub box_id: String,
    /// This input's spending-proof extension.
    #[serde(default)]
    pub context_vars: BTreeMap<String, TypedValue>,
    #[serde(default)]
    pub secrets: Vec<SecretSpec>,
    #[serde(default)]
    pub parties: Vec<PartySpec>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayInputResult {
    pub box_id: String,
    /// `pass` / `proofAccepted` let the spend through; anything else stops it.
    pub verdict: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduced_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub cost: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayResult {
    pub ok: bool,
    pub tx_id: String,
    pub inputs: Vec<PlayInputResult>,
    /// The outputs with their ids and creation height, ready to be boxes.
    pub outputs: Vec<ScenarioBox>,
    pub problems: Vec<String>,
    pub erg_in: u64,
    pub erg_out: u64,
}

fn find<'a>(
    boxes: &'a [ScenarioBox],
    id: &str,
    what: &str,
) -> Result<&'a ScenarioBox, SandboxError> {
    boxes
        .iter()
        .find(|b| {
            b.box_id
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(id))
                .unwrap_or(false)
        })
        .ok_or_else(|| SandboxError::Scenario(format!("{what} {id}: no such unspent box")))
}

/// Apply `req.tx` to `req.boxes` at `req.height`.
pub fn apply(req: &PlayRequest) -> Result<PlayResult, SandboxError> {
    if req.tx.inputs.is_empty() {
        return Err(SandboxError::Scenario(
            "a transaction needs at least one input".into(),
        ));
    }
    let inputs: Vec<&ScenarioBox> = req
        .tx
        .inputs
        .iter()
        .map(|i| find(&req.boxes, &i.box_id, "input"))
        .collect::<Result<_, _>>()?;
    let data_inputs: Vec<&ScenarioBox> = req
        .tx
        .data_inputs
        .iter()
        .map(|id| find(&req.boxes, id, "data input"))
        .collect::<Result<_, _>>()?;
    for (i, b) in inputs.iter().enumerate() {
        if b.ergo_tree.as_deref().unwrap_or("").is_empty() {
            return Err(SandboxError::Scenario(format!("input {i} has no ergoTree")));
        }
    }

    // A deterministic transaction id for the sandbox: the inputs, the
    // height and the outputs' bytes are what a real id commits to.
    let tx_id: [u8; 32] = {
        let mut m = Vec::new();
        for b in &inputs {
            m.extend_from_slice(b.box_id.as_deref().unwrap_or("").as_bytes());
        }
        m.extend_from_slice(&req.height.to_le_bytes());
        m.extend_from_slice(
            serde_json::to_string(&req.tx.outputs)
                .unwrap_or_default()
                .as_bytes(),
        );
        *ergo_primitives::digest::blake2b256(&m).as_bytes()
    };

    // Outputs as the chain would create them.
    let mut outputs: Vec<ScenarioBox> = req.tx.outputs.clone();
    for (i, o) in outputs.iter_mut().enumerate() {
        if o.creation_height == 0 {
            o.creation_height = req.height;
        }
        let tree_hex = o.ergo_tree.clone().unwrap_or_default();
        let tree = hex::decode(tree_hex.trim())
            .map_err(|e| SandboxError::Scenario(format!("output {i} ergoTree hex: {e}")))?;
        o.box_id = None;
        let eb = crate::box_build::build_eval_box_in("outputs", o, Some(&tree), tx_id, i as u16)?;
        o.box_id = Some(hex::encode(eb.id));
    }

    // Every input's script, in the full context.
    let mut results = Vec::with_capacity(inputs.len());
    let mut all_ok = true;
    let input_boxes_json: Vec<serde_json::Value> = inputs
        .iter()
        .map(|b| serde_json::to_value(b).unwrap_or_default())
        .collect();
    for (i, pin) in req.tx.inputs.iter().enumerate() {
        let sc_json = serde_json::json!({
            "tree": inputs[i].ergo_tree,
            "height": req.height,
            "selfIndex": i,
            "inputs": input_boxes_json,
            "outputs": outputs,
            "dataInputs": data_inputs,
            "contextVars": pin.context_vars,
            "secrets": pin.secrets,
            "parties": pin.parties,
            "network": req.network,
        });
        let sc: Scenario = serde_json::from_value(sc_json)
            .map_err(|e| SandboxError::Scenario(format!("input {i}: {e}")))?;
        let out = eval_scenario(&sc)?;
        let verdict = crate::testsuite::verdict_name(out.verdict);
        let ok = matches!(out.verdict, Verdict::Pass | Verdict::ProofAccepted);
        all_ok &= ok;
        results.push(PlayInputResult {
            box_id: pin.box_id.clone(),
            verdict,
            reduced_to: out.reduced_to,
            error: out.error.or_else(|| match out.verdict {
                Verdict::NeedsProof => {
                    Some("needs a signature: give this input the secret, or the parties".into())
                }
                Verdict::Fail => Some("the script refused this transaction".into()),
                _ => None,
            }),
            cost: out.cost,
        });
    }

    // Conservation.
    let mut problems = Vec::new();
    let erg_in: u64 = inputs.iter().map(|b| b.value.max(0) as u64).sum();
    let erg_out: u64 = outputs.iter().map(|b| b.value.max(0) as u64).sum();
    if erg_in != erg_out {
        problems.push(format!(
            "ERG not conserved: inputs {erg_in}, outputs {erg_out}"
        ));
    }
    let mut tin: BTreeMap<String, u128> = BTreeMap::new();
    let mut tout: BTreeMap<String, u128> = BTreeMap::new();
    for b in &inputs {
        for t in &b.tokens {
            *tin.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    for b in &outputs {
        for t in &b.tokens {
            *tout.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    let mint_id = inputs[0].box_id.as_deref().map(|s| s.to_lowercase());
    for (id, out_amt) in &tout {
        let in_amt = tin.get(id).copied().unwrap_or(0);
        if *out_amt > in_amt && Some(id) != mint_id.as_ref() {
            problems.push(format!(
                "token {id}: outputs carry {out_amt} but inputs only {in_amt}"
            ));
        }
    }
    let ok = all_ok && problems.is_empty();
    Ok(PlayResult {
        ok,
        tx_id: hex::encode(tx_id),
        inputs: results,
        outputs,
        problems,
        erg_in,
        erg_out,
    })
}
