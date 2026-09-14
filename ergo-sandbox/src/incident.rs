//! Incident authoring only: preserve chain boxes and leave every verdict unwritten.
use crate::map::source::{ChainBox, ChainSource};
use serde_json::{json, Value};
use std::path::Path;

pub const PLACEHOLDER: &str = "REPLACE-ME";

#[derive(Debug)]
pub struct ScriptSuite {
    pub directory: String,
    /// Deliberately not a runnable `Suite`: its expectations need an author.
    pub document: Value,
}

#[derive(Debug)]
pub struct IncidentScaffold {
    pub suites: Vec<ScriptSuite>,
    pub readme: String,
    pub triage: Value,
}

/// Preserve the scenario projection, without decoding/re-encoding any hex.
fn scenario_box(b: &ChainBox) -> Value {
    let mut out = json!({"value": b.value, "ergoTree": b.ergo_tree,
        "boxId": b.box_id, "creationHeight": b.creation_height});
    if !b.tokens.is_empty() {
        out["tokens"] = json!(b.tokens);
    }
    if !b.registers.is_empty() {
        out["registers"] = b
            .registers
            .iter()
            .map(|(name, raw)| (name.clone(), json!({"type": "raw", "value": raw})))
            .collect::<serde_json::Map<_, _>>()
            .into();
    }
    out
}

/// Fetch exactly one transaction through `ChainSource`, group spent scripts in
/// first-input order, and keep each matching SELF position as a separate case.
/// No reducer, triage search, chain-tip lookup or broadcast is involved.
pub fn scaffold(
    source: &dyn ChainSource,
    tx_id: &str,
    network: &str,
) -> Result<IncidentScaffold, String> {
    if tx_id.len() != 64 || !tx_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("txid must be 64 hex characters".into());
    }
    if !matches!(network, "mainnet" | "testnet") {
        return Err("network must be mainnet or testnet".into());
    }
    let tx = source.transaction(tx_id).map_err(|e| e.to_string())?;
    if tx.inputs.is_empty() || tx.outputs.is_empty() {
        return Err("incident transaction needs spent inputs and outputs".into());
    }
    let inputs: Vec<_> = tx.inputs.iter().map(scenario_box).collect();
    let outputs: Vec<_> = tx.outputs.iter().map(scenario_box).collect();
    let data_inputs: Vec<_> = tx.data_inputs.iter().map(scenario_box).collect();
    let height = tx.inclusion_height.map_or(json!(PLACEHOLDER), |h| json!(h));
    let mut scripts: Vec<Vec<u8>> = Vec::new();
    let mut suites: Vec<ScriptSuite> = Vec::new();
    for (index, b) in tx.inputs.iter().enumerate() {
        let bytes = hex::decode(&b.ergo_tree).map_err(|e| format!("input {index} tree: {e}"))?;
        if bytes.is_empty() {
            return Err(format!("input {index} has an empty tree"));
        }
        let case = json!({"name": format!("input {index}: author must set expectation"),
            "expect": PLACEHOLDER, "height": height, "selfIndex": index,
            "inputs": inputs, "outputs": outputs, "dataInputs": data_inputs});
        if let Some(at) = scripts.iter().position(|s| s == &bytes) {
            suites[at].document["scenarios"]
                .as_array_mut()
                .unwrap()
                .push(case);
        } else {
            scripts.push(bytes);
            suites.push(ScriptSuite {
                directory: format!("input-{index}"),
                document: json!({"tree": b.ergo_tree, "network": network, "scenarios": [case]}),
            });
        }
    }
    let height_note = if tx.inclusion_height.is_some() {
        "Height came from the spending transaction's inclusionHeight."
    } else {
        "MISSING SPENDING HEIGHT: replace every height REPLACE-ME with the inclusion height; the chain tip and box creation heights are not substitutes."
    };
    let files = suites
        .iter()
        .map(|s| format!("- `{}/contract.test.json`", s.directory))
        .collect::<Vec<_>>()
        .join("\n");
    let readme = format!("# Incident draft: {tx_id}\n\nNetwork: {network}.\n\n{height_note}\n\n{files}\n\nOne suite per distinct spent script, including funding/decoy scripts. Each occurrence has its original selfIndex. Inputs, outputs and data inputs keep transaction order.\n\n## Author work\n\nReplace every expect REPLACE-ME with an independently justified verdict. `ergo-es test` rejects the draft until these placeholders (and any missing height) are filled. Never infer expectations from the observed spend. Add a fixed-source suite separately, reusing the same boxes and declaring its expected rejection by hand.\n\nFill triage.json from a fresh static audit and your declared roles, objective and bounds. Null fields are deliberate; empty objective terms are not an authorization policy.\n\n## Evidence boundary\n\nThis scaffold only copies boxes and does not assert exploitability. Subsequent suite reductions are synthetic, nodeValidated: false. Nothing is broadcast. The live explorer path was unverified in the implementation session; the same scaffold path was tested with offline fixtures.\n\n## Projection\n\nBox value, ergoTree, token order/ids/amounts, serialized register hex, creationHeight and boxId are retained. Registers use scenario raw wrappers; empty tokens/registers are omitted. Explorer metadata (address, settlement height, transaction id, output index, rendered registers and proofs) is outside this scenario projection. Hex case is retained.\n");
    let triage = json!({"inputIndex": null, "lint": "", "nodeId": null,
        "drain": {"height": null, "inputs": [], "outputs": [], "network": "",
            "maxPermutations": null, "maxProbes": null, "protocolNfts": [], "objective": {"terms": []}}});
    Ok(IncidentScaffold {
        suites,
        readme,
        triage,
    })
}

impl IncidentScaffold {
    /// Create a new directory; never replace an author's existing incident.
    pub fn write_to(&self, out: &Path) -> Result<(), String> {
        let mut files = vec![
            ("README.md".to_string(), self.readme.clone()),
            (
                "triage.json".into(),
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&self.triage).map_err(|e| e.to_string())?
                ),
            ),
        ];
        for suite in &self.suites {
            files.push((
                format!("{}/contract.test.json", suite.directory),
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&suite.document).map_err(|e| e.to_string())?
                ),
            ));
        }
        std::fs::create_dir(out).map_err(|e| format!("create new incident directory: {e}"))?;
        for (name, contents) in files {
            let path = out.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(path, contents).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
