//! S00 review questions answered only by observations that actually ran.
//! A lint miss stays unchecked. Artifact associations are supplied by the caller,
//! never inferred from acceptance, and do not establish a vector mechanism.

use ergo_ser::address::NetworkPrefix;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{audit, claim::ClaimMetadata, evidence::replay::ReplayBundle, Scenario};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provenance {
    Static,
    Scenario,
    Preflight,
    NodeValidated,
    Unchecked,
}

impl Provenance {
    pub fn label(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Scenario => "scenario",
            Self::Preflight => "preflight",
            Self::NodeValidated => "node-validated",
            Self::Unchecked => "unchecked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub node_id: u64,
    pub ir_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Observation {
    pub text: String,
    pub lint: &'static str,
    pub anchor: Anchor,
    pub snippet: String,
    pub provenance: Provenance,
}

impl From<&crate::Finding> for Observation {
    fn from(f: &crate::Finding) -> Self {
        Self {
            text: f.message.clone(),
            lint: f.lint,
            anchor: Anchor {
                node_id: f.node_id,
                ir_id: f.ir_id,
            },
            snippet: f.snippet.clone(),
            provenance: Provenance::Static,
        }
    }
}

/// The envelope associates an experiment with review questions. It is not an
/// assertion that the experiment exercises or resolves their mechanism.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScenarioArtifact {
    #[serde(default)]
    pub vector_ids: Vec<String>,
    pub scenario: Scenario,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreflightArtifact {
    #[serde(default)]
    pub vector_ids: Vec<String>,
    pub request: crate::txcheck::TxRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceArtifact {
    #[serde(default)]
    pub vector_ids: Vec<String>,
    pub bundle: ReplayBundle,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifacts {
    pub scenario: Option<ScenarioArtifact>,
    pub preflight: Option<PreflightArtifact>,
    pub evidence: Option<EvidenceArtifact>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactResult {
    pub fingerprint: String,
    pub provenance: Provenance,
    pub node_validated: bool,
    pub answer: String,
    /// Caller association, not a detected cause or a property attribution.
    pub vector_ids: Vec<String>,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub class: String,
    pub title: String,
    pub answer: String,
    pub provenance: Provenance,
    /// Static observations remain visible when an artifact also answers a row.
    pub findings: Vec<Observation>,
    pub artifact_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checklist {
    #[serde(flatten)]
    pub claim: ClaimMetadata,
    pub tree_hex: String,
    pub address: String,
    /// Same wire form as inspect: `"complete"` or `"partial"`.
    pub completeness: &'static str,
    pub raw_placeholders: usize,
    pub truncated: bool,
    pub rows: Vec<Row>,
    pub artifacts: Vec<ArtifactResult>,
}

#[derive(Deserialize)]
struct Catalogue {
    vectors: Vec<Vector>,
}
#[derive(Deserialize)]
struct Vector {
    id: String,
    class: String,
    title: String,
    instrument: Vec<String>,
}

fn catalogue() -> Vec<Vector> {
    serde_json::from_str::<Catalogue>(include_str!("../../docs/security/vectors.json"))
        .expect("S00 validates the embedded catalogue")
        .vectors
}

pub(crate) fn names_lint(lint: &str) -> bool {
    static LINTS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    LINTS
        .get_or_init(|| {
            catalogue()
                .iter()
                .flat_map(|v| &v.instrument)
                .filter_map(|i| i.strip_prefix("lint:").map(str::to_owned))
                .collect()
        })
        .contains(lint)
}

/// Resolve offline tree hex, an address on the selected network, or source.
/// Compilation uses the workbench's default tree version (3).
pub fn resolve_input(input: &str, network: NetworkPrefix) -> Result<Vec<u8>, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("input is empty".into());
    }
    if let Ok(bytes) = hex::decode(input) {
        return Ok(bytes);
    }
    if let Ok(bytes) = ergo_ser::address::decode_address_to_tree_bytes(input, network) {
        return Ok(bytes);
    }
    crate::compile_source(input, 3, network)
        .map(|c| c.tree_bytes)
        .map_err(|e| e.to_string())
}

fn associations(vectors: &[Vector], ids: &[String], kind: Provenance) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        let v = vectors
            .iter()
            .find(|v| &v.id == id)
            .ok_or_else(|| format!("unknown vector {id}"))?;
        let permitted = match kind {
            Provenance::Scenario => v.instrument.iter().any(|i| i == "scenario"),
            // Preflight is the supplied-transaction form of a scenario, not a lint.
            Provenance::Preflight => v.instrument.iter().any(|i| i == "scenario" || i == "drain"),
            Provenance::NodeValidated => v.instrument.iter().any(|i| i == "node-validated"),
            _ => false,
        };
        if !permitted || !seen.insert(id) {
            return Err(format!(
                "vector {id} does not name this instrument or is repeated"
            ));
        }
    }
    Ok(())
}

fn artifact_network(raw: Option<&str>, network: NetworkPrefix) -> Result<(), String> {
    let expected = if network == NetworkPrefix::Testnet {
        "testnet"
    } else {
        "mainnet"
    };
    if raw.is_some_and(|s| s != expected) {
        return Err("artifact network differs from the checklist network".into());
    }
    Ok(())
}

/// Run the registered audit and only the supplied experiments. Callers running
/// on small-stack async workers must use their engine budget/large-stack thread.
pub fn checklist(
    bytes: &[u8],
    network: NetworkPrefix,
    supplied: &Artifacts,
) -> Result<Checklist, String> {
    let tree = crate::inspect::parse_tree(bytes).map_err(|e| e.to_string())?;
    let lifted = crate::lift_tree(&tree, network == NetworkPrefix::Testnet);
    let audit = audit::audit(&lifted);
    let vectors = catalogue();
    let mut artifacts = Vec::new();
    let tree_hex = hex::encode(bytes);
    if let Some(a) = &supplied.scenario {
        associations(&vectors, &a.vector_ids, Provenance::Scenario)?;
        artifact_network(a.scenario.network.as_deref(), network)?;
        let mut scenario = a.scenario.clone();
        scenario.network = Some(
            if network == NetworkPrefix::Testnet {
                "testnet"
            } else {
                "mainnet"
            }
            .into(),
        );
        if scenario.tree.is_none() && scenario.source.is_none() {
            scenario.tree = Some(tree_hex.clone());
        }
        let result = crate::eval_scenario(&scenario).map_err(|e| e.to_string())?;
        if hex::decode(&result.tree_hex).map_err(|e| e.to_string())? != bytes {
            return Err("scenario evaluates a different contract".into());
        }
        artifacts.push(ArtifactResult {
            fingerprint: crate::evidence::case::json_digest(&json!(scenario)),
            provenance: Provenance::Scenario, node_validated: false,
            answer: format!("Supplied scenario reduced to {}. Synthetic; full node validation has not run. The caller associates this experiment with the vector; its mechanism is not established.", crate::testsuite::verdict_name(result.verdict)),
            vector_ids: a.vector_ids.clone(),
            result: json!({"verdict": result.verdict, "error": result.error, "cost": result.cost, "costLimit": result.cost_limit, "nodeValidated": false}),
        });
    }
    if let Some(a) = &supplied.preflight {
        associations(&vectors, &a.vector_ids, Provenance::Preflight)?;
        artifact_network(a.request.network.as_deref(), network)?;
        let mut request = a.request.clone();
        request.network = Some(
            if network == NetworkPrefix::Testnet {
                "testnet"
            } else {
                "mainnet"
            }
            .into(),
        );
        // Match the same final box-by-id selection as txcheck, including duplicates.
        let boxes: std::collections::BTreeMap<_, _> = request
            .boxes
            .iter()
            .filter_map(|b| Some((b["boxId"].as_str()?.to_lowercase(), b)))
            .collect();
        let matches = request.tx.inputs.iter().any(|i| {
            boxes
                .get(&i.box_id.to_lowercase())
                .and_then(|b| b["ergoTree"].as_str())
                .and_then(|s| hex::decode(s).ok())
                .is_some_and(|b| b == bytes)
        });
        if !matches {
            return Err("preflight does not spend this contract".into());
        }
        let result = crate::txcheck::check(&request).map_err(|e| e.to_string())?;
        artifacts.push(ArtifactResult {
            fingerprint: crate::evidence::case::json_digest(&json!(request)),
            provenance: Provenance::Preflight, node_validated: false,
            answer: format!("Supplied transaction preflight {}. Signatures are not checked; full node validation has not run. The caller associates this experiment with the vector; its mechanism is not established.", if result.preflight_passed { "passed" } else { "failed" }),
            vector_ids: a.vector_ids.clone(), result: json!(result),
        });
    }
    if let Some(a) = &supplied.evidence {
        associations(&vectors, &a.vector_ids, Provenance::NodeValidated)?;
        if a.bundle
            .execution
            .case
            .premises()
            .target_bytes
            .value()
            .is_some()
            && a.bundle.execution.case.target_bytes()? != bytes
        {
            return Err("evidence target differs from this contract".into());
        }
        let result = crate::evidence::replay::replay(&a.bundle);
        // The case target label alone does not bind a spending input. A valid
        // replayed property binds every spending input by its canonical box id.
        let spends_target = a
            .bundle
            .execution
            .case
            .premises()
            .boxes
            .value()
            .into_iter()
            .flatten()
            .filter_map(|record| crate::evidence::wire::WireBox::from_record(record.clone()).ok())
            .any(|b| {
                b.node().candidate.ergo_tree_bytes() == bytes
                    && b.id()
                        .is_ok_and(|id| a.bundle.property.inputs.iter().any(|i| i.box_id == id))
            });
        let validated = result["nodeValidated"] == true
            && spends_target
            && matches!(
                result["status"].as_str(),
                Some("confirmed-violation" | "accepted-nonviolating")
            );
        artifacts.push(ArtifactResult {
            fingerprint: a.bundle.fingerprint(),
            provenance: if validated { Provenance::NodeValidated } else { Provenance::Unchecked },
            node_validated: validated,
            answer: format!("Supplied bundle replay: {}. Limited to the supplied premises and declared property; no historical inclusion or vector mechanism is established. Vector association is caller-supplied.", result["status"].as_str().unwrap_or("incomplete-bundle")),
            vector_ids: a.vector_ids.clone(),
            // Retain scope, premise provenance and exact replay identity, without
            // duplicating the entire potentially large bundle in this surface.
            result: json!({"status":result["status"], "nodeValidated":result["nodeValidated"], "scope":result["scope"], "claim":result["claim"], "reason":result["reason"], "inputProvenance":result["inputProvenance"], "contextProvenance":result["contextProvenance"]}),
        });
    }
    let (completeness, raw_placeholders, truncated) = match audit.completeness {
        audit::Completeness::Complete => ("complete", 0, false),
        audit::Completeness::Partial {
            raw_placeholders,
            truncated,
        } => ("partial", raw_placeholders, truncated),
    };
    let rows = vectors.into_iter().map(|v| {
        let findings: Vec<Observation> = audit.findings.iter()
            .filter(|f| v.instrument.iter().any(|i| i.strip_prefix("lint:") == Some(f.lint)))
            .map(Observation::from).collect();
        let mut row = Row {
            id: v.id, class: v.class, title: v.title,
            answer: if findings.is_empty() { "Unchecked. No applicable static observation or associated experiment answers this question; a lint miss proves nothing.".into() }
                else { "Static observations for review; no vulnerability or safety verdict.".into() },
            provenance: if findings.is_empty() { Provenance::Unchecked } else { Provenance::Static },
            findings, artifact_fingerprints: vec![],
        };
        for a in &artifacts {
            if a.vector_ids.contains(&row.id) {
                row.artifact_fingerprints.push(a.fingerprint.clone());
                if a.provenance != Provenance::Unchecked {
                    row.provenance = a.provenance;
                    row.answer = a.answer.clone();
                }
            }
        }
        row
    }).collect();
    Ok(Checklist {
        claim: ClaimMetadata::STATIC,
        tree_hex,
        address: ergo_ser::address::encode_p2s(network, bytes),
        completeness,
        raw_placeholders,
        truncated,
        rows,
        artifacts,
    })
}
