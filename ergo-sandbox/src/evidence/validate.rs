//! Full node transaction validation against explicit, supplied premises.
//! No chain membership, property-violation, or future-inclusion claim is made.
use std::collections::HashMap;

use ergo_primitives::{digest::Digest32, reader::VlqReader, writer::VlqWriter};
use ergo_ser::{
    ergo_box::ErgoBox,
    header::{read_header, write_header},
};
use ergo_validation::{
    context::{LocalPolicy, ProtocolParams, TransactionContext, UtxoView},
    cost::{CostAccumulator, JitCost},
    error::ValidationError,
    tx::{
        reemission::ReemissionRuleInputs, validate_transaction, CheckedTransaction,
        TxValidationCtx, TxValidationRules,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{wire::WireBox, EvidenceCase, Origin, Premise};

/// Explicit snapshot, with no mainnet-default fallback. These fields map
/// directly to the node's ProtocolParams; no consensus predicates live here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameters {
    pub min_value_per_byte: u64,
    pub max_block_cost: u64,
    pub max_block_size: u32,
    pub max_box_size: u32,
    pub max_tokens_per_box: u8,
    pub input_cost: u64,
    pub data_input_cost: u64,
    pub output_cost: u64,
    pub token_access_cost: u64,
    pub storage_fee_factor: i32,
    pub storage_period: u32,
}
impl Parameters {
    pub fn node(&self) -> ProtocolParams {
        ProtocolParams {
            min_value_per_byte: self.min_value_per_byte,
            max_block_cost: self.max_block_cost,
            max_block_size: self.max_block_size,
            max_box_size: self.max_box_size,
            max_tokens_per_box: self.max_tokens_per_box,
            input_cost: self.input_cost,
            data_input_cost: self.data_input_cost,
            output_cost: self.output_cost,
            token_access_cost: self.token_access_cost,
            storage_fee_factor: self.storage_fee_factor,
            storage_period: self.storage_period,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockContext {
    pub height: u32,
    pub miner_pubkey: String,
    pub pre_header_timestamp: u64,
    pub activated_script_version: u8,
    pub pre_header_version: u8,
    pub pre_header_parent_id: String,
    pub pre_header_n_bits: u64,
    pub pre_header_votes: [u8; 3],
}
impl BlockContext {
    pub fn node(&self) -> Result<TransactionContext, String> {
        Ok(TransactionContext {
            height: self.height,
            miner_pubkey: array(&self.miner_pubkey)?,
            pre_header_timestamp: self.pre_header_timestamp,
            activated_script_version: self.activated_script_version,
            pre_header_version: self.pre_header_version,
            pre_header_parent_id: array(&self.pre_header_parent_id)?,
            pre_header_n_bits: self.pre_header_n_bits,
            pre_header_votes: self.pre_header_votes,
        })
    }
}

/// A supplied rule configuration, not an assertion that a named network or
/// historical height used it. Disabled must be explicit, with a reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum NetworkRules {
    Disabled {
        description: String,
        reason: String,
    },
    Enabled {
        description: String,
        activation_height: u32,
        reemission_token_id: String,
        pay_to_reemission_tree: String,
    },
}
impl NetworkRules {
    pub fn node(&self) -> Result<Option<ReemissionRuleInputs>, String> {
        match self {
            Self::Disabled {
                description,
                reason,
            } if !description.is_empty() && !reason.is_empty() => Ok(None),
            Self::Enabled {
                description,
                activation_height,
                reemission_token_id,
                pay_to_reemission_tree,
            } if !description.is_empty() => {
                let tree = hex::decode(pay_to_reemission_tree).map_err(err)?;
                // Check the actual supplied tree through node codec APIs only.
                let mut reader = VlqReader::new(&tree);
                let parsed = ergo_ser::ergo_tree::read_ergo_tree(&mut reader).map_err(err)?;
                if !reader.is_empty() {
                    return Err("trailing network-rule tree bytes".into());
                }
                ergo_ser::ergo_tree::check_tree_version_supported(&parsed).map_err(err)?;
                ergo_ser::ergo_tree::check_header_size_bit(&parsed).map_err(err)?;
                ergo_ser::ergo_tree::check_resolvable_methods(&parsed).map_err(err)?;
                ergo_ser::ergo_tree::check_sigma_prop_root(&parsed).map_err(err)?;
                Ok(Some(ReemissionRuleInputs {
                    activation_height: *activation_height,
                    reemission_token_id: array(reemission_token_id)?,
                    pay_to_reemission_tree: tree,
                }))
            }
            _ => Err("network rules require an explicit description and disabled reason".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub max_transaction_size: usize,
}

/// Only input requests deserialize. Every optional premise is explicitly
/// missing or present; absent fields never acquire defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationRequest {
    pub format_version: u32,
    pub case: EvidenceCase,
    pub transaction_bytes: String,
    pub parameters: Premise<Parameters>,
    pub network_rules: Premise<NetworkRules>,
    pub block_context: Premise<BlockContext>,
    pub headers: Premise<Vec<String>>,
    pub local_policy: Premise<Policy>,
    /// Block-cost units already spent by preceding transactions. Always explicit.
    pub prior_block_cost: Premise<u64>,
}
impl ValidationRequest {
    pub fn fingerprint(&self) -> String {
        super::case::json_digest(&serde_json::to_value(self).expect("request serializes"))
    }
}

/// A capability produced only by a successful fresh node call. Reports are
/// inspectable JSON, never an importable substitute for this value.
///
/// ```compile_fail
/// use ergo_sandbox::evidence::validate::AcceptedExecution;
/// let _: AcceptedExecution = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use ergo_sandbox::evidence::validate::AcceptedExecution;
/// let forged = AcceptedExecution {};
/// ```
#[derive(Debug)]
pub struct AcceptedExecution {
    checked: CheckedTransaction,
    request: ValidationRequest,
    total_block_cost: u64,
}
impl AcceptedExecution {
    pub fn checked(&self) -> &CheckedTransaction {
        &self.checked
    }
    pub fn request(&self) -> &ValidationRequest {
        &self.request
    }
    pub fn report(&self) -> Value {
        json!({"formatVersion":1,"status":"node-accepted","stage":"complete","nodeValidated":true,"pipelineInvoked":true,
            "method":"ergo_validation::tx::validate_transaction","scope":"supplied-state-and-rules-only; no historical-state, property-violation or inclusion claim",
            "nodeRevision":node_revision(),"transactionId":hex::encode(self.checked.tx_id()),
            "requestFingerprint":self.request.fingerprint(),"request":self.request,"totalBlockCost":self.total_block_cost})
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationFailure {
    pub status: &'static str,
    pub stage: &'static str,
    pub node_validated: bool,
    pub pipeline_invoked: bool,
    pub detail: String,
    pub request_fingerprint: String,
    pub request: Box<ValidationRequest>,
}
impl ValidationFailure {
    fn input(request: &ValidationRequest, detail: String) -> Self {
        Self {
            status: "incomplete-or-invalid-premises",
            stage: "premises",
            node_validated: false,
            pipeline_invoked: false,
            detail,
            request_fingerprint: request.fingerprint(),
            request: Box::new(request.clone()),
        }
    }
}

struct Snapshot(HashMap<Digest32, ErgoBox>);
impl UtxoView for Snapshot {
    fn get_box(&self, id: &Digest32) -> Option<ErgoBox> {
        self.0.get(id).cloned()
    }
}

/// No preflight oracle, parsed shortcut, or caller-supplied acceptance flag.
pub fn validate(request: &ValidationRequest) -> Result<AcceptedExecution, ValidationFailure> {
    let input = |e| ValidationFailure::input(request, e);
    if request.format_version != 1
        || request.case.premises().engine_revision != node_revision()
        || node_revision() != super::case::engine_revision()
    {
        return Err(input("unsupported request format or node revision".into()));
    }
    if request.case.premises().context.value().is_some()
        && serde_json::to_value(&request.case.premises().context).map_err(|e| input(err(e)))?
            != serde_json::to_value(&request.block_context).map_err(|e| input(err(e)))?
    {
        return Err(input(
            "case context disagrees with the typed block context; resolve explicitly".into(),
        ));
    }
    let params = required(&request.parameters, "parameters")
        .map_err(input)?
        .node();
    let context = required(&request.block_context, "block context/activation")
        .map_err(input)?
        .node()
        .map_err(input)?;
    let rules = required(&request.network_rules, "network rules")
        .map_err(input)?
        .node()
        .map_err(input)?;
    let policy = required(&request.local_policy, "local policy").map_err(input)?;
    let prior = *required(&request.prior_block_cost, "prior block cost").map_err(input)?;
    let header_hex = required(&request.headers, "headers").map_err(input)?;
    let mut headers = Vec::new();
    for hex in header_hex {
        let bytes = hex::decode(hex).map_err(|e| input(err(e)))?;
        let mut r = VlqReader::new(&bytes);
        let h = read_header(&mut r).map_err(|e| input(err(e)))?;
        let mut w = VlqWriter::new();
        write_header(&mut w, &h).map_err(|e| input(err(e)))?;
        if !r.is_empty() || w.result() != bytes {
            return Err(input(
                "header must round-trip without trailing or rewritten bytes".into(),
            ));
        }
        headers.push(h);
    }
    let boxes = required(&request.case.premises().boxes, "UTXO snapshot").map_err(input)?;
    let mut state = Snapshot(HashMap::new());
    for record in boxes {
        let b = WireBox::from_record(record.clone()).map_err(input)?;
        let id = b.node().box_id().map_err(|e| input(err(e)))?;
        if state.0.insert(id, b.node().clone()).is_some() {
            return Err(input("duplicate UTXO snapshot member".into()));
        }
    }
    let bytes = hex::decode(&request.transaction_bytes).map_err(|e| input(err(e)))?;
    if let Some(p) = request.case.premises().assumptions.get("wireTransaction") {
        let material = required(p, "case wire transaction").map_err(input)?;
        let recorded = material
            .get("bytes")
            .and_then(Value::as_str)
            .ok_or_else(|| input("case wire transaction lacks bytes".into()))?;
        if hex::decode(recorded).map_err(|e| input(err(e)))? != bytes {
            return Err(input(
                "request disagrees with case transaction bytes".into(),
            ));
        }
    }
    let limit = JitCost::from_block_cost(params.max_block_cost).map_err(|e| input(err(e)))?;
    let mut cost = CostAccumulator::new(limit);
    cost.add(JitCost::from_block_cost(prior).map_err(|e| input(err(e)))?)
        .map_err(|e| input(err(e)))?;
    let mut cx = TxValidationCtx {
        ctx: &context,
        params: &params,
        cost: &mut cost,
        last_headers: &headers,
        rules: TxValidationRules {
            reemission: rules.as_ref(),
        },
    };
    match validate_transaction(
        &bytes,
        &state,
        &LocalPolicy {
            max_transaction_size: policy.max_transaction_size,
        },
        &mut cx,
    ) {
        Ok(checked) => Ok(AcceptedExecution {
            checked,
            request: request.clone(),
            total_block_cost: cost.total_block_cost(),
        }),
        Err(e) => Err(ValidationFailure {
            status: "node-rejected",
            stage: stage(&e),
            node_validated: false,
            pipeline_invoked: true,
            detail: format!("{e:?}: {e}"),
            request_fingerprint: request.fingerprint(),
            request: Box::new(request.clone()),
        }),
    }
}

/// Diagnostic classification only; the node determines the verdict and order.
pub fn stage(e: &ValidationError) -> &'static str {
    match e {
        ValidationError::Deserialization(_) => "deserialization-or-local-policy",
        ValidationError::NonCanonical => "canonical-encoding",
        ValidationError::DuplicateInput { .. }
        | ValidationError::OutputValueTooLow { .. }
        | ValidationError::BoxTooLarge { .. } => "structural",
        ValidationError::OutputFromFuture { .. }
        | ValidationError::OutputCreationHeightBelowInputs { .. } => "output-heights",
        ValidationError::InputBoxNotFound { .. } | ValidationError::DataInputBoxNotFound { .. } => {
            "utxo-resolution"
        }
        ValidationError::CostExceeded { .. } => "aggregate-cost",
        _ => "node-validation",
    }
}
fn required<'a, T>(p: &'a Premise<T>, name: &str) -> Result<&'a T, String> {
    match p {
        Premise::Present { value, origin } if *origin != Origin::IndependentlyChecked => Ok(value),
        Premise::Missing { reason } => Err(format!("missing {name}: {reason}")),
        _ => Err(format!(
            "{name} claims independent provenance unsupported by this adapter"
        )),
    }
}
fn array<const N: usize>(hex: &str) -> Result<[u8; N], String> {
    hex::decode(hex)
        .map_err(err)?
        .try_into()
        .map_err(|_| format!("expected {N} bytes"))
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// The validator dependency's own pin, checked against the compiler/case pin.
pub fn node_revision() -> &'static str {
    include_str!("../../../Cargo.toml")
        .lines()
        .find(|line| line.starts_with("ergo-validation "))
        .and_then(|line| line.split("rev").nth(1))
        .and_then(|line| line.split('"').nth(1))
        .expect("workspace pins ergo-validation")
}
