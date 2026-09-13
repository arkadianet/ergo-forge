//! Export one input's synthetic Play evaluation as a suite and a CLI scenario.
//! Signing material is never exported. No source is recovered or recompiled.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::play::{prepare, PlayRequest};
use crate::testsuite::{Case, Expect, Suite};
use crate::{eval_scenario, SandboxError, Scenario};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportKind {
    Test,
    Scenario,
}

pub struct PlayExport {
    pub suite: Suite,
    pub scenario: Scenario,
}

/// `verdict` is the chosen input's verdict from `play::apply`, not `PlayResult.ok`
/// (which also checks conservation and every other input). Compare the replay
/// before exporting; a disagreement must be investigated as synthetic drift.
pub fn export(
    req: &PlayRequest,
    input_index: usize,
    verdict: &str,
) -> Result<PlayExport, SandboxError> {
    let mut scenario = prepare(req)?.scenario(req, input_index)?;
    let signing_removed = !scenario.secrets.is_empty() || !scenario.parties.is_empty();
    scenario.secrets.clear();
    scenario.parties.clear();
    let replay = eval_scenario(&scenario)?;
    let actual = crate::testsuite::verdict_name(replay.verdict);
    if !signing_removed && actual != verdict {
        return Err(SandboxError::Scenario(format!(
            "synthetic-drift: Play input {input_index} produced {verdict}, export replay produced {actual}"
        )));
    }
    let expect: Expect =
        serde_json::from_value(json!(actual)).map_err(|e| SandboxError::Scenario(e.to_string()))?;
    let mut case_scenario = scenario.clone();
    // The suite runner supplies the only contract under test to each case.
    case_scenario.tree = None;
    let note = if signing_removed {
        "; signing material omitted, expectation from signature-less run"
    } else {
        ""
    };
    let case = Case {
        name: format!("Synthetic Play input {input_index}{note}"),
        expect,
        expect_residual: None,
        expect_residual_excludes: None,
        scenario: case_scenario,
    };
    let suite = Suite {
        tree: scenario.tree.clone(),
        source: None,
        params: Default::default(),
        network: req.network.clone(),
        tree_version: None,
        scenarios: vec![case],
    };
    Ok(PlayExport { suite, scenario })
}

impl PlayExport {
    /// A runner-compatible JSON document, with explicit synthetic labels.
    /// Omit absent source/tree and params instead of naming empty contracts.
    pub fn document(&self, kind: ExportKind) -> Value {
        fn compact(value: impl Serialize) -> Value {
            let mut value = serde_json::to_value(value).expect("export serializes");
            value
                .as_object_mut()
                .expect("export object")
                .retain(|key, v| !v.is_null() && key != "params" && key != "source");
            value
        }
        let mut doc = match kind {
            ExportKind::Test => json!({
                "tree": self.suite.tree,
                "network": self.suite.network,
                "scenarios": [compact(&self.suite.scenarios[0])],
            }),
            ExportKind::Scenario => compact(&self.scenario),
        };
        let object = doc.as_object_mut().expect("export object");
        object.extend(
            serde_json::to_value(crate::claim::ClaimMetadata::SIMULATION)
                .expect("labels serialize")
                .as_object()
                .expect("labels object")
                .clone(),
        );
        object.insert("synthetic".into(), json!(true));
        doc
    }
}
