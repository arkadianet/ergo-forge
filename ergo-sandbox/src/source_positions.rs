//! Compiler citations checked against the exact bytes evaluated by a run.
use crate::{eval::EvalOutcome, SandboxError, Scenario};
use ergo_compiler::SourceMap;
use ergo_ser::{
    ergo_tree::ErgoTree,
    opcode::{node_opcode, preorder},
};
use serde::Serialize;

/// The compiler supplies only a byte start, never an expression end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePosition {
    pub offset: u32,
    pub line: u32,
    pub col: u32,
    pub kind: &'static str,
}
impl SourcePosition {
    pub fn at(source: &str, offset: u32) -> Option<Self> {
        if offset as usize >= source.len() || !source.is_char_boundary(offset as usize) {
            return None;
        }
        let (line, col) = ergo_compiler::span::line_col(source, offset);
        Some(Self {
            offset,
            line,
            col,
            kind: "start",
        })
    }
}

pub struct SourcePositions {
    pub tree: ErgoTree,
    pub map: Option<SourceMap>,
    pub status: &'static str,
}
impl SourcePositions {
    /// Recompilation obtains metadata only; values/costs stay those of `out`.
    pub fn for_run(sc: &Scenario, out: &EvalOutcome) -> Result<Self, SandboxError> {
        let bytes = hex::decode(&out.tree_hex).map_err(|source| SandboxError::Hex {
            field: "tree",
            source,
        })?;
        let tree = crate::inspect::parse_tree(&bytes)?;
        let mut result = Self {
            tree,
            map: None,
            status: "no-source",
        };
        let Some(source) = &sc.source else {
            return Ok(result);
        };
        let network = if sc.network.as_deref() == Some("testnet") {
            ergo_ser::address::NetworkPrefix::Testnet
        } else {
            ergo_ser::address::NetworkPrefix::Mainnet
        };
        let (compiled, map) = crate::compile::compile_with_params_and_map(
            source,
            &sc.params,
            sc.tree_version,
            network,
        )
        .map_err(|e| SandboxError::Scenario(e.to_string()))?;
        result.status = "no-map";
        let Some(map) = map else { return Ok(result) };
        result.status = "misaligned";
        if compiled.tree_bytes != bytes
            || !map.aligns_with(preorder(&result.tree.body).map(|(_, e)| node_opcode(e)))
        {
            return Ok(result);
        }
        result.status = "substituted-source";
        // The public compile API supplies no substitution origin map. The
        // compiler scans alternating quote-delimited strings for parameter
        // names; conservatively withhold all citations on any such reference.
        // Include every parameter type so aliases and later substitutions
        // cannot accidentally make a replacement look like original text.
        if source.split('"').skip(1).step_by(2).any(|literal| {
            sc.params
                .keys()
                .any(|name| literal == name || literal.contains(&format!("${name}")))
        }) {
            return Ok(result);
        }
        result.status = "aligned";
        result.map = Some(map);
        Ok(result)
    }
    pub fn position(&self, source: &str, id: u64) -> Option<SourcePosition> {
        SourcePosition::at(source, self.map.as_ref()?.offset(id)?)
    }
}
