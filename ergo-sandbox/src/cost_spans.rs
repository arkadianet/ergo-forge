//! Conservative cost ownership. Trace order and runtime `n` are not node ids.
//! Counts include ALL opcode nodes, even unmapped and unevaluated candidates.
use crate::{eval::CostLine, source_positions::SourcePosition};
use ergo_compiler::SourceMap;
use ergo_ser::opcode::{node_opcode, preorder, Expr};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rule {
    Exact,
    Ambiguous,
    Unattributed,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub ir_id: u64,
    pub span: Option<SourcePosition>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSpan {
    pub label: String,
    pub raw_label: String,
    pub rule: Rule,
    pub reason: &'static str,
    pub jit: u64,
    pub count: usize,
    pub share: f64,
    pub candidates: Vec<Candidate>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostSpans {
    pub map_status: &'static str,
    pub total_jit: u64,
    pub attributed_jit: u64,
    pub ambiguous_jit: u64,
    pub unattributed_jit: u64,
    pub exact_share: f64,
    pub rows: Vec<CostSpan>,
}
fn opcode(label: &str) -> Option<u8> {
    // These are the engine's explicit opcode labels. EQ/Method/Crypto do
    // not name a unique IR operation. Do not infer ownership from their cost.
    let rest = label
        .strip_prefix("OP:0x")
        .or_else(|| label.strip_prefix("Arith:0x"))?;
    let (hex, detail) = rest
        .split_once(':')
        .map_or((rest, None), |(h, d)| (h, Some(d)));
    if hex.len() != 2
        || detail.is_some_and(|d| {
            d.strip_prefix("n=")
                .and_then(|n| n.parse::<u32>().ok())
                .is_none()
        })
    {
        return None;
    }
    u8::from_str_radix(hex, 16)
        .ok()
        .filter(|op| !matches!(op, 0x00 | 0x73 | 0x7f | 0x80))
}

/// The cumulative trace includes some charges without their own label
/// (e.g. inline constants). Preserve those gaps as unattributed cost.
/// A failed final charge beyond the last recorded total is outside this trace.
pub fn cost_spans(
    source: &str,
    root: &Expr,
    map: Option<&SourceMap>,
    map_status: &'static str,
    lines: &[CostLine],
) -> CostSpans {
    let walk: Vec<_> = preorder(root).collect();
    let aligned = map.is_some_and(|m| m.aligns_with(walk.iter().map(|(_, e)| node_opcode(e))));
    let map_status = if map.is_some() && !aligned {
        "misaligned"
    } else {
        map_status
    };
    // Deserialisation can introduce arbitrary opcodes absent from the tree.
    let dynamic = walk
        .iter()
        .any(|(_, e)| matches!(node_opcode(e), 0xd4 | 0xd5));
    let methods = walk
        .iter()
        .any(|(_, e)| matches!(node_opcode(e), 0xdb | 0xdc));
    let mut grouped: BTreeMap<String, (u64, usize)> = BTreeMap::new();
    let mut previous = 0;
    for line in lines {
        let gap = line.total.saturating_sub(previous + line.delta);
        if gap > 0 {
            let row = grouped.entry("Unlabelled trace cost".into()).or_default();
            row.0 += gap;
            row.1 += 1;
        }
        let row = grouped.entry(line.label.clone()).or_default();
        row.0 += line.delta;
        row.1 += 1;
        previous = line.total;
    }
    let total_jit = grouped.values().map(|(jit, _)| jit).sum();
    let share = |jit| {
        if total_jit == 0 {
            0.0
        } else {
            jit as f64 / total_jit as f64
        }
    };
    let mut result = CostSpans {
        map_status,
        total_jit,
        attributed_jit: 0,
        ambiguous_jit: 0,
        unattributed_jit: 0,
        exact_share: 0.0,
        rows: Vec::new(),
    };
    for (raw_label, (jit, count)) in grouped {
        let op = opcode(&raw_label);
        // Method implementations reuse these labels for operations with no
        // corresponding opcode node (pinned evaluator/method_call/{misc,global}).
        let alias = methods && op.is_some_and(|o| matches!(o, 0xc6 | 0xe3 | 0x9b));
        let candidates: Vec<_> = if aligned && !dynamic && !alias {
            walk.iter()
                .filter(|(_, e)| matches!(e, Expr::Op(_)) && Some(node_opcode(e)) == op)
                .map(|(id, _)| Candidate {
                    ir_id: *id,
                    span: map
                        .and_then(|m| m.offset(*id))
                        .and_then(|off| SourcePosition::at(source, off)),
                })
                .collect()
        } else {
            Vec::new()
        };
        let (rule, reason) = if !aligned {
            (Rule::Unattributed, "source map unavailable or misaligned")
        } else if dynamic {
            (
                Rule::Unattributed,
                "deserialisation may evaluate nodes outside the mapped tree",
            )
        } else if alias {
            (
                Rule::Unattributed,
                "method calls reuse this opcode cost label",
            )
        } else if candidates.len() > 1 {
            (
                Rule::Ambiguous,
                "several opcode nodes; trace has no node id; detail does not identify a node",
            )
        } else if candidates.first().is_some_and(|c| c.span.is_some()) {
            (
                Rule::Exact,
                "one opcode node in the entire tree, with a compiler start position",
            )
        } else {
            (
                Rule::Unattributed,
                "no uniquely positioned opcode node for this label",
            )
        };
        match rule {
            Rule::Exact => result.attributed_jit += jit,
            Rule::Ambiguous => result.ambiguous_jit += jit,
            Rule::Unattributed => result.unattributed_jit += jit,
        }
        result.rows.push(CostSpan {
            label: crate::hot_spots::describe_label(&raw_label),
            raw_label,
            rule,
            reason,
            jit,
            count,
            share: share(jit),
            candidates,
        });
    }
    result.exact_share = share(result.attributed_jit);
    result
        .rows
        .sort_by(|a, b| b.jit.cmp(&a.jit).then(a.raw_label.cmp(&b.raw_label)));
    result
}
