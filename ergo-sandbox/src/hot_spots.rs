//! Cost hot-spots (P3c): the per-step cost trace folded into a ranked view —
//! which operations a scenario's evaluation actually spends its budget on.
//!
//! Works on [`CostLine`]s, so the fold itself needs no feature; producing
//! the lines does (`cost-trace`, a thread-local recorder in `ergo-sigma`).

use std::collections::HashMap;

use serde::Serialize;

use crate::eval::CostLine;
use crate::inspect::opcode_name;

/// One row of the ranked view.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotSpot {
    /// Human label — opcode name with its byte, plus any detail the
    /// evaluator attached (`Exists (0xAE) n=8`).
    pub label: String,
    /// JitCost units charged under this label in total.
    pub jit: u64,
    /// Number of charging steps under this label.
    pub count: usize,
    /// `jit` as a fraction of the whole trace (0..=1).
    pub share: f64,
}

/// Turn a raw trace label into a readable one. `OP:0xED` → `BinAnd (0xED)`;
/// `OP:0xAE:n=1` → `Exists (0xAE) n=1`; anything else is returned as is.
pub fn describe_label(raw: &str) -> String {
    let Some(rest) = raw.strip_prefix("OP:0x") else {
        return raw.to_string();
    };
    let (hex, detail) = match rest.split_once(':') {
        Some((h, d)) => (h, Some(d)),
        None => (rest, None),
    };
    let Ok(op) = u8::from_str_radix(hex, 16) else {
        return raw.to_string();
    };
    let name = opcode_name(op).unwrap_or("Op");
    match detail {
        Some(d) => format!("{name} (0x{hex}) {d}"),
        None => format!("{name} (0x{hex})"),
    }
}

/// Fold a trace into rows grouped by label, ranked by cost (ties by label).
pub fn hot_spots(lines: &[CostLine]) -> Vec<HotSpot> {
    let mut acc: HashMap<&str, (u64, usize)> = HashMap::new();
    for l in lines {
        let e = acc.entry(l.label.as_str()).or_default();
        e.0 += l.delta;
        e.1 += 1;
    }
    let total: u64 = lines.iter().map(|l| l.delta).sum();
    let mut rows: Vec<HotSpot> = acc
        .into_iter()
        .map(|(raw, (jit, count))| HotSpot {
            label: describe_label(raw),
            jit,
            count,
            share: if total == 0 {
                0.0
            } else {
                jit as f64 / total as f64
            },
        })
        .collect();
    rows.sort_by(|a, b| b.jit.cmp(&a.jit).then_with(|| a.label.cmp(&b.label)));
    rows
}
