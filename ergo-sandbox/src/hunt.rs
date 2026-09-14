//! The spend hunt: **"can someone who holds no key spend this box?"**
//!
//! Bounded scenario sampling over the sandbox evaluator. Each probe is a
//! node-engine scenario reduction of the tree with **no proof and no context
//! variables**. Six legacy height/output samples are followed by six
//! immobilisation samples: absent registers, minimum-valued outputs, and an
//! explicit block-budget reduction, each with attacker/preserve outputs.
//! Positional scaffolding is bounded and recorded; it is synthetic material.
//!
//! A hit is a passing sampled scenario; canonical transaction validation has
//! not run. A miss says only "not under these probes". Synthetic SELF applies
//! to positive and negative results alike — see [`Hunt::self_synthetic`].
//!
//! Design record: `docs/superpowers/specs/2026-09-02-p3b-spend-hunt-design.md`.

use std::{collections::BTreeSet, sync::OnceLock};

use ergo_ser::address::NetworkPrefix;
use serde::Serialize;

use crate::eval::{eval_scenario, Verdict};
use crate::scenario::{Scenario, ScenarioBox};
use crate::{compile, SandboxError};

/// Base height when the caller gives none: near the mainnet tip at the time
/// of writing. Only the *relative* probes (`+1M`, `1`) matter for most
/// guards; pass the real height for anything time-sensitive.
pub const DEFAULT_BASE_HEIGHT: u32 = 1_500_000;

/// What the attacker puts in `OUTPUTS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputShape {
    /// One box with SELF's value and tokens, guarded by `sigmaProp(true)`:
    /// the sampled output receives the funds. Canonical validation has not run.
    Attacker,
    /// One box copying SELF entirely (tree, value, tokens, registers): the
    /// funds stay in the contract in this sample.
    Preserve,
}

/// The fixed experiment family; all outcomes remain synthetic reductions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeKind {
    Spend,
    RegistersAbsent,
    MinimumOutputValue,
    CostLimit,
}

/// Work caps, including positional material recovered from the lifted tree.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeCaps {
    pub max_probes: usize,
    pub max_boxes_per_collection: usize,
    pub block_cost_limit: u64,
    pub min_value_per_byte: u64,
    pub basis: &'static str,
}

pub const MAX_PROBES: usize = 12;
pub const MAX_BOXES: usize = 16;
pub const NODE_RULE_BASIS: &str = "arkadianet/ergo@9468043396e5daa2828211bcff4234bc70fae4f0: ergo-validation/src/tx/structural.rs::check_output_box; context.rs::ProtocolParams::mainnet_default; ergo-sandbox/src/eval.rs::DEFAULT_COST_LIMIT";

/// Caller-controlled knobs. `Default` is the anonymous hunt: synthetic SELF,
/// default base height, mainnet.
#[derive(Debug, Clone, Default)]
pub struct HuntOptions {
    /// Base spending height (default [`DEFAULT_BASE_HEIGHT`]).
    pub height: Option<u32>,
    /// The box being spent. `None` means a synthetic box: no registers,
    /// value 0 — any register read then errors, a false negative the
    /// report flags via [`Hunt::self_synthetic`].
    pub self_box: Option<ScenarioBox>,
    /// Network for address rendering in outcomes (default mainnet).
    pub network: Option<NetworkPrefix>,
    /// Read-only data inputs (`CONTEXT.dataInputs`). On-chain facts, not
    /// spender secrets, so supplying them keeps the "anyone" question
    /// honest; contracts that read an oracle box error out without them.
    pub data_inputs: Vec<ScenarioBox>,
}

/// One probe: its context and what the reducer said.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub kind: ProbeKind,
    /// A scoped observation, never a chain spendability fact.
    pub observation: &'static str,
    pub cost_limit: u64,
    pub cost_exhausted: bool,
    /// Absent reads reached before an Option.get error, from evaluator values.
    pub erroring_reads: Vec<String>,
    pub output_values: Vec<i64>,
    /// Spending height.
    pub height: u32,
    /// Output shape.
    pub output: OutputShape,
    /// The sandbox verdict for this context.
    pub verdict: Verdict,
    /// Residual sigma proposition when the tree needs a proof.
    pub reduced_to: Option<String>,
    /// Runtime error text when the script raised one.
    pub error: Option<String>,
    /// Block-cost units the reduction consumed.
    pub cost: u64,
}

/// The aggregate answer, in priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HuntVerdict {
    /// An attacker-output sample passed without a proof; legacy wire name retained.
    SpendableByAnyone,
    /// Only preserve-output samples passed: the sampled output pays back
    /// into the same contract. Often by design (refresh boxes, oracle pools).
    MovableByAnyone,
    /// Nothing passed; at least one probe reduced to a sigma proposition.
    /// [`Hunt::residuals`] records observed proof requirements, not all possible spenders.
    RequiresProof,
    /// Every probe failed or errored. Explicitly *not* "safe".
    NotUnderProbes,
}

/// The hunt's result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunt {
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    /// The aggregate verdict.
    pub verdict: HuntVerdict,
    pub caps: ProbeCaps,
    pub probe_set: Vec<ProbeKind>,
    pub truncated: bool,
    pub register_reads: Vec<String>,
    /// Human-readable aggregate observation.
    pub observation: &'static str,
    /// Every probe, in the order run.
    pub probes: Vec<Probe>,
    /// Distinct residual propositions across `needsProof` probes.
    pub residuals: Vec<String>,
    /// True when no `self_box` was supplied, so SELF has no registers and
    /// value 0. Every verdict with this set describes synthetic SELF.
    pub self_synthetic: bool,
}

/// The attacker's output script, `sigmaProp(true)`, compiled once by the
/// oracle-pinned compiler rather than hand-written.
fn attacker_tree_hex() -> &'static str {
    static HEX: OnceLock<String> = OnceLock::new();
    HEX.get_or_init(|| {
        let out = compile::compile_source("sigmaProp(true)", 3, NetworkPrefix::Mainnet)
            .expect("sigmaProp(true) compiles");
        hex::encode(out.tree_bytes)
    })
}

/// Run the hunt over `tree_bytes`.
///
/// Errors are marshalling only (unparseable tree, bad `self_box`); a script
/// that ran and failed or errored is a normal probe outcome.
pub fn hunt(tree_bytes: &[u8], opts: &HuntOptions) -> Result<Hunt, SandboxError> {
    // Fail fast on bytes the reducer could never run, before building probes.
    let tree = crate::inspect::parse_tree(tree_bytes)?;
    let lifted = crate::lift_tree(&tree, false);
    let mut vals = crate::audit::boxrefs::Vals::new();
    crate::audit::boxrefs::collect_vals(&lifted.node, &mut vals);
    let mut reads = Vec::new();
    let mut positions = [1usize, 1, opts.data_inputs.len()];
    let mut truncated =
        lifted.truncated || lifted.raw_placeholders > 0 || opts.data_inputs.len() > MAX_BOXES;
    scan_accesses(
        &lifted.node,
        &vals,
        &mut reads,
        &mut positions,
        &mut truncated,
    );
    let params = ergo_validation::ProtocolParams::mainnet_default();
    let caps = ProbeCaps {
        max_probes: MAX_PROBES,
        max_boxes_per_collection: MAX_BOXES,
        block_cost_limit: crate::eval::DEFAULT_COST_LIMIT,
        min_value_per_byte: params.min_value_per_byte,
        basis: NODE_RULE_BASIS,
    };

    let tree_hex = hex::encode(tree_bytes);
    // A supplied box may name its tree, but it must be the tree under test:
    // the evaluator pins SELF's script to `tree_bytes`, so a different
    // `ergoTree` would be silently ignored rather than honoured.
    if let Some(named) = opts
        .self_box
        .as_ref()
        .and_then(|b| b.ergo_tree.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !named.eq_ignore_ascii_case(&tree_hex) {
            return Err(SandboxError::Scenario(
                "selfBox.ergoTree differs from the tree under test; omit it or make it match"
                    .into(),
            ));
        }
    }
    let base = opts.height.unwrap_or(DEFAULT_BASE_HEIGHT);
    let self_synthetic = opts.self_box.is_none();
    let self_box = opts.self_box.clone().unwrap_or_default();
    let network = match opts.network {
        Some(NetworkPrefix::Testnet) => Some("testnet".to_string()),
        _ => None,
    };

    let attacker_out = ScenarioBox {
        value: self_box.value,
        ergo_tree: Some(attacker_tree_hex().to_string()),
        tokens: self_box.tokens.clone(),
        creation_height: base,
        registers: Default::default(),
        box_id: None,
        extension: Default::default(),
    };
    let preserve_out = ScenarioBox {
        ergo_tree: Some(tree_hex.clone()),
        creation_height: base,
        box_id: None,
        ..self_box.clone()
    };

    let heights = [base, base.saturating_add(1_000_000), 1];
    let shapes = [
        (OutputShape::Attacker, &attacker_out),
        (OutputShape::Preserve, &preserve_out),
    ];

    let mut probes = Vec::with_capacity(MAX_PROBES);
    let probe_set = vec![
        ProbeKind::Spend,
        ProbeKind::RegistersAbsent,
        ProbeKind::MinimumOutputValue,
        ProbeKind::CostLimit,
    ];
    for &kind in &probe_set {
        let sample_heights: &[u32] = if kind == ProbeKind::Spend {
            &heights
        } else {
            &heights[..1]
        };
        for &height in sample_heights {
            for &(shape, out) in &shapes {
                let mut sc = Scenario {
                    params: Default::default(),
                    headers: Vec::new(),
                    secrets: Vec::new(),
                    parties: Vec::new(),
                    avl: Default::default(),
                    tree: Some(tree_hex.clone()),
                    source: None,
                    tree_version: 0,
                    network: network.clone(),
                    height,
                    self_box: Some(self_box.clone()),
                    self_index: None,
                    inputs: Vec::new(),
                    outputs: vec![out.clone()],
                    data_inputs: opts.data_inputs.iter().take(MAX_BOXES).cloned().collect(),
                    context_vars: Default::default(),
                    miner_pubkey: None,
                    pre_header: None,
                    cost_limit: None,
                    activated_script_version: None,
                    proof: None,
                    message: None,
                };
                if kind != ProbeKind::Spend {
                    let filler = ScenarioBox {
                        ergo_tree: Some(attacker_tree_hex().into()),
                        ..Default::default()
                    };
                    sc.inputs.resize(
                        positions[0].saturating_sub(1).min(MAX_BOXES - 1),
                        filler.clone(),
                    );
                    sc.outputs.resize(positions[1].min(MAX_BOXES), out.clone());
                    sc.data_inputs.resize(positions[2].min(MAX_BOXES), filler);
                    if kind == ProbeKind::RegistersAbsent {
                        sc.self_box.as_mut().unwrap().registers.clear();
                        for b in sc
                            .inputs
                            .iter_mut()
                            .chain(&mut sc.outputs)
                            .chain(&mut sc.data_inputs)
                        {
                            b.registers.clear();
                        }
                    }
                    if kind == ProbeKind::MinimumOutputValue {
                        for (index, b) in sc.outputs.iter_mut().enumerate() {
                            minimum_output_value(b, index as u16, caps.min_value_per_byte)?;
                        }
                    }
                    // The budget is the node's block cost limit, stated
                    // explicitly for the cost probe; no lower limit is invented,
                    // and the other probes keep the evaluator's default (the
                    // same value) implicitly.
                    if kind == ProbeKind::CostLimit {
                        sc.cost_limit = Some(caps.block_cost_limit);
                    }
                }
                let o = eval_scenario(&sc)?;
                let cost_exhausted = o
                    .error
                    .as_deref()
                    .is_some_and(|e| e.contains("cost limit exceeded:"));
                let erroring_reads = if o
                    .error
                    .as_deref()
                    .is_some_and(|e| e.contains("Some value") && e.contains("None"))
                {
                    reads
                        .iter()
                        .filter(|(id, _)| {
                            lifted.ir_ids.get(id).is_some_and(|ir| {
                                o.values
                                    .iter()
                                    .any(|v| v.ir_id == *ir && v.value.contains("None"))
                            })
                        })
                        .map(|(_, name)| name.clone())
                        .collect()
                } else {
                    vec![]
                };
                probes.push(Probe {
                    kind,
                    observation: match o.verdict {
                        Verdict::Pass => "spendable-under-probe",
                        Verdict::Error => "unspendable-under-probe",
                        _ => "not-spendable-under-probe",
                    },
                    cost_limit: o.cost_limit,
                    cost_exhausted,
                    erroring_reads,
                    output_values: sc.outputs.iter().map(|b| b.value).collect(),
                    height,
                    output: shape,
                    verdict: o.verdict,
                    reduced_to: o.reduced_to,
                    error: o.error,
                    cost: o.cost,
                });
            }
        }
    }

    let passed = |shape: OutputShape| {
        probes
            .iter()
            .any(|p| p.output == shape && p.verdict == Verdict::Pass)
    };
    let mut residuals: Vec<String> = Vec::new();
    for p in &probes {
        if p.verdict == Verdict::NeedsProof {
            if let Some(r) = &p.reduced_to {
                if !residuals.contains(r) {
                    residuals.push(r.clone());
                }
            }
        }
    }
    let verdict = if passed(OutputShape::Attacker) {
        HuntVerdict::SpendableByAnyone
    } else if passed(OutputShape::Preserve) {
        HuntVerdict::MovableByAnyone
    } else if !residuals.is_empty() {
        HuntVerdict::RequiresProof
    } else {
        HuntVerdict::NotUnderProbes
    };

    Ok(Hunt {
        claim: crate::claim::ClaimMetadata::legacy(
            "bounded-scenario-sampling",
            if self_synthetic {
                "synthetic SELF; caller-supplied/default context"
            } else {
                "caller-supplied SELF and data inputs; default/generated scenario material"
            },
        ),
        verdict,
        observation: if probes.iter().all(|p| p.verdict == Verdict::Error) {
            "unspendable-under-probe: not spendable under these probes; every probe errored"
        } else if !probes.iter().any(|p| p.verdict == Verdict::Pass) {
            "not spendable under these probes"
        } else {
            "a synthetic sample admitted a spend; full node validation has not run"
        },
        caps,
        probe_set,
        truncated,
        register_reads: reads
            .into_iter()
            .map(|(_, r)| r)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        probes,
        residuals,
        self_synthetic,
    })
}

// Literal positions only. Unknown/computed references and lift limits are visible.
fn scan_accesses(
    n: &crate::Node,
    vals: &crate::audit::boxrefs::Vals,
    reads: &mut Vec<(u64, String)>,
    positions: &mut [usize; 3],
    truncated: &mut bool,
) {
    use crate::audit::boxrefs::{as_indexed, box_collection, box_key, deref};
    if let Some((coll, idx)) = as_indexed(n) {
        if let Some(name) = box_collection(coll, vals) {
            let slot = match name {
                "INPUTS" => 0,
                "OUTPUTS" => 1,
                _ => 2,
            };
            match crate::decompile::print(deref(idx, vals)).parse::<usize>() {
                Ok(i) if i < MAX_BOXES => positions[slot] = positions[slot].max(i + 1),
                _ => *truncated = true,
            }
        }
    }
    if let crate::NodeKind::Method(receiver, name, args) = &n.kind {
        if name == "get" && args.is_empty() {
            let prop = deref(receiver, vals);
            if let crate::NodeKind::Prop(b, r) = &prop.kind {
                if r.starts_with('R') && r.contains('[') {
                    if let Some(key) = box_key(b, vals) {
                        reads.push((prop.id, format!("{key}.{r}.get")));
                    }
                }
            }
        }
    }
    for c in crate::audit::children(n) {
        scan_accesses(c, vals, reads, positions, truncated);
    }
}

/// Smallest value satisfying the pinned node's size-dependent rule. Serialise
/// through the existing node-backed box builder, including the output index.
/// Starting at zero reaches the least fixed point as the value VLQ grows.
fn minimum_output_value(b: &mut ScenarioBox, index: u16, factor: u64) -> Result<(), SandboxError> {
    b.value = 0;
    loop {
        let built = crate::box_build::build_eval_box_in("outputs", b, None, [0; 32], index)?;
        let minimum = (built.raw_bytes.len() as u64).saturating_mul(factor) as i64;
        if b.value >= minimum {
            return Ok(());
        }
        b.value = minimum;
    }
}
