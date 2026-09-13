//! Offline byte and structural verification on the pinned compiler.
use std::collections::BTreeMap;

use ergo_ser::address::NetworkPrefix;
use serde::Serialize;

use crate::{identity, SandboxError, TypedValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Exact,
    Template,
    NoMatch,
}

impl Outcome {
    /// CLI: 0 exact, 3 template, 4 no match; 1 is an input/engine error.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Exact => 0,
            Self::Template => 3,
            Self::NoMatch => 4,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    pub outcome: Outcome,
    pub compiled_tree_hex: String,
    pub target_tree_hex: String,
    /// Verbatim identity differences: left is compiled, right is target.
    pub constant_differences: Vec<identity::ConstantDifference>,
    pub limitation: &'static str,
}

/// Decode addresses offline. Hex, including exactly 32 bytes, always means a
/// tree here; box IDs are not accepted. A box caller supplies its ergoTree.
pub fn target_bytes(target: &str, network: NetworkPrefix) -> Result<Vec<u8>, SandboxError> {
    let target = target.trim();
    if let Some(hex) = target.strip_prefix("tree:") {
        return hex::decode(hex).map_err(|e| SandboxError::Tree(e.to_string()));
    }
    if let Some(address) = target.strip_prefix("address:") {
        return crate::tree::decode_address(address, Some(network))
            .map(|(bytes, _)| bytes)
            .map_err(SandboxError::Tree);
    }
    if !target.is_empty() && target.bytes().all(|b| b.is_ascii_hexdigit()) {
        hex::decode(target).map_err(|e| SandboxError::Tree(e.to_string()))
    } else {
        crate::tree::decode_address(target, Some(network))
            .map(|(bytes, _)| bytes)
            .map_err(SandboxError::Tree)
    }
}

/// Call on the engine's large stack, as for compilation and identity matching.
pub fn verify(
    target: &str,
    source: &str,
    params: &BTreeMap<String, TypedValue>,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<VerifyReport, SandboxError> {
    let target = target_bytes(target, network)?;
    let compiled = crate::compile::compile_with_params(source, params, tree_version, network)?;
    let matched = identity::match_trees(&compiled.tree_bytes, &target)?;
    let outcome = if matched.byte_identical {
        Outcome::Exact
    } else if matched.verdict == identity::MatchVerdict::SameProgramWithDifferingConstants {
        Outcome::Template
    } else {
        // SameProgram can ignore serialization flags or unused constants.
        // Neither that case nor structural overlap meets either positive test.
        Outcome::NoMatch
    };
    Ok(VerifyReport {
        claim: crate::claim::ClaimMetadata::STATIC,
        outcome,
        compiled_tree_hex: hex::encode(compiled.tree_bytes),
        target_tree_hex: hex::encode(target),
        constant_differences: if outcome == Outcome::Template {
            matched.constant_differences
        } else {
            Vec::new()
        },
        limitation: identity::LIMITATION,
    })
}
