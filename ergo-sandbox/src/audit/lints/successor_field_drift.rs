//! Fields read on SELF without a recognised successor constraint.
//!
//! Reuses `delegated_reserves`' script/token successor detection and reserve
//! comparisons. Adds token-amount slots read on SELF (even without an id
//! equality) and extracted SELF registers. A register comparison must mention
//! the same typed register on SELF and this successor; arithmetic transitions
//! count, while presence tests and constraints on a different output do not.
//!
//! Known limits (deliberate): whole-tree syntax, not reachability or invariant
//! preservation. Branch-local comparisons can suppress observations. Register
//! arithmetic, direction and sufficiency are not proved; dynamic registers,
//! computed indices and searched successors are undecided. Intentional state
//! resets, singleton supply and delegated accounting need human review, so
//! findings are LOW observations. Silence never proves fields preserved.

use std::collections::{BTreeSet, HashSet};

use crate::audit::boxrefs::{box_key, collect_vals, deref, reserve_read, Vals};
use crate::audit::{children, Finding, Severity};
use crate::{Node, NodeKind};

/// Report identity-bound successors with missing local field relationships.
#[must_use]
pub fn successor_field_drift(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let evidence = super::delegated_reserves::evidence(root, &vals);
    let mut slots = BTreeSet::new();
    let mut registers = BTreeSet::new();
    let mut carried = HashSet::new();
    scan(root, &vals, &mut slots, &mut registers, &mut carried);
    evidence.successors.iter().filter_map(|(key, successor)| {
        let mut missing = Vec::new();
        if !evidence.erg_bounds.contains(key) {
            missing.push("ERG value has no recognised floor relating it to SELF.value".into());
        }
        if !evidence.token_collections.contains(key) {
            for slot in slots.union(&successor.id_slots) {
                if !evidence.amounts.contains(&(key.clone(), slot.clone())) {
                    missing.push(format!("tokens({slot})._2 has no recognised amount constraint"));
                }
            }
        }
        for register in &registers {
            if !carried.contains(&(key.clone(), register.clone())) {
                missing.push(format!("{register} is read on SELF without a recognised successor relationship"));
            }
        }
        if missing.is_empty() { return None; }
        Some(Finding {
            triage: Default::default(),
            lint: "successor-field-drift",
            severity: Severity::Low,
            node_id: successor.node_id,
            ir_id: None,
            message: format!("{key} preserves SELF's identity, but {}. Review intentional state changes, \
                singleton supply and companion contracts; this syntax observation is not evidence of exploitability.", missing.join("; ")),
            snippet: key.clone(),
        })
    }).collect()
}

fn register(n: &Node, vals: &Vals) -> Option<(String, String)> {
    let NodeKind::Prop(b, name) = &deref(n, vals).kind else {
        return None;
    };
    (name.starts_with('R') && name.contains('['))
        .then(|| Some((box_key(b, vals)?, name.clone())))
        .flatten()
}

// Only value/option expressions, never Boolean presence tests or nested
// comparisons: `SELF.R4.isDefined == out.R4.isDefined` does not carry R4.
fn register_operands(n: &Node, vals: &Vals, depth: u32, out: &mut HashSet<(String, String)>) {
    if depth == 0 {
        return;
    }
    let d = deref(n, vals);
    if let Some(r) = register(d, vals) {
        out.insert(r);
        return;
    }
    let descend = match &d.kind {
        NodeKind::Infix("+" | "-" | "*" | "/" | "%", ..) | NodeKind::Unary("-", _) => true,
        NodeKind::Method(_, name, _) => matches!(
            name.as_str(),
            "get" | "getOrElse" | "toLong" | "toInt" | "toBigInt"
        ),
        _ => false,
    };
    if descend {
        for c in children(d) {
            register_operands(c, vals, depth - 1, out);
        }
    }
}

fn scan(
    n: &Node,
    vals: &Vals,
    slots: &mut BTreeSet<String>,
    registers: &mut BTreeSet<String>,
    carried: &mut HashSet<(String, String)>,
) {
    if let Some((b, how)) = reserve_read(n, vals) {
        if box_key(b, vals).as_deref() == Some("SELF") {
            if let Some(slot) = how
                .strip_prefix(".tokens(")
                .and_then(|s| s.strip_suffix(")._2"))
            {
                slots.insert(slot.into());
            }
        }
    }
    if let NodeKind::Method(b, name, args) = &n.kind {
        if (name == "get" && args.is_empty()) || (name == "getOrElse" && args.len() == 1) {
            if let Some((key, r)) = register(b, vals) {
                if key == "SELF" {
                    registers.insert(r);
                }
            }
        }
    }
    if let NodeKind::Infix("==" | "<" | "<=" | ">" | ">=", a, b) = &n.kind {
        let mut reads = HashSet::new();
        register_operands(a, vals, 32, &mut reads);
        register_operands(b, vals, 32, &mut reads);
        for (key, r) in &reads {
            if key.starts_with("OUTPUTS(") && reads.contains(&("SELF".into(), r.clone())) {
                carried.insert((key.clone(), r.clone()));
            }
        }
    }
    for c in children(n) {
        scan(c, vals, slots, registers, carried);
    }
}
