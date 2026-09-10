//! Bounded discovery from supplied canonical material. Lifted syntax is observation,
//! never an exact-tree derivation or a statement about mandatory execution.
use super::discovery::{DiscoveryMap, DiscoveryVersion, MatchStatus, Observation};
use super::relations::Selector;
use crate::decompile::Stmt;
use crate::evidence::{
    case::{Premise, RecordedBox},
    wire::WireBox,
};
use crate::{Node, NodeKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Separate budgets for this supplied-material API; legacy traversal caps are untouched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caps {
    pub tree_bytes: usize,
    pub nodes: usize,
    pub boxes: usize,
    pub comparisons: usize,
}
impl Default for Caps {
    fn default() -> Self {
        Self {
            tree_bytes: 65536,
            nodes: 10000,
            boxes: 96,
            comparisons: 10000,
        }
    }
}
/// Assertions by the supplier, not a historical UTXO certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supply {
    pub records: Vec<RecordedBox>,
    pub queries: Vec<Value>,
    pub snapshot: Premise<Value>,
    pub expected_records: Option<usize>,
    pub missing_pages: Vec<String>,
    pub unsupported_queries: Vec<String>,
    pub inconsistencies: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteDisposition {
    pub id: String,
    pub status: MatchStatus,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryRun {
    pub map: DiscoveryMap,
    pub sites: Vec<SiteDisposition>,
}
#[derive(Clone)]
enum Identity {
    Id(String),
    Script(String),
    Hash(String),
    Token(Option<usize>, String),
}
#[derive(Clone)]
enum Binding<'a> {
    Alias(&'a Node),
    Box,
    Token,
}
type Env<'a> = BTreeMap<String, Binding<'a>>;
fn resolve<'a>(mut n: &'a Node, env: &Env<'a>) -> &'a Node {
    for _ in 0..32 {
        if let NodeKind::Val(v) = &n.kind {
            if let Some(Binding::Alias(a)) = env.get(v) {
                n = a;
                continue;
            }
        }
        break;
    }
    n
}
fn property<'a>(n: &'a Node, name: &str, env: &Env<'a>) -> Option<&'a Node> {
    match &resolve(n, env).kind {
        NodeKind::Prop(b, p) if p == name => Some(resolve(b, env)),
        NodeKind::Method(b, p, a) if p == name && a.is_empty() => Some(resolve(b, env)),
        _ => None,
    }
}
fn index<'a>(n: &'a Node, env: &Env<'a>) -> Option<(&'a Node, usize)> {
    match &resolve(n, env).kind {
        NodeKind::ApplyFn(b, a) if a.len() == 1 => match resolve(&a[0], env).kind {
            NodeKind::Int(i) if i >= 0 => Some((resolve(b, env), i as usize)),
            _ => None,
        },
        _ => None,
    }
}
fn collection(n: &Node) -> bool {
    matches!(
        &n.kind,
        NodeKind::Leaf("INPUTS" | "OUTPUTS" | "CONTEXT.dataInputs")
    ) || matches!(&n.kind,NodeKind::Method(b,p,a) if a.is_empty() && matches!(b.kind,NodeKind::Leaf("CONTEXT")) && p=="dataInputs")
        || matches!(&n.kind,NodeKind::Prop(b,p) if matches!(b.kind,NodeKind::Leaf("CONTEXT")) && p=="dataInputs")
}
fn box_receiver<'a>(n: &'a Node, env: &Env<'a>) -> bool {
    let n = resolve(n, env);
    if let NodeKind::Val(v) = &n.kind {
        return matches!(env.get(v), Some(Binding::Box));
    }
    index(n, env).is_some_and(|(b, _)| collection(b))
}
fn literal<'a>(n: &'a Node, env: &Env<'a>) -> Option<String> {
    let NodeKind::Const(s) = &resolve(n, env).kind else {
        return None;
    };
    let h = s.strip_prefix("fromBase16(\"")?.strip_suffix("\")")?;
    hex::decode(h).ok().map(hex::encode)
}
fn identity<'a>(lhs: &'a Node, rhs: &'a Node, env: &Env<'a>) -> Option<Identity> {
    let h = literal(rhs, env)?;
    if property(lhs, "id", env).is_some_and(|b| box_receiver(b, env)) && h.len() == 64 {
        return Some(Identity::Id(h));
    }
    if property(lhs, "propositionBytes", env).is_some_and(|b| box_receiver(b, env)) {
        return Some(Identity::Script(h));
    }
    if let NodeKind::Global(f, args) = &resolve(lhs, env).kind {
        if f == "blake2b256"
            && args.len() == 1
            && h.len() == 64
            && property(&args[0], "propositionBytes", env).is_some_and(|b| box_receiver(b, env))
        {
            return Some(Identity::Hash(h));
        }
    }
    if h.len() == 64 {
        if let Some(t) = property(lhs, "_1", env) {
            if let Some((tokens, i)) = index(t, env) {
                if property(tokens, "tokens", env).is_some_and(|b| box_receiver(b, env)) {
                    return Some(Identity::Token(Some(i), h));
                }
            }
            if let NodeKind::Val(v) = &t.kind {
                if matches!(env.get(v), Some(Binding::Token)) {
                    return Some(Identity::Token(None, h));
                }
            }
        }
    }
    None
}
struct Scan {
    refs: Vec<(u64, Identity, bool)>,
    reasons: BTreeSet<String>,
    remaining: usize,
}
fn walk<'a>(n: &'a Node, env: &Env<'a>, scan: &mut Scan) {
    if scan.remaining == 0 {
        scan.reasons.insert("node-cap".into());
        return;
    }
    scan.remaining -= 1;
    match &n.kind {
        NodeKind::Block(stmts, result) => {
            let mut local = env.clone();
            for s in stmts {
                match s {
                    Stmt::Val(v, x) => {
                        walk(x, &local, scan);
                        local.insert(v.clone(), Binding::Alias(x));
                    }
                    Stmt::Def(_, x) => {
                        walk(x, &local, scan);
                        scan.reasons.insert("function-alias-unsupported".into());
                    }
                }
            }
            walk(result, &local, scan);
            return;
        }
        NodeKind::Method(receiver, method, args) if method == "exists" && args.len() == 1 => {
            if let NodeKind::Lambda(params, body) = &args[0].kind {
                if params.len() == 1 {
                    let binding = if matches!(resolve(receiver, env).kind, NodeKind::Leaf("INPUTS"))
                    {
                        Some(Binding::Box)
                    } else if property(receiver, "tokens", env)
                        .is_some_and(|b| box_receiver(b, env))
                    {
                        Some(Binding::Token)
                    } else {
                        None
                    };
                    if let Some(binding) = binding {
                        let mut local = env.clone();
                        local.insert(params[0].split(':').next().unwrap().trim().into(), binding);
                        walk(body, &local, scan);
                        return;
                    }
                }
            }
            scan.reasons
                .insert("existential-receiver-or-scope-unsupported".into());
        }
        NodeKind::Lambda(..) => {
            scan.reasons
                .insert("lambda-outside-supported-exists".into());
            return;
        }
        NodeKind::Infix("==", a, b) => {
            let found = identity(a, b, env).or_else(|| identity(b, a, env));
            if let Some(i) = found {
                scan.refs.push((n.id, i, false));
            } else {
                scan.reasons
                    .insert("nonliteral-or-unsupported-identity".into());
                // Untyped register constants remain visibly weak token hints. Never
                // reinterpret a computed box index or AVL lookup as an identity.
                for (field, value) in [(&**a, &**b), (&**b, &**a)] {
                    if let NodeKind::Method(reg, m, args) = &resolve(field, env).kind {
                        if m == "get"
                            && args.is_empty()
                            && matches!(&reg.kind,NodeKind::Prop(_,p) if p.starts_with('R') && p.contains("Coll[Byte]"))
                        {
                            if let Some(h) = literal(value, env).filter(|h| h.len() == 64) {
                                scan.refs.push((n.id, Identity::Token(None, h), true));
                            }
                        }
                    }
                }
            }
        }
        NodeKind::ApplyFn(b, _) if collection(resolve(b, env)) && index(n, env).is_none() => {
            scan.reasons.insert("computed-index-unsupported".into());
        }
        NodeKind::Prop(_, p) if p.contains("AvlTree") => {
            scan.reasons.insert("avl-identity-unsupported".into());
        }
        NodeKind::Raw(_) => {
            scan.reasons.insert("unsupported-lift-node".into());
        }
        _ => {}
    }
    for c in crate::audit::children(n) {
        walk(c, env, scan);
    }
}
fn matching(identity: &Identity, b: &WireBox) -> Vec<Selector> {
    let node = b.node();
    let tree = hex::encode(node.candidate.ergo_tree_bytes());
    match identity {
        Identity::Id(h) if b.id().as_ref() == Ok(h) => vec![Selector::BoxId { hex: h.clone() }],
        Identity::Script(h) if &tree == h => vec![Selector::PropositionBytes { hex: h.clone() }],
        Identity::Hash(h)
            if hex::encode(
                ergo_primitives::digest::blake2b256(node.candidate.ergo_tree_bytes()).as_bytes(),
            ) == *h =>
        {
            vec![Selector::PropositionHash { hex: h.clone() }]
        }
        Identity::Token(position, h) => node
            .candidate
            .tokens
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                position.is_none_or(|p| p == *i) && hex::encode(t.token_id.as_bytes()) == *h
            })
            .map(|(i, t)| match position {
                Some(_) => Selector::TokenAt {
                    index: i as u32,
                    id: h.clone(),
                    amount: t.amount,
                },
                None => Selector::TokenMember {
                    id: h.clone(),
                    amount: t.amount,
                },
            })
            .collect(),
        _ => vec![],
    }
}
/// Discover only within supplied records, from this one root. No network queries,
/// action construction, transitive necessity, or simultaneous-availability claim.
/// Token selector amounts describe the observed target, not a constraint extracted
/// from an ID-only equality. Optional/negated predicates are still observations.
pub fn discover(
    root: &[u8],
    subject: &str,
    sites: &[Site],
    supply: &Supply,
    caps: &Caps,
) -> DiscoveryRun {
    let mut reasons = BTreeSet::new();
    for (prefix, items) in [
        ("missing-page", &supply.missing_pages),
        ("unsupported-query", &supply.unsupported_queries),
        ("source-inconsistency", &supply.inconsistencies),
    ] {
        for item in items {
            reasons.insert(format!("{prefix}: {item}"));
        }
    }
    if supply.expected_records != Some(supply.records.len()) {
        reasons.insert("unknown-or-incomplete-inventory".into());
    }
    if supply.snapshot.value().is_none() {
        reasons.insert("unknown-snapshot".into());
    }
    if supply.records.len() > caps.boxes {
        reasons.insert("box-cap".into());
    }
    let mut boxes = BTreeMap::new();
    for record in supply.records.iter().take(caps.boxes) {
        match WireBox::from_record(record.clone()) {
            Ok(b) => {
                let id = b.id().expect("validated box");
                if boxes.insert(id, b).is_some() {
                    reasons.insert("source-inconsistency: duplicate box ID".into());
                }
            }
            Err(e) => {
                reasons.insert(format!("source-inconsistency: {e}"));
            }
        }
    }
    if boxes
        .get(subject)
        .is_none_or(|b| b.node().candidate.ergo_tree_bytes() != root)
    {
        reasons.insert("source-inconsistency: subject/root mismatch or missing subject".into());
    }
    let mut scan = Scan {
        refs: vec![],
        reasons: BTreeSet::new(),
        remaining: caps.nodes,
    };
    if root.len() > caps.tree_bytes {
        reasons.insert("tree-byte-cap".into());
    } else {
        match crate::inspect::parse_tree(root) {
            Ok(t) => walk(&crate::lift_tree(&t, true).node, &Env::new(), &mut scan),
            Err(e) => {
                reasons.insert(format!("parse-failure: {e}"));
            }
        }
    }
    reasons.extend(scan.reasons);
    // An incomplete walk cannot silently emit a successful recovered prefix.
    let blocked = reasons.iter().any(|r| {
        r.contains("cap") || r.starts_with("source-inconsistency") || r.starts_with("parse-failure")
    });
    let mut observations = vec![];
    let mut comparisons = 0;
    for (site, identity, hint) in scan.refs {
        for (id, b) in &boxes {
            if comparisons == caps.comparisons {
                reasons.insert("comparison-cap".into());
                break;
            }
            comparisons += 1;
            if id == subject {
                continue;
            }
            for reference in matching(&identity, b) {
                observations.push(Observation{
                    id:format!("{subject}/candidate/{}",observations.len()),source_box:subject.into(),site:format!("lift/{site}"),reference,proposed_targets:vec![id.clone()],proposed_roles:vec![],
                    status:if blocked {MatchStatus::Unresolved} else if hint {MatchStatus::Hint} else {MatchStatus::ObservedMatch},
                    reason:if hint {"untyped register constant token collision; candidate only"} else {"literal identity observed in supplied canonical bytes; token amount, if present, is observed; no necessity or execution authority"}.into(),
                });
            }
        }
    }
    if reasons.contains("comparison-cap") {
        for o in &mut observations {
            o.status = MatchStatus::Unresolved;
            o.reason = "comparison-cap; incomplete discovery".into();
        }
    }
    let dispositions=sites.iter().map(|s|SiteDisposition{id:s.id.clone(),status:MatchStatus::Unresolved,reason:
        if s.text.contains("AvlTree") {"avl-identity-unsupported"}
        else if s.text.contains("INPUTS(getVar") {"computed-index-unsupported"}
        else {"lexical inventory anchor has no exact lift correspondence; independent lift observations retained, no proof authority"}.into()}).collect();
    DiscoveryRun {
        map: DiscoveryMap {
            version: DiscoveryVersion::V2,
            seeds: vec![json!({"boxId":subject})],
            recorded_boxes: supply.records.clone(),
            canonical_references: boxes
                .iter()
                .map(|(id, b)| (id.clone(), hex::encode(b.bytes().expect("validated box"))))
                .collect(),
            observed_code: BTreeMap::from([(subject.into(), hex::encode(root))]),
            observations,
            source_queries: supply.queries.clone(),
            snapshot_scope: supply.snapshot.clone(),
            caps: BTreeMap::from([
                ("treeBytes".into(), caps.tree_bytes as u64),
                ("nodes".into(), caps.nodes as u64),
                ("boxes".into(), caps.boxes as u64),
                ("comparisons".into(), caps.comparisons as u64),
            ]),
            unresolved_reasons: reasons.into_iter().collect(),
        },
        sites: dispositions,
    }
}
