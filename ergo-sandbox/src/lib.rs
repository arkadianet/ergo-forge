//! ErgoScript workbench engine — the sandbox layer between the consensus
//! primitives and the tooling shells (CLI, REST, WASM/browser).
//!
//! This crate composes exactly two consensus primitives and adds NO third
//! implementation of either:
//!
//! | Concern | Crate | Entry used |
//! |---|---|---|
//! | reduce / cost / verify | `ergo-sigma` | `dispatch::reduce_expr_traced_with_cost`, `reduce::verify_spending_proof_with_context_and_cost` |
//! | compile (source→tree) | `ergo-compiler` | `tree::compile` |
//!
//! The sandbox owns only marshalling: JSON scenarios → `ReductionContext`
//! (the same context shape block validation assembles in
//! `ergo-validation/src/tx/script/mod.rs`), and primitive outputs →
//! [`EvalOutcome`]. It never mutates chain state and never runs outside
//! `#[cfg(test)]`-style trust boundaries; it is NOT a consensus surface — a
//! bug here produces a wrong answer in a playground, never a fork.
//!
//! # Module map
//!
//! - [`scenario`] — the JSON scenario model and the typed-value parser that
//!   turns user JSON into `SigmaType`/`SigmaValue` pairs and `EvalBox`es.
//! - [`eval`] — scenario → owned box collections → `ReductionContext` →
//!   bounded-cost reduction → [`EvalOutcome`] (verdict + cost + trace).
//! - [`inspect`] — the structural printer: ErgoTree bytes → readable IR view
//!   (the P0 spike, productized).
//! - [`compile`] — thin wrapper over `ergo_compiler::compile`.
//!
//! # Example
//!
//! ```
//! use ergo_sandbox::{Scenario, eval_scenario};
//!
//! let json = r#"{
//!     "source": "sigmaProp(HEIGHT > 100)",
//!     "height": 200
//! }"#;
//! let scenario: Scenario = serde_json::from_str(json).unwrap();
//! let outcome = eval_scenario(&scenario).unwrap();
//! assert_eq!(outcome.verdict, ergo_sandbox::Verdict::Pass);
//! assert!(outcome.cost > 0);
//! ```

pub mod audit;
pub mod box_build;
pub mod compile;
pub mod decompile;
pub mod eval;
pub mod hot_spots;
pub mod hunt;
pub mod inspect;
pub mod method_names;
pub mod scenario;

pub use audit::{Finding, Severity};
pub use compile::{compile_source, CompileOutput};
pub use decompile::{lift_tree, Lifted, Node, NodeKind};
pub use eval::{eval_scenario, EvalOutcome, Verdict, DEFAULT_COST_LIMIT};
pub use hunt::{hunt, Hunt, HuntOptions, HuntVerdict};
pub use inspect::{sigma_boolean_pretty, tree_report, tree_structure};
pub use scenario::{parse_typed_value, Scenario, ScenarioBox, TypedValue};

use thiserror::Error;

/// Sandbox-level failure: input marshalling or primitive invocation failed
/// before/around a verdict could form. A script that *evaluated* to `false`
/// or raised a runtime exception is a normal [`EvalOutcome`], not this.
#[derive(Debug, Error)]
pub enum SandboxError {
    /// Hex decode failure on a user-supplied byte field.
    #[error("invalid hex in `{field}`: {source}")]
    Hex {
        /// Field name for context (e.g. `tree`, `minerPubkey`).
        field: &'static str,
        /// Underlying hex error.
        source: hex::FromHexError,
    },
    /// ErgoTree bytes failed to parse.
    #[error("invalid ergo tree: {0}")]
    Tree(String),
    /// Scenario JSON was structurally valid but semantically unusable
    /// (bad value/type pair, dense-register violation, missing tree…).
    #[error("scenario error: {0}")]
    Scenario(String),
    /// Source compilation failed.
    #[error("compile failed: {0}")]
    Compile(#[from] ergo_compiler::CompileError),
    /// Cost limit itself was invalid (must fit the JIT block-cost domain).
    #[error("invalid cost limit {0}")]
    CostLimit(u64),
}
