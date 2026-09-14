//! Register-selected output code with a syntactically mutable continuation.
//!
//! A sibling keeps trust-assumptions' existing provenance recogniser unchanged.
//! We follow result aliases, sigmaProp, AND/OR, allOf/anyOf and if branches,
//! bounded to 64 alternatives and depth 128. An output digest/bytes comparison
//! names the hook. A same-script continuation of its source
//! box must carry the same typed register by direct equality on that path.
//! Only those recognised continuations are reported, once per source/register.
//!
//! LOW observations: no reachability, signature satisfiability, token supply,
//! companion-script enforcement or exploitability is established. Computed
//! boxes, arbitrary predicates, dynamic registers and over-cap expressions are
//! undecided. An upgrade destination alone is not assumed to be a continuation.
use crate::audit::boxrefs::{box_key, collect_vals, deref, script_of, Vals};
use crate::audit::{snippet, Finding, Severity};
use crate::{Node, NodeKind};
use std::collections::{BTreeMap, BTreeSet};

type Paths<'a> = Vec<Vec<&'a Node>>;

#[must_use]
pub fn upgrade_hook(root: &Node) -> Vec<Finding> {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let Some(paths) = paths(root, &vals, 128) else {
        return vec![];
    };
    let mut hooks = BTreeMap::new();
    for path in &paths {
        for n in path {
            if let NodeKind::Infix("==", a, b) = &n.kind {
                for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
                    if output_script(lhs, &vals).is_some() {
                        if let Some((source, register)) = register(rhs, &vals) {
                            if source == "SELF"
                                || source.starts_with("INPUTS(")
                                || source.starts_with("CONTEXT.dataInputs(")
                            {
                                hooks.entry((source, register)).or_insert(*n);
                            }
                        }
                    }
                }
            }
        }
    }
    hooks.into_iter().filter_map(|((source, reg), site)| {
        let mut rewrites = BTreeSet::new();
        for path in &paths {
            let mut successors = BTreeSet::new();
            let mut carried = BTreeSet::new();
            for n in path {
                if let NodeKind::Infix("==", a, b) = &n.kind {
                    for (lhs, rhs) in [(&**a, &**b), (&**b, &**a)] {
                        let scripts = script_of(lhs, &vals).and_then(|b| box_key(b, &vals))
                            .zip(script_of(rhs, &vals).and_then(|b| box_key(b, &vals)));
                        if let Some((out, from)) = scripts {
                            if out.starts_with("OUTPUTS(") && from == source { successors.insert(out); }
                        }
                        if let (Some((out, r)), Some((from, original))) = (register(lhs, &vals), register(rhs, &vals)) {
                            if from == source && r == reg && original == reg { carried.insert(out); }
                        }
                    }
                }
            }
            for out in successors.difference(&carried) {
                let guards: BTreeSet<_> = path.iter().filter_map(|n| sigma_guard(n, &vals)).collect();
                rewrites.insert(format!("{out}: {}", if guards.is_empty() { "unguarded (no recognised sigma guard on this path)".into() } else { format!("sigma guard {}", guards.into_iter().collect::<Vec<_>>().join(" AND ")) }));
            }
        }
        if rewrites.is_empty() { return None; }
        Some(Finding {
            triage: Default::default(), lint: "upgrade-hook", severity: Severity::Low,
            node_id: site.id, ir_id: None, snippet: snippet(site),
            message: format!("{source}.{reg} selects output script bytes/hash; a recognised continuation does not require its corresponding register to equal {source}'s. Rewrite-path authority: {}. Static review observation; reachability, other contracts and intended upgrade authority need review. This does not establish an exploitable upgrade.", rewrites.into_iter().collect::<Vec<_>>().join("; ")),
        })
    }).collect()
}

fn register(n: &Node, vals: &Vals) -> Option<(String, String)> {
    let mut d = deref(n, vals);
    if let NodeKind::Method(b, name, args) = &d.kind {
        if name == "get" && args.is_empty() {
            d = deref(b, vals);
        }
    }
    if let NodeKind::Prop(b, name) = &d.kind {
        if name.starts_with('R') && name.ends_with("[Coll[Byte]]") {
            return Some((box_key(b, vals)?, name.clone()));
        }
    }
    None
}

fn output_script(n: &Node, vals: &Vals) -> Option<String> {
    let mut d = deref(n, vals);
    if let NodeKind::Global(name, args) = &d.kind {
        if name == "blake2b256" && args.len() == 1 {
            d = deref(&args[0], vals);
        }
    }
    box_key(script_of(d, vals)?, vals).filter(|k| k.starts_with("OUTPUTS("))
}

fn sigma_guard(n: &Node, vals: &Vals) -> Option<String> {
    let d = deref(n, vals);
    match &d.kind {
        NodeKind::Const(s)
            if s.starts_with("PK(")
                || s.starts_with("proveDlog(")
                || s.starts_with("proveDHTuple(") =>
        {
            Some(s.clone())
        }
        NodeKind::Global(name, args)
            if matches!(name.as_str(), "proveDlog" | "proveDHTuple" | "PK") =>
        {
            Some(format!(
                "{name}({})",
                args.iter()
                    .map(|a| anchor_name(a, vals))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        NodeKind::Method(b, name, args) if name == "get" && args.is_empty() => {
            if let NodeKind::Prop(receiver, register) = &deref(b, vals).kind {
                if register.ends_with("[SigmaProp]") {
                    return Some(format!("{}.{register}.get", box_key(receiver, vals)?));
                }
            }
            None
        }
        NodeKind::AtLeast(_, _) => Some(snippet(d)),
        _ => None,
    }
}
fn anchor_name(n: &Node, vals: &Vals) -> String {
    let d = deref(n, vals);
    match &d.kind {
        NodeKind::Method(b, name, args) if args.is_empty() => {
            format!("{}.{name}", anchor_name(b, vals))
        }
        NodeKind::Prop(b, name) => format!(
            "{}.{name}",
            box_key(b, vals).unwrap_or_else(|| anchor_name(b, vals))
        ),
        _ => snippet(d),
    }
}

fn product<'a>(a: Paths<'a>, b: Paths<'a>) -> Option<Paths<'a>> {
    if a.len().saturating_mul(b.len()) > 64 {
        return None;
    }
    Some(
        a.into_iter()
            .flat_map(|left| {
                b.iter()
                    .map(move |right| left.iter().chain(right).copied().collect())
            })
            .collect(),
    )
}
fn paths<'a>(n: &'a Node, vals: &Vals<'a>, depth: u32) -> Option<Paths<'a>> {
    if depth == 0 {
        return None;
    }
    let d = deref(n, vals);
    let recurse = |n| paths(n, vals, depth - 1);
    match &d.kind {
        NodeKind::Block(_, result) => recurse(result),
        NodeKind::Bool(false) => Some(vec![]),
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => recurse(&args[0]),
        NodeKind::Infix("&&", a, b) => product(recurse(a)?, recurse(b)?),
        NodeKind::Infix("||", a, b) => {
            let mut p = recurse(a)?;
            p.extend(recurse(b)?);
            (p.len() <= 64).then_some(p)
        }
        NodeKind::If(c, t, e) => {
            let mut p = product(recurse(c)?, recurse(t)?)?;
            // Negative condition is not a positive equality/authority guard.
            p.extend(recurse(e)?);
            (p.len() <= 64).then_some(p)
        }
        NodeKind::Global(name, args)
            if matches!(name.as_str(), "allOf" | "anyOf") && args.len() == 1 =>
        {
            let NodeKind::Coll(_, items) = &deref(&args[0], vals).kind else {
                return None;
            };
            let mut p = if name == "allOf" {
                vec![vec![]]
            } else {
                vec![]
            };
            for item in items {
                let q = recurse(item)?;
                if name == "allOf" {
                    p = product(p, q)?;
                } else {
                    p.extend(q);
                }
                if p.len() > 64 {
                    return None;
                }
            }
            Some(p)
        }
        _ => Some(vec![vec![d]]),
    }
}
