//! Thin wrapper over the ONE compile primitive,
//! `ergo_compiler::compile` (`ergoscript-tooling-api.md` §3.5). No
//! reimplementation; the env is empty (named-constant envs are an M6/M7
//! concern; the sandbox exposes them when the compiler surface does).

use ergo_compiler::{CompileError, CompileResult, NetworkPrefix};

/// The output of a successful source compilation.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// Canonical ErgoTree wire bytes.
    pub tree_bytes: Vec<u8>,
    /// Parsed tree.
    pub ergo_tree: ergo_ser::ergo_tree::ErgoTree,
    /// Pay-to-script address.
    pub p2s_address: String,
    /// Pay-to-script-hash address.
    pub p2sh_address: String,
}

/// Compile ErgoScript source into tree bytes + addresses.
///
/// `network` affects only the address encodings, not the tree bytes.
pub fn compile_source(
    source: &str,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<CompileOutput, CompileError> {
    let CompileResult {
        tree_bytes,
        ergo_tree,
        p2s_address,
        p2sh_address,
    } = ergo_compiler::compile(
        &ergo_compiler::ScriptEnv::new(),
        source,
        tree_version,
        network,
    )?;
    Ok(CompileOutput {
        tree_bytes,
        ergo_tree,
        p2s_address,
        p2sh_address,
    })
}
