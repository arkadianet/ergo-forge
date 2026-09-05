//! Transaction validation: "will this transaction validate?" — every input's
//! script run in the real context (SELF at its index, all inputs in order,
//! all outputs, data inputs, that input's extension), plus the
//! transaction-level checks the node makes before scripts: ERG and token
//! conservation (one new token may be minted with the first input's id).
//!
//! Signatures are not checked — the transaction is unsigned by design. An
//! input whose script reduces to a sigma proposition is reported as needing
//! a signature and does not invalidate the transaction.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::eval::{eval_scenario, Verdict};
use crate::scenario::{Scenario, ScenarioBox, TokenAmount, TypedValue};
use crate::testsuite::verdict_name;
use crate::SandboxError;

/// The request: a node-format unsigned transaction and the boxes it spends
/// and reads. Boxes are in the explorer/node box shape (`boxId`, `value`,
/// `ergoTree`, `assets`, `additionalRegisters`, `creationHeight`); a
/// register value may be the node's `serializedValue` hex or an object
/// carrying one.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxRequest {
    pub tx: Tx,
    /// Input and data-input boxes, by id.
    #[serde(default)]
    pub boxes: Vec<serde_json::Value>,
    /// Spending height (default: the newest input's creation height + 1).
    #[serde(default)]
    pub height: Option<u32>,
    /// `mainnet` (default) or `testnet` — address rendering only.
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tx {
    pub inputs: Vec<TxInput>,
    #[serde(default)]
    pub data_inputs: Vec<TxInput>,
    #[serde(default)]
    pub outputs: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxInput {
    pub box_id: String,
    /// Context extension: var id → serialized constant hex.
    #[serde(default)]
    pub extension: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputCheck {
    pub index: usize,
    pub box_id: String,
    /// The box's address, when the box was found.
    pub address: Option<String>,
    /// `pass` / `fail` / `error` / `needsProof` / `missing` / `invalid`.
    pub verdict: &'static str,
    pub error: Option<String>,
    pub reduced_to: Option<String>,
    pub cost: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxCheck {
    /// Every script passes or needs a signature, and the balances hold.
    pub valid: bool,
    /// Inputs whose script reduced to a sigma proposition.
    pub signatures_needed: usize,
    pub inputs: Vec<InputCheck>,
    /// Human-readable reasons the transaction would be rejected.
    pub problems: Vec<String>,
    pub erg_in: u64,
    pub erg_out: u64,
    pub height: u32,
}

/// Run the check. Errors are marshalling only (a request no check can be
/// made of); a rejected transaction is a `TxCheck` with `valid: false`.
pub fn check(req: &TxRequest) -> Result<TxCheck, SandboxError> {
    let network_name = match req.network.as_deref() {
        Some("testnet") => Some("testnet".to_string()),
        _ => None,
    };
    let net = if network_name.is_some() {
        ergo_ser::address::NetworkPrefix::Testnet
    } else {
        ergo_ser::address::NetworkPrefix::Mainnet
    };
    let by_id: BTreeMap<String, &serde_json::Value> = req
        .boxes
        .iter()
        .filter_map(|b| b["boxId"].as_str().map(|id| (id.to_lowercase(), b)))
        .collect();

    let input_boxes: Vec<Option<ScenarioBox>> = req
        .tx
        .inputs
        .iter()
        .map(|i| by_id.get(&i.box_id.to_lowercase()).map(|b| scenario_box(b)))
        .collect();
    let data_boxes: Vec<Option<ScenarioBox>> = req
        .tx
        .data_inputs
        .iter()
        .map(|i| by_id.get(&i.box_id.to_lowercase()).map(|b| scenario_box(b)))
        .collect();
    let outputs: Vec<ScenarioBox> = req.tx.outputs.iter().map(scenario_box).collect();

    let height = req.height.unwrap_or_else(|| {
        input_boxes
            .iter()
            .flatten()
            .map(|b| b.creation_height)
            .max()
            .unwrap_or(0)
            + 1
    });

    let mut problems = Vec::new();
    let mut inputs = Vec::new();
    let mut signatures_needed = 0;

    // Script checks need every input and data input present.
    let all_present =
        input_boxes.iter().all(Option::is_some) && data_boxes.iter().all(Option::is_some);
    for (i, tx_in) in req.tx.inputs.iter().enumerate() {
        let Some(self_box) = &input_boxes[i] else {
            problems.push(format!("input {i}: box {} was not provided", tx_in.box_id));
            inputs.push(InputCheck {
                index: i,
                box_id: tx_in.box_id.clone(),
                address: None,
                verdict: "missing",
                error: None,
                reduced_to: None,
                cost: 0,
            });
            continue;
        };
        let tree_hex = self_box.ergo_tree.clone().unwrap_or_default();
        let address = hex::decode(&tree_hex)
            .ok()
            .and_then(|b| ergo_ser::address::encode_address_from_tree_bytes(net, &b).ok());
        if !all_present {
            inputs.push(InputCheck {
                index: i,
                box_id: tx_in.box_id.clone(),
                address,
                verdict: "invalid",
                error: Some("not run: another input or data input is missing".into()),
                reduced_to: None,
                cost: 0,
            });
            continue;
        }
        let mut context_vars = BTreeMap::new();
        for (k, hex_val) in &tx_in.extension {
            if let Ok(id) = k.parse::<u8>() {
                context_vars.insert(
                    id,
                    TypedValue {
                        r#type: "raw".into(),
                        value: serde_json::Value::String(hex_val.clone()),
                    },
                );
            }
        }
        let sc = Scenario {
            params: Default::default(),
            headers: Vec::new(),
            secrets: Vec::new(),
            parties: Vec::new(),
            avl: Default::default(),
            tree: Some(tree_hex),
            source: None,
            tree_version: 0,
            network: network_name.clone(),
            height,
            self_box: None,
            self_index: Some(i),
            inputs: input_boxes.iter().flatten().cloned().collect(),
            outputs: outputs.clone(),
            data_inputs: data_boxes.iter().flatten().cloned().collect(),
            context_vars,
            miner_pubkey: None,
            pre_header: None,
            cost_limit: None,
            activated_script_version: None,
            proof: None,
            message: None,
        };
        let check = match eval_scenario(&sc) {
            Ok(o) => {
                match o.verdict {
                    Verdict::Pass => {}
                    Verdict::NeedsProof => signatures_needed += 1,
                    Verdict::Fail => problems.push(format!("input {i}: script evaluates to false")),
                    Verdict::Error => problems.push(format!(
                        "input {i}: script threw: {}",
                        o.error.clone().unwrap_or_default()
                    )),
                    _ => {}
                }
                InputCheck {
                    index: i,
                    box_id: tx_in.box_id.clone(),
                    address,
                    verdict: verdict_name(o.verdict),
                    error: o.error,
                    reduced_to: o.reduced_to,
                    cost: o.cost,
                }
            }
            Err(e) => {
                problems.push(format!("input {i}: {e}"));
                InputCheck {
                    index: i,
                    box_id: tx_in.box_id.clone(),
                    address,
                    verdict: "invalid",
                    error: Some(e.to_string()),
                    reduced_to: None,
                    cost: 0,
                }
            }
        };
        inputs.push(check);
    }
    for (k, d) in data_boxes.iter().enumerate() {
        if d.is_none() {
            problems.push(format!(
                "data input {k}: box {} was not provided",
                req.tx.data_inputs[k].box_id
            ));
        }
    }

    // Conservation: ERG exactly; tokens exactly, except one new id equal to
    // the first input's box id (minting) that may appear on the output side.
    let erg_in: u64 = input_boxes
        .iter()
        .flatten()
        .map(|b| b.value.max(0) as u64)
        .sum();
    let erg_out: u64 = outputs.iter().map(|b| b.value.max(0) as u64).sum();
    if all_present && erg_in != erg_out {
        problems.push(format!(
            "ERG not conserved: inputs {erg_in}, outputs {erg_out}"
        ));
    }
    let mut tin: BTreeMap<String, u128> = BTreeMap::new();
    let mut tout: BTreeMap<String, u128> = BTreeMap::new();
    for b in input_boxes.iter().flatten() {
        for t in &b.tokens {
            *tin.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    for b in &outputs {
        for t in &b.tokens {
            *tout.entry(t.id.to_lowercase()).or_default() += t.amount as u128;
        }
    }
    let mint_id = req.tx.inputs.first().map(|i| i.box_id.to_lowercase());
    if all_present {
        for (id, out_amt) in &tout {
            let in_amt = tin.get(id).copied().unwrap_or(0);
            if *out_amt > in_amt && Some(id) != mint_id.as_ref() {
                problems.push(format!(
                    "token {id}: outputs carry {out_amt} but inputs only {in_amt}"
                ));
            }
        }
    }

    let valid = problems.is_empty();
    Ok(TxCheck {
        valid,
        signatures_needed,
        inputs,
        problems,
        erg_in,
        erg_out,
        height,
    })
}

/// Node/explorer box JSON → a scenario box with raw registers.
fn scenario_box(b: &serde_json::Value) -> ScenarioBox {
    let mut registers = BTreeMap::new();
    if let Some(regs) = b["additionalRegisters"].as_object() {
        for (k, v) in regs {
            let raw = v
                .as_str()
                .or_else(|| v["serializedValue"].as_str())
                .map(str::to_string);
            if let Some(hex_val) = raw {
                registers.insert(
                    k.clone(),
                    TypedValue {
                        r#type: "raw".into(),
                        value: serde_json::Value::String(hex_val),
                    },
                );
            }
        }
    }
    let tokens = b["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    Some(TokenAmount {
                        id: t["tokenId"].as_str()?.to_string(),
                        amount: t["amount"].as_u64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    ScenarioBox {
        value: b["value"].as_i64().unwrap_or(0),
        ergo_tree: b["ergoTree"].as_str().map(str::to_string),
        tokens,
        creation_height: b["creationHeight"].as_u64().unwrap_or(0) as u32,
        registers,
        box_id: b["boxId"].as_str().map(str::to_string),
    }
}
