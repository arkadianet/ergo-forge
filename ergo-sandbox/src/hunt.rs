//! The spend hunt: **"can someone who holds no key spend this box?"**
//!
//! Bounded scenario sampling over the sandbox evaluator. Each probe is a
//! full consensus reduction of the tree with **no proof and no context
//! variables** — that is what "anyone" means — varying only the two things
//! an attacker controls freely: the spending height and the outputs.
//!
//! A hit is real: the probe's context is a transaction anyone can build. A
//! miss says only "not under these probes" — see [`Hunt::self_synthetic`].
//!
//! Design record: `docs/superpowers/specs/2026-09-02-p3b-spend-hunt-design.md`.

use std::sync::OnceLock;

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
    /// the funds leave the contract. A pass here means *stealable*.
    Attacker,
    /// One box copying SELF entirely (tree, value, tokens, registers): the
    /// funds stay in the contract. A pass here means *movable by anyone*.
    Preserve,
}

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
    /// An attacker-output probe passed: anyone can take the funds.
    SpendableByAnyone,
    /// Only preserve-output probes passed: anyone can re-spend the box back
    /// into the same contract. Often by design (refresh boxes, oracle pools).
    MovableByAnyone,
    /// Nothing passed; at least one probe reduced to a sigma proposition.
    /// [`Hunt::residuals`] says who can spend.
    RequiresProof,
    /// Every probe failed or errored. Explicitly *not* "safe".
    NotUnderProbes,
}

/// The hunt's result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunt {
    /// The aggregate verdict.
    pub verdict: HuntVerdict,
    /// Every probe, in the order run.
    pub probes: Vec<Probe>,
    /// Distinct residual propositions across `needsProof` probes.
    pub residuals: Vec<String>,
    /// True when no `self_box` was supplied, so SELF has no registers and
    /// value 0. A `notUnderProbes` verdict with this set means "supply the
    /// real box before drawing a conclusion".
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
    crate::inspect::parse_tree(tree_bytes)?;

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

    let mut probes = Vec::with_capacity(heights.len() * shapes.len());
    for &height in &heights {
        for &(shape, out) in &shapes {
            let sc = Scenario {
                headers: Vec::new(),
                secrets: Vec::new(),
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
                data_inputs: opts.data_inputs.clone(),
                context_vars: Default::default(),
                miner_pubkey: None,
                pre_header: None,
                cost_limit: None,
                activated_script_version: None,
                proof: None,
                message: None,
            };
            let o = eval_scenario(&sc)?;
            probes.push(Probe {
                height,
                output: shape,
                verdict: o.verdict,
                reduced_to: o.reduced_to,
                error: o.error,
                cost: o.cost,
            });
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
        verdict,
        probes,
        residuals,
        self_synthetic,
    })
}
