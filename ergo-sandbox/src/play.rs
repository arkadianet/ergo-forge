//! Play: the one operation of a sandbox chain — apply a transaction to a
//! set of boxes. Every input's script is evaluated in the full transaction
//! context (`selfIndex`, all inputs, outputs, data inputs, that input's
//! context variables and secrets), ERG and tokens must balance (one new
//! token named after the first input may be minted), and the outputs come
//! back with deterministic simulation IDs. Scenario proofs use a supplied/default
//! message, not canonical transaction signing bytes. Full node validation has
//! not run. The state itself lives
//! with the caller: the request carries the boxes, the response the new
//! ones. Nothing here touches a network.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::prove::{PartySpec, SecretSpec};
use crate::scenario::{ScenarioBox, TypedValue};
use crate::{eval_scenario, SandboxError, Scenario, Verdict};

/// A drafted transaction. `Serialize` is for the experiment record only: a
/// draft is hashed into a replay fingerprint, never echoed into a result body
/// with its secrets in it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayRequest {
    pub height: u32,
    /// The unspent boxes the transaction may use, each with its `boxId`.
    pub boxes: Vec<ScenarioBox>,
    pub tx: PlayTx,
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayTx {
    pub inputs: Vec<PlayInput>,
    #[serde(default)]
    pub data_inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<ScenarioBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
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

/// Prepared context shared with Play export. In particular, outputs already
/// carry Play's deterministic IDs and default creation heights.
pub(crate) struct PreparedPlay<'a> {
    inputs: Vec<&'a ScenarioBox>,
    data_inputs: Vec<&'a ScenarioBox>,
    outputs: Vec<ScenarioBox>,
    tx_id: [u8; 32],
}

pub(crate) fn prepare(req: &PlayRequest) -> Result<PreparedPlay<'_>, SandboxError> {
    if req.tx.inputs.is_empty() {
        return Err(SandboxError::Scenario(
            "a transaction needs at least one input".into(),
        ));
    }
    {
        let mut seen = std::collections::BTreeSet::new();
        for i in &req.tx.inputs {
            if !seen.insert(i.box_id.to_lowercase()) {
                return Err(SandboxError::Scenario(format!(
                    "input {} is listed twice",
                    i.box_id
                )));
            }
        }
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

    // A deterministic simulation ID, not the node's canonical transaction ID.
    // Preserve this legacy encoding so saved Play chains remain reproducible.
    let tx_id: [u8; 32] = {
        let mut m = Vec::new();
        for b in &inputs {
            m.extend_from_slice(b.box_id.as_deref().unwrap_or("").as_bytes());
        }
        m.extend_from_slice(&req.height.to_le_bytes());
        for id in &req.tx.data_inputs {
            m.extend_from_slice(id.to_lowercase().as_bytes());
        }
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
        if eb.raw_bytes.is_empty() {
            return Err(SandboxError::Scenario(format!(
                "output {i}: ergoTree does not parse as an ErgoTree"
            )));
        }
        o.box_id = Some(hex::encode(eb.id));
    }

    Ok(PreparedPlay {
        inputs,
        data_inputs,
        outputs,
        tx_id,
    })
}

impl PreparedPlay<'_> {
    pub(crate) fn scenario(
        &self,
        req: &PlayRequest,
        index: usize,
    ) -> Result<Scenario, SandboxError> {
        let pin =
            req.tx.inputs.get(index).ok_or_else(|| {
                SandboxError::Scenario(format!("inputIndex {index} is out of range"))
            })?;
        serde_json::from_value(serde_json::json!({
            "tree": self.inputs[index].ergo_tree,
            "height": req.height,
            "selfIndex": index,
            "inputs": self.inputs,
            "outputs": self.outputs,
            "dataInputs": self.data_inputs,
            "contextVars": pin.context_vars,
            "secrets": pin.secrets,
            "parties": pin.parties,
            "network": req.network,
        }))
        .map_err(|e| SandboxError::Scenario(format!("input {index}: {e}")))
    }
}

/// Apply `req.tx` to `req.boxes` at `req.height`.
pub fn apply(req: &PlayRequest) -> Result<PlayResult, SandboxError> {
    let prepared = prepare(req)?;
    let inputs = &prepared.inputs;
    let outputs = &prepared.outputs;
    let tx_id = prepared.tx_id;
    // Every input's script, in the full context.
    let mut results = Vec::with_capacity(inputs.len());
    let mut all_ok = true;
    for (i, pin) in req.tx.inputs.iter().enumerate() {
        let sc = prepared.scenario(req, i)?;
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
    let erg_in_total: u128 = inputs.iter().map(|b| b.value.max(0) as u128).sum();
    let erg_out_total: u128 = outputs.iter().map(|b| b.value.max(0) as u128).sum();
    if erg_in_total != erg_out_total {
        problems.push(format!(
            "ERG not conserved: inputs {erg_in_total}, outputs {erg_out_total}"
        ));
    }
    let erg_in = erg_in_total.min(u64::MAX as u128) as u64;
    let erg_out = erg_out_total.min(u64::MAX as u128) as u64;
    let mut tin: BTreeMap<String, u128> = BTreeMap::new();
    let mut tout: BTreeMap<String, u128> = BTreeMap::new();
    for b in inputs {
        for t in &b.tokens {
            *tin.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    for b in outputs {
        for t in &b.tokens {
            *tout.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    // A mint is a NEW id (the first input's box id) that no input carries.
    let mint_id = inputs[0]
        .box_id
        .as_deref()
        .map(|s| s.to_lowercase())
        .filter(|id| !tin.contains_key(id));
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
        claim: crate::claim::ClaimMetadata::SIMULATION,
        ok,
        tx_id: hex::encode(tx_id),
        inputs: results,
        outputs: prepared.outputs,
        problems,
        erg_in,
        erg_out,
    })
}

// ── Carrying a step forward ─────────────────────────────────────────────────

/// What a following step can be drafted from, and what the box cap cost.
pub(crate) struct NextStep {
    /// The draft: the carried boxes, the transaction that spends them, and the
    /// height one above the step that produced them.
    pub request: PlayRequest,
    /// Boxes available before the cap.
    pub available: usize,
    /// Boxes carried after the cap.
    pub carried: usize,
}

/// The draft for the step *after* an applied transaction: the boxes that
/// transaction produced, plus the boxes it left unspent, capped at `max_boxes`
/// and spent together. Each carried box is recreated as an output with the
/// same script, value, tokens and registers, so the next step runs against the
/// same contract shape rather than a synthetic `sigmaProp(true)` sink.
///
/// This is what makes a multi-step experiment possible: the next step's script
/// runs on boxes the previous step produced, so a sequence can work a position
/// over several moves. Recreated outputs receive fresh Play ids and heights;
/// they are still the same declared contract shape, not a new protocol state.
///
/// `None` when the transaction left nothing spendable: it produced no output,
/// or every box it left has no id to spend by. The cap keeps outputs first and
/// then the remaining unspent boxes in their declared order; a data input the
/// cap dropped is dropped from the transaction too rather than left dangling.
/// A carried input brings no secrets and no context variables: the next step is
/// the spender's own transaction, and a script that needed a key will ask for
/// one again over there.
pub(crate) fn next_step_draft(
    req: &PlayRequest,
    result: &PlayResult,
    max_boxes: usize,
) -> Result<Option<NextStep>, SandboxError> {
    if result.outputs.is_empty() {
        return Ok(None);
    }
    let spent: BTreeSet<String> = req
        .tx
        .inputs
        .iter()
        .map(|i| i.box_id.to_lowercase())
        .collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut boxes: Vec<ScenarioBox> = Vec::new();
    let mut available = 0usize;
    // Outputs lead so the cap keeps the material this step just created.
    for o in &result.outputs {
        if let Some(id) = o.box_id.as_deref() {
            if seen.insert(id.to_lowercase()) {
                available += 1;
                boxes.push(o.clone());
            }
        }
    }
    for b in req.boxes.iter().filter(|b| {
        b.box_id
            .as_deref()
            .is_some_and(|id| !spent.contains(&id.to_lowercase()))
    }) {
        if let Some(id) = b.box_id.as_deref() {
            if seen.insert(id.to_lowercase()) {
                available += 1;
                boxes.push(b.clone());
            }
        }
    }
    boxes.truncate(max_boxes.max(1));
    if boxes.is_empty() {
        return Ok(None);
    }
    let carried = boxes.len();

    // Recreate each carried box as an output with a fresh identity. This keeps
    // the next step's scripts and reserves meaningful while conserving value
    // and tokens exactly, without minting a universal pass-through box.
    let outputs: Vec<ScenarioBox> = boxes
        .iter()
        .map(|b| {
            let mut output = b.clone();
            output.box_id = None;
            output.creation_height = 0;
            output
        })
        .collect();

    let carried_ids: BTreeSet<String> = boxes
        .iter()
        .map(|b| b.box_id.clone().unwrap_or_default().to_lowercase())
        .collect();
    let inputs: Vec<PlayInput> = boxes
        .iter()
        .map(|b| PlayInput {
            box_id: b.box_id.clone().unwrap_or_default(),
            context_vars: Default::default(),
            secrets: Vec::new(),
            parties: Vec::new(),
        })
        .collect();
    let data_inputs: Vec<String> = req
        .tx
        .data_inputs
        .iter()
        .filter(|id| carried_ids.contains(&id.to_lowercase()))
        .cloned()
        .collect();
    Ok(Some(NextStep {
        request: PlayRequest {
            height: req.height.saturating_add(1),
            boxes,
            tx: PlayTx {
                inputs,
                data_inputs,
                outputs,
            },
            network: req.network.clone(),
        },
        available,
        carried,
    }))
}
