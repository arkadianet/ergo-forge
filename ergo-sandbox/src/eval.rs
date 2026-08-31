//! Scenario evaluation: assemble an owned evaluation context from scenario
//! JSON, run the consensus reduce path (`reduce_expr_traced_with_cost`, or
//! `verify_spending_proof_with_context_and_cost` when a proof is supplied),
//! and render the outcome.
//!
//! The context assembly mirrors
//! `ergo-validation/src/tx/script/mod.rs::validate_scripts` field-for-field;
//! the difference is only where the inputs come from (JSON, not wire boxes).

use ergo_primitives::reader::VlqReader;
use ergo_ser::address::{encode_p2s, NetworkPrefix};
use ergo_ser::ergo_tree::read_ergo_tree;
use ergo_sigma::evaluator::{reduce_expr_traced_with_cost, EvalBox, ReductionContext, TraceEntry};
use ergo_sigma::reduce::verify_spending_proof_with_context_and_cost;
use serde::Serialize;

use crate::box_build::build_eval_box;
use crate::scenario::{Scenario, ScenarioBox};
use crate::{compile, inspect, SandboxError};

/// Default cost budget: the consensus `max_block_cost` (default voted param,
/// `ergo-validation/src/context.rs:47`; matches the tooling-API design §3.4).
pub const DEFAULT_COST_LIMIT: u64 = 8_001_091;

/// The decided outcome of a sandbox evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    /// Script reduced to `TrivialProp(true)`.
    Pass,
    /// Script reduced to `TrivialProp(false)`.
    Fail,
    /// Script raised a runtime exception (failed `Option.get`, out-of-range
    /// index, cost exceeded, …) — see [`EvalOutcome::error`].
    Error,
    /// Script reduced to a sigma proposition that needs a spending proof;
    /// see [`EvalOutcome::reduced_to`].
    NeedsProof,
    /// A supplied spending proof verified against the proposition.
    ProofAccepted,
    /// A supplied spending proof failed verification.
    ProofRejected,
}

/// One evaluator trace step (a `val` binding, an `If` condition, or a
/// sigma-protocol child result).
#[derive(Debug, Clone, Serialize)]
pub struct TraceLine {
    /// What was evaluated.
    pub label: String,
    /// Rendered intermediate value.
    pub value: String,
}

/// One cost-charging step (feature `cost-trace`).
#[derive(Debug, Clone, Serialize)]
pub struct CostLine {
    /// Cost source (opcode name, `Crypto:N`, …).
    pub label: String,
    /// Cost added at this step (JitCost units).
    pub delta: u64,
    /// Running total after this step.
    pub total: u64,
}

/// The full outcome of one scenario evaluation.
#[derive(Debug, Clone, Serialize)]
pub struct EvalOutcome {
    /// The decided verdict.
    pub verdict: Verdict,
    /// Runtime evaluation error text, when the script raised one.
    pub error: Option<String>,
    /// Block-cost units consumed (JIT model, as the node accounts them).
    pub cost: u64,
    /// The budget [`EvalOutcome::cost`] was charged against.
    pub cost_limit: u64,
    /// Pretty rendering of the reduced `SigmaBoolean` (when reduction
    /// succeeded) — e.g. `AND(ProveDlog(0274…), GE(HEIGHT, 1000))`.
    pub reduced_to: Option<String>,
    /// Evaluator trace (bindings, branches, sigma children).
    pub trace: Vec<TraceLine>,
    /// Per-step cost breakdown (requires the `cost-trace` feature).
    #[cfg(feature = "cost-trace")]
    pub cost_breakdown: Vec<CostLine>,
    /// The evaluated tree's canonical bytes, hex (echo for the UI).
    pub tree_hex: String,
    /// Pay-to-script address of the evaluated tree.
    pub p2s_address: String,
}

/// Evaluate a scenario end to end.
///
/// Error returns mean marshalling failed (bad hex, uncompilable source,
/// malformed scenario) — a script that *ran and failed* is a normal
/// [`EvalOutcome`] with [`Verdict::Error`] or [`Verdict::Fail`].
pub fn eval_scenario(sc: &Scenario) -> Result<EvalOutcome, SandboxError> {
    // 1. Resolve the tree under evaluation + the address network.
    let network = parse_network(sc.network.as_deref())?;
    let tree_bytes: Vec<u8> = match (&sc.tree, &sc.source) {
        (Some(_), Some(_)) => {
            return Err(SandboxError::Scenario(
                "supply either `tree` or `source`, not both".into(),
            ))
        }
        (None, None) => {
            return Err(SandboxError::Scenario(
                "supply a `tree` (hex) or `source` (ErgoScript)".into(),
            ))
        }
        (Some(hex_str), None) => {
            hex::decode(hex_str.trim()).map_err(|source| SandboxError::Hex {
                field: "tree",
                source,
            })?
        }
        (None, Some(src)) => {
            compile::compile_source(
                src,
                sc.tree_version,
                network.unwrap_or(NetworkPrefix::Mainnet),
            )?
            .tree_bytes
        }
    };

    let mut r = VlqReader::new(&tree_bytes);
    let tree = read_ergo_tree(&mut r).map_err(|e| SandboxError::Tree(e.to_string()))?;

    // 2. Box collections. SELF is ALWAYS INPUTS(0) — the invariant
    // `CONTEXT.INPUTS(0) == SELF` relies on. An omitted `inputs` list
    // defaults to `[self]`; an explicit `inputs` list is a proposal for
    // INPUTS(1..), prepended with the self box. To override the spent box's
    // other fields, use `selfBox`.
    let default_self = ScenarioBox::default();
    let self_box = build_eval_box(
        "selfBox",
        sc.self_box.as_ref().unwrap_or(&default_self),
        Some(&tree_bytes),
    )?;
    let mut inputs: Vec<EvalBox> = Vec::with_capacity(sc.inputs.len() + 1);
    inputs.push(self_box.clone());
    for (i, b) in sc.inputs.iter().enumerate() {
        inputs.push(build_eval_box("inputs", b, None).map_err(index_err(i))?);
    }
    let outputs: Vec<EvalBox> = sc
        .outputs
        .iter()
        .map(|b| build_eval_box("outputs", b, None))
        .collect::<Result<_, _>>()?;
    let data_inputs: Vec<EvalBox> = sc
        .data_inputs
        .iter()
        .map(|b| build_eval_box("dataInputs", b, None))
        .collect::<Result<_, _>>()?;

    // 3. Context variables (BTreeMap iteration is sorted by var id, so the
    // IndexMap insertion order is deterministic).
    let extension = sc
        .context_vars
        .iter()
        .map(|(id, tv)| {
            let (tpe, value) = crate::scenario::parse_typed_value(&tv.r#type, &tv.value)?;
            Ok((*id, (tpe, value)))
        })
        .collect::<Result<indexmap::IndexMap<u8, (_, _)>, SandboxError>>()?;
    let input_extensions = vec![indexmap::IndexMap::new(); inputs.len()];

    // 4. Pre-header + miner key.
    let miner_pubkey: [u8; 33] = match &sc.miner_pubkey {
        Some(s) => hex::decode(s.trim())
            .map_err(|source| SandboxError::Hex {
                field: "minerPubkey",
                source,
            })?
            .try_into()
            .map_err(|v: Vec<u8>| {
                SandboxError::Scenario(format!("`minerPubkey` must be 33 bytes, got {}", v.len()))
            })?,
        None => [0u8; 33],
    };
    let ph = sc.pre_header.clone().unwrap_or_default();
    let pre_header_parent_id: [u8; 32] = match &ph.parent_id {
        Some(s) => hex::decode(s.trim())
            .map_err(|source| SandboxError::Hex {
                field: "preHeader.parentId",
                source,
            })?
            .try_into()
            .map_err(|v: Vec<u8>| {
                SandboxError::Scenario(format!(
                    "`preHeader.parentId` must be 32 bytes, got {}",
                    v.len()
                ))
            })?,
        None => [0u8; 32],
    };

    // 5. The reduction context — same shape validate_scripts assembles.
    let ctx = ReductionContext {
        height: sc.height,
        self_box: Some(&self_box),
        self_creation_height: self_box.creation_height,
        outputs: &outputs,
        inputs: &inputs,
        data_inputs: &data_inputs,
        miner_pubkey,
        pre_header_timestamp: ph.timestamp.unwrap_or(0),
        pre_header_version: ph.version.unwrap_or(0),
        pre_header_parent_id,
        pre_header_n_bits: ph.n_bits.unwrap_or(0),
        pre_header_votes: ph.votes.unwrap_or([0u8; 3]),
        extension,
        input_extensions: &input_extensions,
        last_headers: &[],
        last_block_utxo_root: None,
        activated_script_version: sc.activated_script_version.unwrap_or(3),
        ergo_tree_version: tree.version,
    };

    // 6. Bounded cost budget (never `recording_only()` — this is a
    // code-execution surface). The diagnostic reduction and the proof
    // verification each get their OWN budget-fresh accumulator: the verify
    // path re-evaluates the tree internally, and `CostAccumulator` is
    // additive — sharing one accumulator would double-charge the preview
    // pass and could reject a spend that fits the budget on a single
    // evaluation (what the consensus path actually charges).
    let limit = sc.cost_limit.unwrap_or(DEFAULT_COST_LIMIT);
    let new_accumulator = || -> Result<ergo_primitives::cost::CostAccumulator, SandboxError> {
        Ok(ergo_primitives::cost::CostAccumulator::new(
            ergo_primitives::cost::JitCost::from_block_cost(limit)
                .map_err(|_| SandboxError::CostLimit(limit))?,
        ))
    };
    let mut cost = new_accumulator()?;

    // 7. Diagnostic reduction, with the semantic trace. With the
    // `cost-trace` feature, capture the per-step cost breakdown from the
    // global recorder.
    #[cfg(feature = "cost-trace")]
    ergo_sigma::cost_trace::enable();
    let (reduced, entries) =
        reduce_expr_traced_with_cost(&tree.body, &ctx, &tree.constants, &mut cost);
    #[cfg(feature = "cost-trace")]
    let cost_breakdown: Vec<CostLine> = ergo_sigma::cost_trace::take()
        .unwrap_or_default()
        .entries
        .into_iter()
        .map(|e| CostLine {
            label: e.label,
            delta: e.delta,
            total: e.total,
        })
        .collect();

    let mut outcome = EvalOutcome {
        verdict: Verdict::Error,
        error: None,
        cost: cost.total_block_cost(),
        cost_limit: limit,
        reduced_to: None,
        trace: entries
            .into_iter()
            .map(|TraceEntry { label, value }| TraceLine { label, value })
            .collect(),
        #[cfg(feature = "cost-trace")]
        cost_breakdown,
        tree_hex: hex::encode(&tree_bytes),
        p2s_address: encode_p2s(network.unwrap_or(NetworkPrefix::Mainnet), &tree_bytes),
    };

    // Reduction failure: a runtime exception (or budget exhaustion) is a
    // normal outcome, not a marshalling error.
    let proposition = match reduced {
        Ok(sb) => {
            outcome.reduced_to = Some(inspect::sigma_boolean_pretty(&sb));
            sb
        }
        Err(e) => {
            outcome.error = Some(e.to_string());
            return Ok(outcome);
        }
    };

    outcome.verdict = match &proposition {
        ergo_ser::sigma_value::SigmaBoolean::TrivialProp(true) => Verdict::Pass,
        ergo_ser::sigma_value::SigmaBoolean::TrivialProp(false) => Verdict::Fail,
        _ => Verdict::NeedsProof,
    };

    // 8. Optional proof verification through the FULL consensus path
    // (pre-reduction checks + deserialize-substitution cost + trivial fast
    // path + evaluator + crypto verify) with a FRESH accumulator — this is
    // the authoritative spend cost; the diagnostic pass above is a preview.
    if let Some(proof_hex) = &sc.proof {
        let proof_bytes = hex::decode(proof_hex.trim()).map_err(|source| SandboxError::Hex {
            field: "proof",
            source,
        })?;
        let message: Vec<u8> = match &sc.message {
            Some(m) => hex::decode(m.trim()).map_err(|source| SandboxError::Hex {
                field: "message",
                source,
            })?,
            None => Vec::new(),
        };
        let mut verify_cost = new_accumulator()?;
        match verify_spending_proof_with_context_and_cost(
            &tree,
            &proof_bytes,
            &message,
            &ctx,
            &mut verify_cost,
        ) {
            Ok(true) => outcome.verdict = Verdict::ProofAccepted,
            Ok(false) => outcome.verdict = Verdict::ProofRejected,
            Err(e) => {
                outcome.verdict = Verdict::Error;
                outcome.error = Some(e.to_string());
            }
        }
        outcome.cost = verify_cost.total_block_cost();
    }

    Ok(outcome)
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Prefix a box-marshalling error with the box's index in its collection.
fn index_err(i: usize) -> impl Fn(SandboxError) -> SandboxError {
    move |e| match e {
        SandboxError::Hex { field, source } => SandboxError::Hex { field, source },
        other => SandboxError::Scenario(format!("[{i}] {}", other)),
    }
}

fn parse_network(name: Option<&str>) -> Result<Option<NetworkPrefix>, SandboxError> {
    match name {
        None => Ok(None),
        Some("mainnet") => Ok(Some(NetworkPrefix::Mainnet)),
        Some("testnet") => Ok(Some(NetworkPrefix::Testnet)),
        Some(other) => Err(SandboxError::Scenario(format!(
            "unknown network `{other}` (expected `mainnet` or `testnet`)"
        ))),
    }
}
