//! Contract test suites: a contract plus named scenarios with expected
//! verdicts, run as one unit. Design record:
//! `docs/superpowers/specs/2026-09-03-contract-tests-design.md`.

use std::collections::BTreeMap;

use ergo_ser::address::NetworkPrefix;
use serde::{Deserialize, Serialize};

use crate::compile::{compile_with_params, ParamError};
use crate::eval::{eval_scenario, Verdict};
use crate::scenario::{Scenario, TypedValue};
use crate::SandboxError;

/// The suite file (`contract.test.json`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suite {
    /// ErgoScript source of the contract under test (with `params`).
    #[serde(default)]
    pub source: Option<String>,
    /// Compile-time constants for `source`.
    #[serde(default)]
    pub params: BTreeMap<String, TypedValue>,
    /// ErgoTree hex of the contract under test (instead of `source`).
    #[serde(default)]
    pub tree: Option<String>,
    /// `mainnet` (default) or `testnet`.
    #[serde(default)]
    pub network: Option<String>,
    /// Tree version for compilation (default 3).
    #[serde(default)]
    pub tree_version: Option<u8>,
    /// The cases.
    pub scenarios: Vec<Case>,
}

/// One case: a scenario plus a name and the verdict it must produce.
#[derive(Debug, Clone, Deserialize)]
pub struct Case {
    pub name: String,
    pub expect: Expect,
    #[serde(flatten)]
    pub scenario: Scenario,
}

/// The verdict a case expects — the sandbox verdict names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Expect {
    Pass,
    Fail,
    Error,
    NeedsProof,
    ProofAccepted,
    ProofRejected,
}

impl Expect {
    fn matches(self, v: Verdict) -> bool {
        matches!(
            (self, v),
            (Expect::Pass, Verdict::Pass)
                | (Expect::Fail, Verdict::Fail)
                | (Expect::Error, Verdict::Error)
                | (Expect::NeedsProof, Verdict::NeedsProof)
                | (Expect::ProofAccepted, Verdict::ProofAccepted)
                | (Expect::ProofRejected, Verdict::ProofRejected)
        )
    }
}

/// The wire name of a verdict (`pass`, `needsProof`, …).
pub fn verdict_name(v: Verdict) -> &'static str {
    match v {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Error => "error",
        Verdict::NeedsProof => "needsProof",
        Verdict::ProofAccepted => "proofAccepted",
        Verdict::ProofRejected => "proofRejected",
    }
}

/// What one case produced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseResult {
    pub name: String,
    pub expected: &'static str,
    /// The verdict the reducer gave, or `"invalid"` when the scenario could
    /// not be marshalled (see `error`).
    pub actual: &'static str,
    pub passed: bool,
    /// Runtime error text (script threw) or the marshalling error.
    pub error: Option<String>,
    pub reduced_to: Option<String>,
    pub cost: u64,
}

/// The whole suite's outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuiteResult {
    pub tree_hex: String,
    pub address: String,
    pub cases: Vec<CaseResult>,
    pub passed: usize,
    pub failed: usize,
}

/// Why a suite could not run at all (a case that merely fails is a result).
#[derive(Debug, thiserror::Error)]
pub enum SuiteError {
    #[error("suite needs `source` or `tree`")]
    NoContract,
    #[error("suite may not name both `source` and `tree`")]
    BothContracts,
    #[error(
        "scenario `{name}` names its own source/tree; the suite's contract is what is under test"
    )]
    ScenarioHasContract { name: String },
    #[error("unknown network {0:?}; expected \"mainnet\" or \"testnet\"")]
    Network(String),
    #[error("contract: {0}")]
    Compile(#[from] ParamError),
    #[error("{0}")]
    Sandbox(#[from] SandboxError),
}

/// Compile the contract once, then run every case against it.
pub fn run(suite: &Suite) -> Result<SuiteResult, SuiteError> {
    let network = match suite.network.as_deref() {
        None | Some("mainnet") => NetworkPrefix::Mainnet,
        Some("testnet") => NetworkPrefix::Testnet,
        Some(other) => return Err(SuiteError::Network(other.to_string())),
    };
    let tree_hex = match (&suite.source, &suite.tree) {
        (Some(_), Some(_)) => return Err(SuiteError::BothContracts),
        (None, None) => return Err(SuiteError::NoContract),
        (None, Some(t)) => t.trim().to_string(),
        (Some(src), None) => {
            let out =
                compile_with_params(src, &suite.params, suite.tree_version.unwrap_or(3), network)?;
            hex::encode(out.tree_bytes)
        }
    };
    let bytes = hex::decode(&tree_hex).map_err(|source| SandboxError::Hex {
        field: "tree",
        source,
    })?;
    let address = ergo_ser::address::encode_p2s(network, &bytes);

    let mut cases = Vec::with_capacity(suite.scenarios.len());
    for case in &suite.scenarios {
        if case.scenario.source.is_some() || case.scenario.tree.is_some() {
            return Err(SuiteError::ScenarioHasContract {
                name: case.name.clone(),
            });
        }
        let mut sc = case.scenario.clone();
        sc.tree = Some(tree_hex.clone());
        // "$self" as a box's ergoTree = the contract under test.
        let fill = |boxes: &mut Vec<crate::scenario::ScenarioBox>| {
            for b in boxes.iter_mut() {
                if b.ergo_tree.as_deref().map(str::trim) == Some("$self") {
                    b.ergo_tree = Some(tree_hex.clone());
                }
            }
        };
        fill(&mut sc.inputs);
        fill(&mut sc.outputs);
        fill(&mut sc.data_inputs);
        if let Some(sb) = sc.self_box.as_mut() {
            if sb.ergo_tree.as_deref().map(str::trim) == Some("$self") {
                sb.ergo_tree = None;
            }
        }
        sc.network = Some(if network == NetworkPrefix::Testnet {
            "testnet".into()
        } else {
            "mainnet".into()
        });
        let expected = expect_name(case.expect);
        let result = match eval_scenario(&sc) {
            Ok(o) => CaseResult {
                name: case.name.clone(),
                expected,
                actual: verdict_name(o.verdict),
                passed: case.expect.matches(o.verdict),
                error: o.error,
                reduced_to: o.reduced_to,
                cost: o.cost,
            },
            Err(e) => CaseResult {
                name: case.name.clone(),
                expected,
                actual: "invalid",
                passed: false,
                error: Some(e.to_string()),
                reduced_to: None,
                cost: 0,
            },
        };
        cases.push(result);
    }
    let passed = cases.iter().filter(|c| c.passed).count();
    Ok(SuiteResult {
        failed: cases.len() - passed,
        passed,
        tree_hex,
        address,
        cases,
    })
}

fn expect_name(e: Expect) -> &'static str {
    match e {
        Expect::Pass => "pass",
        Expect::Fail => "fail",
        Expect::Error => "error",
        Expect::NeedsProof => "needsProof",
        Expect::ProofAccepted => "proofAccepted",
        Expect::ProofRejected => "proofRejected",
    }
}
