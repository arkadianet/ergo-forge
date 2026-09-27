//! Bounded, fail-closed flow from spender-controlled sources to
//! security-sensitive sinks in the lifted AST.
//!
//! The question this answers is narrow and syntactic: **which values the spender
//! chooses reach a place where the script's security meaning is decided?** It is
//! a review obligation, never a claim that any path is exploitable.
//!
//! # Vocabulary
//!
//! A **source** is a node the analysis positively names as something the
//! spender assembles: a context variable (`getVar[T](id)`, the bytes behind
//! `executeFromVar` included), any property or register of a spender-chosen
//! `INPUTS(i)` / `OUTPUTS(i)` / `CONTEXT.dataInputs(i)` box, the `INPUTS` and
//! `OUTPUTS` collections themselves, a lambda parameter (an element of a
//! collection the spender chose), an unresolved `val` reference, or a
//! `NodeKind::Raw` placeholder.
//!
//! A **sink** is a site whose value decides something security-relevant:
//!
//! - [`SinkKind::CodeBytes`] — dynamic code or code-template bytes;
//! - [`SinkKind::ReserveGuard`] — a comparison that orders or equals `SELF`'s
//!   reserves (`.value`, `.tokens(i)._2`);
//! - [`SinkKind::IdentityGuard`] — a spender value as the other side of an
//!   equality with a `SELF`-rooted value (successor script, box id, register);
//! - [`SinkKind::SelfGuard`] — a spender value ordering a `SELF`-rooted value;
//! - [`SinkKind::CollectionIndex`] — a spender value selecting a collection
//!   element or a register;
//! - [`SinkKind::Unresolved`] — a hard tree bound stopped the search before a
//!   site could be named.
//!
//! `SELF`, `HEIGHT` and the protocol constants are *not* sources: the box being
//! spent, the chain and the protocol fix them. `OUTPUTS(i)` *is* treated as
//! spender-supplied, because the spender assembles the outputs and only the
//! script can constrain them.
//!
//! # Fail-closed
//!
//! Every construct the analysis does not model taints, and never sanitises:
//!
//! - a `Raw` placeholder and an unresolved binding carry a named `unmodelled`
//!   provenance;
//! - a shape with no recognised source classification takes the union of its
//!   operands, and `children` is exhaustive over `NodeKind`, so an unmodelled
//!   variant still reaches its sub-expressions instead of laundering one away;
//! - a search that runs out of depth returns taint with [`DEPTH_LIMITED`]
//!   rather than a clean answer, and a backtrace that cannot name a source
//!   within its bound reports an explicit `unresolved-bounded-source` flow;
//! - a comparison whose `SELF` anchor cannot be ruled out within the depth
//!   bound is still treated as a `SELF` guard.
//!
//! Nothing here decides whether a value is *constrained*, only where a
//! spender-chosen value *arrives*. Guards in an unreachable branch, under
//! negation, or on a path the script never takes all count, exactly as they do
//! for the sibling lints.
//!
//! # Bounds
//!
//! Every answer is bounded by [`Bounds`], and every ceiling that was reached is
//! reported in [`Limits`] rather than quietly truncating to a clean result.
//! Absence of a flow under a bound is not evidence of absence.
//!
//! Known limits (deliberate): only `SELF` is treated as a security anchor, so a
//! spender-chosen value that gates on `HEIGHT` alone is not reported. No type
//! information is used, so a comparison between two spender values, or between a
//! spender value and a literal, is not a sink. Sink operands are traced to the
//! *nearest* named source per branch, not to every ancestor. Sign, direction,
//! reachability, token supply, signatures, AVL proofs and economic intent are
//! all undecided.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use serde::Serialize;

use crate::audit::boxrefs::{
    as_indexed, box_collection, box_key, collect_vals, deref, reads_in_operand, tokens_receiver,
    Vals, OPERAND_DEPTH,
};
use crate::audit::children;
use crate::{Node, NodeKind};

/// Provenance label for a construct with no modelled source.
const UNMODELLED: &str = "unmodelled";

/// Provenance label for an expression the depth ceiling stopped the search in.
const DEPTH_LIMITED: &str = "depth-limited";

/// Source label for a backtrace that ran out of bound before naming a source.
const UNRESOLVED: &str = "unresolved-bounded-source";

/// Hard recursion ceiling for a caller-supplied AST. Parsed trees are far
/// below this; a hand-built tree beyond it is reported as unresolved rather
/// than walked recursively.
pub const MAX_AST_DEPTH: u32 = 256;

/// Hard ceilings for one analysis run.
///
/// A bound is a stopping point, never a filter: hitting one sets the matching
/// [`Limits`] flag instead of quietly dropping evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Maximum expression nesting followed while classifying a value, and the
    /// maximum number of value-dependency steps a source→sink trace may take.
    pub max_depth: u32,
    /// Maximum distinct source names retained per expression, and maximum
    /// flows reported for one sink. Truncation keeps the lexicographically
    /// smallest names so two runs over one tree cannot disagree.
    pub max_alternatives: usize,
    /// Maximum sink sites recorded. Sinks past this are never examined.
    pub max_sinks: usize,
    /// Maximum flows reported in total.
    pub max_observations: usize,
}

impl Default for Bounds {
    /// Tuned so the common real contract is answered outright and the bounds
    /// act as a safety net: a large pool contract still analyses in single-digit
    /// milliseconds, and the corpus only reaches `max_sinks` on the biggest
    /// trees. Raise them for exhaustive review; lower them when a bound must be
    /// provable rather than merely declared.
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_alternatives: 4,
            max_sinks: 256,
            max_observations: 256,
        }
    }
}

/// Which ceiling, if any, stopped the analysis short of a decided answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    /// A depth or trace ceiling was reached before a value was classified.
    pub depth: bool,
    /// Some expression carried more sources than `max_alternatives` retained.
    pub alternatives: bool,
    /// The sink budget filled; later sinks were not examined at all.
    pub sinks: bool,
    /// A flow budget filled; later sinks were left untraced.
    pub observations: bool,
}

impl Limits {
    /// Did any ceiling apply? Then the answer is a lower bound, not a clean bill.
    #[must_use]
    pub fn any(self) -> bool {
        self.depth || self.alternatives || self.sinks || self.observations
    }

    /// The ceilings that applied, in reviewer's words, or `None` if none did.
    #[must_use]
    pub fn note(self) -> Option<String> {
        if !self.any() {
            return None;
        }
        let mut parts = Vec::new();
        if self.depth {
            parts.push("a depth or trace ceiling was reached");
        }
        if self.alternatives {
            parts.push("some values carried more sources than the budget retained");
        }
        if self.sinks {
            parts.push("the sink budget filled before the tree was exhausted");
        }
        if self.observations {
            parts.push("a flow budget filled before every sink was traced");
        }
        Some(parts.join("; "))
    }
}

/// What a security-sensitive site does with the value it consumes.
///
/// Declaration order is review priority order, most severe first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SinkKind {
    /// Dynamic code or code-template bytes fed to an interpreter.
    CodeBytes,
    /// A spender value compared or ordered against `SELF`'s reserves.
    ReserveGuard,
    /// A spender value as the other side of a `SELF`-rooted equality.
    IdentityGuard,
    /// A spender value ordering a `SELF`-rooted value.
    SelfGuard,
    /// A spender value selecting a collection element or a register.
    CollectionIndex,
    /// The analysis reached a hard AST bound before it could name a site.
    Unresolved,
}

impl SinkKind {
    /// Stable machine-readable name, for callers that key on the class.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::CodeBytes => "code-bytes",
            Self::ReserveGuard => "reserve-guard",
            Self::IdentityGuard => "identity-guard",
            Self::SelfGuard => "self-guard",
            Self::CollectionIndex => "collection-index",
            Self::Unresolved => "unresolved",
        }
    }
}

/// One spender-controlled source reaching one security-sensitive sink.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    /// The sink class; see [`SinkKind`].
    pub kind: SinkKind,
    /// Lift-local id of the sink site a finding anchors to
    /// (`ast::Node::id`, stable within one decompilation).
    pub site_id: u64,
    /// What the sink consumes, in reviewer's words, e.g. `"SELF.value"`.
    pub sink: String,
    /// The named spender-controlled source, or [`UNRESOLVED`].
    pub source: String,
    /// Lift-local id of the node the source label was derived from, when one
    /// was reached inside the trace bound.
    pub source_id: Option<u64>,
    /// Value-dependency steps between the sink's operand and the source.
    pub hops: usize,
    /// That chain's node ids, sink operand first, source last.
    pub path: Vec<u64>,
    /// Set when a bound, not the tree, ended the trace — the source label may
    /// understate what reaches the sink.
    pub bounded: bool,
}

/// The result of one bounded run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Deduplicated source→sink observations, in sink-then-distance order.
    pub flows: Vec<Flow>,
    /// Sink records recorded, up to two per comparison site. When
    /// `limits.sinks` is true this is the configured sink budget; trust the
    /// limit flag rather than inferring truncation from this count.
    pub sinks: usize,
    /// Named source nodes found anywhere in the tree.
    pub sources: usize,
    /// Which ceilings, if any, applied.
    pub limits: Limits,
}

/// What the analysis could establish about one expression.
///
/// `sources.is_empty() && !unknown` is the *only* claim of "not derived from
/// anything the spender chose". Every other state is an obligation to review.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Taint {
    /// Named sources, sorted and deduplicated.
    sources: BTreeSet<String>,
    /// Set when naming a source was dropped to stay inside `max_alternatives`.
    alternatives_dropped: bool,
    /// Set when the taint is a placeholder for provenance this analysis does
    /// not model, rather than a positively identified source.
    unknown: bool,
}

impl Taint {
    fn named(label: &str) -> Self {
        Self {
            sources: BTreeSet::from([label.to_owned()]),
            alternatives_dropped: false,
            unknown: false,
        }
    }

    fn tainted(label: &str) -> Self {
        Self {
            unknown: true,
            ..Self::named(label)
        }
    }

    fn is_tainted(&self) -> bool {
        !self.sources.is_empty() || self.unknown
    }

    /// Union in `other`, keeping at most `cap` names.
    ///
    /// Truncation is deterministic — the lexicographically smallest names
    /// survive — and always leaves the taint itself intact, so a dropped name
    /// can never turn a tainted value into a clean one.
    fn absorb(&mut self, other: Taint, cap: usize) {
        self.unknown |= other.unknown;
        self.alternatives_dropped |= other.alternatives_dropped;
        if other.sources.is_empty() {
            return;
        }
        if self.sources.len().saturating_add(other.sources.len()) > cap {
            self.alternatives_dropped = true;
            // A value whose every name was dropped is not thereby clean.
            self.unknown = true;
            let mut all = std::mem::take(&mut self.sources);
            all.extend(other.sources);
            self.sources = all.into_iter().take(cap).collect();
        } else {
            self.sources.extend(other.sources);
        }
    }
}

/// A node's own classification as a spender-controlled source.
struct Origin {
    /// Reviewer-facing name, e.g. `"INPUTS(0).R4[Long]"`.
    label: String,
    /// Set when the label stands in for provenance this analysis does not
    /// model (`Raw`, an unresolved binding, an unrecognised leaf).
    unmodelled: bool,
}

/// Per-tree read model: `val` bindings and lambda parameter names.
struct Env<'a> {
    vals: Vals<'a>,
    params: HashSet<String>,
}

/// Bottom-up taint and source labels, keyed by lift-local node id.
#[derive(Default)]
struct Measured {
    taint: HashMap<u64, Taint>,
    origins: HashMap<u64, String>,
}

/// A security-sensitive site and the sub-expression whose value it consumes.
struct Sink<'a> {
    kind: SinkKind,
    /// The value the sink consumes.
    operand: &'a Node,
    /// The node a finding anchors to.
    site: &'a Node,
    /// Provenance the sink consumes by construction, when it has any
    /// (`executeFromVar` reads context-variable bytes whatever its id spells).
    supplied: Option<Taint>,
    /// What the sink consumes, in reviewer's words.
    label: String,
}

/// One traced chain from a sink's operand back to a named source.
struct Trace<'a> {
    origin: Option<&'a Node>,
    label: String,
    path: Vec<u64>,
    bounded: bool,
}

/// Run the bounded flow analysis with the default bounds.
#[must_use]
pub fn analyze(root: &Node) -> Report {
    analyze_with(root, Bounds::default())
}

fn exceeds_depth(root: &Node, max_depth: u32) -> bool {
    let mut stack = vec![(root, 0u32)];
    while let Some((node, depth)) = stack.pop() {
        if depth > max_depth {
            return true;
        }
        stack.extend(children(node).into_iter().map(|child| (child, depth + 1)));
    }
    false
}

/// Run the bounded flow analysis.
///
/// Total: cannot fail. A tree this analysis cannot read is reported through
/// [`Report::limits`], never by panicking and never by reporting clean.
#[must_use]
pub fn analyze_with(root: &Node, bounds: Bounds) -> Report {
    let mut vals = Vals::new();
    collect_vals(root, &mut vals);
    let mut params = HashSet::new();
    collect_params(root, &mut params);
    let env = Env { vals, params };

    let mut limits = Limits::default();
    if exceeds_depth(root, MAX_AST_DEPTH) {
        limits.depth = true;
        return Report {
            flows: vec![Flow {
                kind: SinkKind::Unresolved,
                site_id: root.id,
                sink: "unresolved tree depth".to_owned(),
                source: UNRESOLVED.to_owned(),
                source_id: None,
                hops: 0,
                path: vec![root.id],
                bounded: true,
            }],
            sinks: 1,
            sources: 0,
            limits,
        };
    }
    let mut measured = Measured::default();
    measure(
        root,
        &env,
        bounds,
        bounds.max_depth,
        &mut measured,
        &mut limits,
    );

    let mut sinks = Vec::new();
    collect_sinks(
        root,
        &env,
        &measured,
        bounds,
        &mut sinks,
        &mut limits,
        &mut HashMap::new(),
    );

    let mut flows: Vec<Flow> = Vec::new();
    let mut seen: BTreeSet<(u64, SinkKind, String)> = BTreeSet::new();
    for sink in &sinks {
        if flows.len() >= bounds.max_observations {
            limits.observations = true;
            break;
        }
        let mut traces = Vec::new();
        trace(sink, &env, &measured, bounds, &mut traces, &mut limits);
        for found in traces {
            if flows.len() >= bounds.max_observations {
                limits.observations = true;
                break;
            }
            if !seen.insert((sink.site.id, sink.kind, found.label.clone())) {
                continue;
            }
            flows.push(Flow {
                kind: sink.kind,
                site_id: sink.site.id,
                sink: sink.label.clone(),
                source: found.label,
                source_id: found.origin.map(|n| n.id),
                hops: found.path.len().saturating_sub(1),
                path: found.path,
                bounded: found.bounded,
            });
        }
        if limits.observations {
            break;
        }
    }

    Report {
        flows,
        sinks: sinks.len(),
        sources: measured.origins.len(),
        limits,
    }
}

// ── read model ─────────────────────────────────────────────────────────────────

/// Lambda parameters, by source-form name. The lift gives them `name: Type`
/// spellings, and a parameter stands for an element of a collection the
/// spender chose.
fn collect_params(n: &Node, out: &mut HashSet<String>) {
    if let NodeKind::Lambda(params, _) = &n.kind {
        for p in params {
            out.insert(bare_name(p).to_owned());
        }
    }
    for c in children(n) {
        collect_params(c, out);
    }
}

/// The source-form name of a lift-generated binding or parameter.
fn bare_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name).trim()
}

/// A method or global name without its source-form type arguments.
fn without_type_args(name: &str) -> &str {
    name.split('[').next().unwrap_or(name)
}

/// Nodes whose value contributes to `n`'s: its operands plus, for a `val`
/// reference, the expression the name stands for.
fn deps<'a>(n: &'a Node, env: &Env<'a>) -> Vec<&'a Node> {
    let d = deref(n, &env.vals);
    let mut out: Vec<&'a Node> = Vec::new();
    if !std::ptr::eq(d, n) {
        out.push(d);
    }
    out.extend(children(d));
    out
}

/// Is this box one the spender chose? `SELF` and everything unnamed is not.
fn spender_box(key: &str) -> bool {
    key.starts_with("INPUTS(")
        || key.starts_with("OUTPUTS(")
        || key.starts_with("CONTEXT.dataInputs(")
}

/// The path a `Prop` reads off a box, for the two receiver shapes a plain box
/// key cannot see: a direct box reference, and a token-slot projection of one.
///
/// Returns the box's key and the path below it — `"R4[Long]"`, or
/// `"tokens(0)._1"` — or `None` for every other receiver, including a computed
/// index, which this analysis does not follow.
fn box_relative(box_expr: &Node, name: &str, vals: &Vals) -> Option<(String, String)> {
    if let Some(key) = box_key(box_expr, vals) {
        return Some((key, name.to_owned()));
    }
    let (collection, index) = as_indexed(deref(box_expr, vals))?;
    let key = box_key(tokens_receiver(collection, vals)?, vals)?;
    Some((
        key,
        format!(
            "tokens({}).{name}",
            crate::decompile::print(deref(index, vals))
        ),
    ))
}

// ── source classification ─────────────────────────────────────────────────────

/// `n`'s own classification as spender-controlled, if it names one.
///
/// This is the only place a node may become a source; everything else is
/// classified as the union of its operands.
fn self_origin(d: &Node, env: &Env) -> Option<Origin> {
    let named = |label: String, unmodelled: bool| Some(Origin { label, unmodelled });
    match &d.kind {
        NodeKind::GetVar(id, tpe) => named(format!("getVar[{id}]:{tpe}"), false),
        NodeKind::Leaf(name) => match *name {
            // Fixed by the box being spent, by consensus, or by the protocol:
            // the spender chooses none of them.
            "SELF"
            | "HEIGHT"
            | "Global"
            | "groupGenerator"
            | "MinerPubkey"
            | "LastBlockUtxoRootHash"
            | "true"
            | "false" => None,
            "INPUTS" | "OUTPUTS" => named((*name).to_owned(), false),
            // `CONTEXT` and any leaf this analysis does not name: the spender
            // assembles the transaction its contents come from.
            other => named(other.to_owned(), true),
        },
        // Any property of a spender-chosen box, registers included: `R4[Long]`
        // and `.value` are both values the spender wrote.
        NodeKind::Prop(box_expr, name) => {
            let (key, path) = box_relative(box_expr, name, &env.vals)?;
            spender_box(&key).then(|| Origin {
                label: format!("{key}.{path}"),
                unmodelled: false,
            })
        }
        NodeKind::Raw(_) => named(UNMODELLED.into(), true),
        NodeKind::Val(name) => {
            // `deref` left the name unresolved, so it is either a lambda
            // parameter or a binding the tree does not define. Neither is
            // provenance this analysis can account for.
            if env.params.contains(bare_name(name)) {
                named(format!("lambda-parameter:{}", bare_name(name)), false)
            } else {
                named(format!("unbound-binding:{name}"), true)
            }
        }
        _ => None,
    }
}

// ── taint ─────────────────────────────────────────────────────────────────────

/// Classify the tree bottom-up, recording each node's taint and the nodes that
/// are sources in their own right.
fn measure(
    n: &Node,
    env: &Env,
    bounds: Bounds,
    depth: u32,
    out: &mut Measured,
    limits: &mut Limits,
) {
    let d = match &n.kind {
        NodeKind::Val(name) => env.vals.get(name).copied().unwrap_or(n),
        _ => n,
    };
    for c in children(n) {
        measure(c, env, bounds, depth.saturating_sub(1), out, limits);
    }
    let taint = if std::ptr::eq(d, n) {
        classify(d, env, bounds, depth, out, limits)
    } else {
        // `val` indirection: the use stands for the bound expression, whose
        // taint the binding site has already recorded. An out-of-order
        // binding is reported as unresolved rather than assumed clean.
        out.taint.get(&d.id).cloned().unwrap_or_else(|| {
            limits.depth = true;
            Taint::tainted(UNRESOLVED)
        })
    };
    if std::ptr::eq(d, n) {
        if let Some(origin) = self_origin(n, env) {
            out.origins.insert(n.id, origin.label);
        }
    }
    if taint.alternatives_dropped {
        limits.alternatives = true;
    }
    out.taint.insert(n.id, taint);
}

/// The taint of one already-decomposed node.
fn classify(
    d: &Node,
    env: &Env,
    bounds: Bounds,
    depth: u32,
    out: &Measured,
    limits: &mut Limits,
) -> Taint {
    if depth == 0 {
        // Fail closed: an expression the search did not reach has not been
        // shown to be independent of the spender.
        limits.depth = true;
        return Taint::tainted(DEPTH_LIMITED);
    }
    match &d.kind {
        NodeKind::Bool(_) | NodeKind::Int(_) | NodeKind::Num(_) | NodeKind::Const(_) => {
            Taint::default()
        }
        _ => match self_origin(d, env) {
            Some(origin) => Taint {
                sources: BTreeSet::from([origin.label]),
                alternatives_dropped: false,
                unknown: origin.unmodelled,
            },
            // Every remaining shape takes the value of its operands: casts,
            // wrappers, collections, blocks, lambdas and any variant added
            // later. `children` is exhaustive, so an unmodelled shape still
            // reaches its sub-expressions rather than laundering a source away.
            None => {
                let mut acc = Taint::default();
                for c in children(d) {
                    let taint = out
                        .taint
                        .get(&c.id)
                        .cloned()
                        .unwrap_or_else(|| Taint::tainted(UNRESOLVED));
                    acc.absorb(taint, bounds.max_alternatives);
                }
                acc
            }
        },
    }
}

fn tainted(n: &Node, measured: &Measured) -> bool {
    measured.taint.get(&n.id).is_some_and(Taint::is_tainted)
}

// ── sinks ──────────────────────────────────────────────────────────────────────

fn sink_room(out: &[Sink<'_>], bounds: Bounds, limits: &mut Limits) -> bool {
    if out.len() >= bounds.max_sinks {
        limits.sinks = true;
        false
    } else {
        true
    }
}

/// Record every security-sensitive site whose consumed value is tainted.
fn collect_sinks<'a>(
    n: &'a Node,
    env: &Env<'a>,
    measured: &Measured,
    bounds: Bounds,
    out: &mut Vec<Sink<'a>>,
    limits: &mut Limits,
    self_mentions: &mut HashMap<(u64, u32), Option<bool>>,
) {
    if !sink_room(out, bounds, limits) {
        return;
    }
    let d = n;

    // Dynamic code and code-template bytes.
    if let Some((operand, label)) = code_bytes(d) {
        if let Some(supplied) = context_var_bytes(d) {
            if !sink_room(out, bounds, limits) {
                return;
            }
            out.push(Sink {
                kind: SinkKind::CodeBytes,
                operand,
                site: d,
                supplied: Some(supplied),
                label: label.to_owned(),
            });
        } else if tainted(operand, measured) {
            if !sink_room(out, bounds, limits) {
                return;
            }
            out.push(Sink {
                kind: SinkKind::CodeBytes,
                operand,
                site: d,
                supplied: None,
                label: label.to_owned(),
            });
        }
    }

    // A spender-chosen index picks which element — or which register — the rest
    // of the script is then allowed to reason about.
    if let Some((index, what)) = index_operand(d, &env.vals) {
        if tainted(index, measured) {
            if !sink_room(out, bounds, limits) {
                return;
            }
            out.push(Sink {
                kind: SinkKind::CollectionIndex,
                operand: index,
                site: d,
                supplied: None,
                label: what,
            });
        }
    }

    // Comparisons whose meaning is anchored on the box being spent.
    if let NodeKind::Infix(op, lhs, rhs) = &d.kind {
        if is_comparison(op) && self_cannot_be_ruled_out(d, env, bounds, limits, self_mentions) {
            let (kind, label) = self_anchor(op, d, env);
            for side in [lhs.as_ref(), rhs.as_ref()] {
                if tainted(side, measured) {
                    if !sink_room(out, bounds, limits) {
                        return;
                    }
                    out.push(Sink {
                        kind,
                        operand: side,
                        site: d,
                        supplied: None,
                        label: label.clone(),
                    });
                }
            }
        }
    }

    for c in children(d) {
        collect_sinks(c, env, measured, bounds, out, limits, self_mentions);
    }
}

fn is_comparison(op: &str) -> bool {
    matches!(op, "==" | "!=" | "<" | "<=" | ">" | ">=")
}

/// The bytes a dynamic-code call consumes, and what the call is called.
fn code_bytes(d: &Node) -> Option<(&Node, &'static str)> {
    match &d.kind {
        NodeKind::Global(name, args) => {
            let base = without_type_args(name);
            let label = match base {
                "executeFromVar" => "dynamic code bytes",
                "deserialize" | "deserializeTo" => "dynamic deserialization bytes",
                "substConstants" => "dynamic code-template bytes",
                _ => return None,
            };
            args.first().map(|a| (a, label))
        }
        NodeKind::Method(recv, name, _)
            if matches!(without_type_args(name), "deserialize" | "deserializeTo") =>
        {
            Some((recv.as_ref(), "dynamic deserialization bytes"))
        }
        _ => None,
    }
}

/// `executeFromVar[T](id)` consumes context-variable `id`'s bytes, which the
/// spender supplies with the proof whatever the id literal says. A non-literal
/// id keeps the taint and says so.
fn context_var_bytes(d: &Node) -> Option<Taint> {
    let NodeKind::Global(name, args) = &d.kind else {
        return None;
    };
    if without_type_args(name) != "executeFromVar" {
        return None;
    }
    Some(match args.first().and_then(var_id) {
        Some(id) => Taint::named(&format!("getVar[{id}]:Coll[Byte]")),
        None => Taint::tainted("executeFromVar:unresolved-id"),
    })
}

fn var_id(n: &Node) -> Option<i64> {
    match &n.kind {
        NodeKind::Int(id) => Some(*id),
        NodeKind::Num(text) => text.trim_end_matches(['L', 'l']).parse().ok(),
        _ => None,
    }
}

/// The index a positional access selects with, and what it selects from.
fn index_operand<'a>(d: &'a Node, vals: &Vals<'a>) -> Option<(&'a Node, String)> {
    match &d.kind {
        NodeKind::ApplyFn(coll, args) if args.len() == 1 => {
            Some((&args[0], collection_name(coll, vals)))
        }
        NodeKind::Index(coll, index, _) => Some((index.as_ref(), collection_name(coll, vals))),
        NodeKind::GetRegDyn(_, _, args) if args.len() == 1 => {
            Some((&args[0], "a dynamic register index".to_owned()))
        }
        NodeKind::Method(_, name, args)
            if without_type_args(name) == "getOrElse" && !args.is_empty() =>
        {
            Some((&args[0], "a defaulted index".to_owned()))
        }
        _ => None,
    }
}

/// Name the collection a positional access selects from, as far as the tree
/// shows. An unrecognised receiver is named generically rather than guessed at.
fn collection_name<'a>(coll: &'a Node, vals: &Vals<'a>) -> String {
    if let Some(name) = box_collection(coll, vals) {
        return name.to_owned();
    }
    if let Some(box_expr) = tokens_receiver(coll, vals) {
        if let Some(key) = box_key(box_expr, vals) {
            return key;
        }
    }
    match &deref(coll, vals).kind {
        NodeKind::Leaf(name) => (*name).to_owned(),
        NodeKind::Val(name) => format!("val {name}"),
        _ => "a collection".to_owned(),
    }
}

/// Can `SELF` be ruled out of this comparison inside the depth bound?
///
/// A bound that runs out is a `yes`, not a `no`: a comparison whose anchor the
/// search could not read is treated as a `SELF` guard.
fn self_cannot_be_ruled_out(
    n: &Node,
    env: &Env,
    bounds: Bounds,
    limits: &mut Limits,
    cache: &mut HashMap<(u64, u32), Option<bool>>,
) -> bool {
    match mentions_self(n, env, bounds.max_depth, cache) {
        Some(false) => false,
        Some(true) => true,
        None => {
            limits.depth = true;
            true
        }
    }
}

/// `Some(true)`/`Some(false)` once the search completed; `None` if the depth
/// bound cut it short with the answer still open.
fn mentions_self(
    n: &Node,
    env: &Env,
    depth: u32,
    cache: &mut HashMap<(u64, u32), Option<bool>>,
) -> Option<bool> {
    if let Some(found) = cache.get(&(n.id, depth)) {
        return *found;
    }
    cache.insert((n.id, depth), None);
    let found = mentions_self_uncached(n, env, depth, cache);
    cache.insert((n.id, depth), found);
    found
}

fn mentions_self_uncached(
    n: &Node,
    env: &Env,
    depth: u32,
    cache: &mut HashMap<(u64, u32), Option<bool>>,
) -> Option<bool> {
    if depth == 0 {
        return None;
    }
    if let NodeKind::Val(name) = &n.kind {
        if let Some(bound) = env.vals.get(name) {
            return mentions_self(bound, env, depth, cache);
        }
    }
    let d = n;
    if matches!(d.kind, NodeKind::Leaf("SELF")) {
        return Some(true);
    }
    let mut found = false;
    for c in children(d) {
        match mentions_self(c, env, depth - 1, cache) {
            Some(true) => found = true,
            Some(false) => {}
            None => return None,
        }
    }
    Some(found)
}

/// What a comparison anchored on `SELF` decides, and the anchor's name.
fn self_anchor(op: &str, cmp: &Node, env: &Env) -> (SinkKind, String) {
    // Reserve reads are recognised through casts, negation and nested
    // arithmetic, so `SELF.value - amount >= 0` is still a reserve guard.
    let mut reads = Vec::new();
    for side in children(cmp) {
        reads_in_operand(side, &env.vals, OPERAND_DEPTH, &mut reads);
    }
    if let Some((_, _, spelling)) = reads
        .iter()
        .find(|(box_expr, _, _)| box_key(box_expr, &env.vals).as_deref() == Some("SELF"))
    {
        return (SinkKind::ReserveGuard, format!("SELF{spelling}"));
    }
    let mut properties = BTreeSet::new();
    self_properties(cmp, env, OPERAND_DEPTH, &mut properties);
    let label = if properties
        .iter()
        .any(|p| matches!(p.as_str(), "propositionBytes" | "ergoTree" | "scriptBytes"))
    {
        "SELF's script bytes".to_owned()
    } else if properties.contains("id") {
        "SELF.id".to_owned()
    } else {
        match properties.iter().next() {
            Some(name) if name.starts_with("tokens(") => format!("SELF's {name}"),
            Some(name) => format!("SELF.{name}"),
            None => "a SELF-rooted value".to_owned(),
        }
    };
    if is_comparison(op) && matches!(op, "==" | "!=") {
        (SinkKind::IdentityGuard, label)
    } else {
        (SinkKind::SelfGuard, label)
    }
}

/// Property names read off `SELF` anywhere inside `n`.
fn self_properties(n: &Node, env: &Env, depth: u32, out: &mut BTreeSet<String>) {
    if depth == 0 {
        return;
    }
    let d = deref(n, &env.vals);
    if let NodeKind::Prop(box_expr, name) = &d.kind {
        if let Some((key, path)) = box_relative(box_expr, name, &env.vals) {
            if key == "SELF" {
                out.insert(path);
            }
        }
    }
    for c in children(d) {
        self_properties(c, env, depth - 1, out);
    }
}

// ── tracing ───────────────────────────────────────────────────────────────────

/// Walk a sink's operand back to the nearest named source on each branch.
///
/// The descent follows value dependency, so it never leaves the subgraph the
/// sink's operand is computed from, and it never descends into a value the
/// analysis proved clean. A branch that ends at a bound without naming a
/// source is reported as one unresolved flow rather than dropped.
fn trace<'a>(
    sink: &Sink<'a>,
    env: &Env<'a>,
    measured: &Measured,
    bounds: Bounds,
    out: &mut Vec<Trace<'a>>,
    limits: &mut Limits,
) {
    if let Some(supplied) = &sink.supplied {
        for label in &supplied.sources {
            out.push(Trace {
                origin: Some(sink.operand),
                label: label.clone(),
                path: vec![sink.operand.id],
                bounded: supplied.unknown,
            });
        }
        return;
    }
    let mut queue: VecDeque<(&'a Node, Vec<u64>)> = VecDeque::new();
    let mut seen: HashSet<u64> = HashSet::new();
    seen.insert(sink.operand.id);
    queue.push_back((sink.operand, vec![sink.operand.id]));
    let mut cut = false;
    while let Some((n, path)) = queue.pop_front() {
        if out.len() >= bounds.max_alternatives {
            limits.alternatives = true;
            cut = true;
            break;
        }
        if let Some(label) = measured.origins.get(&n.id) {
            // The label already names this branch's origin; the coarse
            // ancestor sources below it would only add noise.
            out.push(Trace {
                origin: Some(n),
                label: label.clone(),
                path,
                bounded: false,
            });
            continue;
        }
        if !tainted(n, measured) {
            continue;
        }
        if path.len() >= bounds.max_depth as usize {
            limits.depth = true;
            cut = true;
            continue;
        }
        for c in deps(n, env) {
            if seen.insert(c.id) {
                let mut next = path.clone();
                next.push(c.id);
                queue.push_back((c, next));
            }
        }
    }
    if cut && out.is_empty() {
        out.push(Trace {
            origin: None,
            label: UNRESOLVED.to_owned(),
            path: Vec::new(),
            bounded: true,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::decompile::Stmt;

    thread_local! {
        /// Lift ids are unique within one decompilation, and the analysis keys
        /// its measured taints and origins on them. A shared counter keeps
        /// hand-built trees as faithful as lifted ones.
        static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    }

    fn node(kind: NodeKind) -> Node {
        let id = NEXT_ID.with(|next| {
            let id = next.get();
            next.set(id + 1);
            id
        });
        Node { id, kind }
    }

    fn leaf(name: &'static str) -> Node {
        node(NodeKind::Leaf(name))
    }
    fn int(value: i64) -> Node {
        node(NodeKind::Int(value))
    }
    fn var(id: i64) -> Node {
        node(NodeKind::GetVar(id, "Long".into()))
    }
    fn val(name: &str) -> Node {
        node(NodeKind::Val(name.into()))
    }
    fn raw() -> Node {
        node(NodeKind::Raw("<op 0x99>".into()))
    }
    fn prop(obj: Node, name: &str) -> Node {
        node(NodeKind::Prop(Box::new(obj), name.into()))
    }
    fn method(obj: Node, name: &str) -> Node {
        node(NodeKind::Method(Box::new(obj), name.into(), vec![]))
    }
    fn indexed(collection: Node, index: Node) -> Node {
        node(NodeKind::ApplyFn(Box::new(collection), vec![index]))
    }
    fn self_box() -> Node {
        leaf("SELF")
    }
    fn input(index: Node) -> Node {
        indexed(leaf("INPUTS"), index)
    }
    fn output(index: Node) -> Node {
        indexed(leaf("OUTPUTS"), index)
    }
    /// `INPUTS(i).R4[Long].get`
    fn input_register(index: Node) -> Node {
        method(prop(input(index), "R4[Long]"), "get")
    }
    /// `SELF.value`
    fn self_value() -> Node {
        prop(self_box(), "value")
    }
    fn comparison(op: &'static str, lhs: Node, rhs: Node) -> Node {
        node(NodeKind::Infix(op, Box::new(lhs), Box::new(rhs)))
    }
    fn block(bindings: Vec<(&str, Node)>, result: Node) -> Node {
        node(NodeKind::Block(
            bindings
                .into_iter()
                .map(|(name, expr)| Stmt::Val(name.into(), expr))
                .collect(),
            Box::new(result),
        ))
    }

    fn flows_of(tree: Node) -> Report {
        analyze(&tree)
    }

    fn kinds(report: &Report) -> Vec<SinkKind> {
        report.flows.iter().map(|f| f.kind).collect()
    }

    fn sources_of(report: &Report) -> Vec<&str> {
        report.flows.iter().map(|f| f.source.as_str()).collect()
    }

    #[test]
    fn a_spender_register_ordering_self_value_is_a_reserve_guard() {
        let tree = comparison("<=", input_register(int(0)), self_value());
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(report.flows[0].sink, "SELF.value");
        assert_eq!(sources_of(&report), vec!["INPUTS(0).R4[Long]"]);
        assert!(!report.limits.any(), "{:?}", report.limits);
    }

    #[test]
    fn a_spender_register_matching_self_value_is_still_a_reserve_guard() {
        let tree = comparison("==", self_value(), input_register(int(1)));
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(sources_of(&report), vec!["INPUTS(1).R4[Long]"]);
    }

    #[test]
    fn reserve_arithmetic_is_read_through_to_self() {
        let tree = comparison(
            ">=",
            comparison("-", self_value(), input_register(int(0))),
            int(0),
        );
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(report.flows[0].sink, "SELF.value");
        assert!(report.flows[0].hops > 0);
    }

    #[test]
    fn self_only_guards_produce_no_flow() {
        for tree in [
            comparison("<=", self_value(), int(1_000_000)),
            comparison("==", prop(self_box(), "id"), int(7)),
            comparison("==", prop(self_box(), "R4[Long]"), int(3)),
        ] {
            assert!(flows_of(tree).flows.is_empty());
        }
    }

    #[test]
    fn comparisons_without_a_self_anchor_produce_no_flow() {
        for tree in [
            comparison("<=", input_register(int(0)), int(1_000_000)),
            comparison("==", prop(input(int(0)), "id"), prop(input(int(1)), "id")),
        ] {
            let report = flows_of(tree);
            assert!(report.flows.is_empty(), "{:?}", report.flows);
        }
    }

    #[test]
    fn unknown_constructs_taint_rather_than_sanitize() {
        let tree = comparison("==", raw(), prop(self_box(), "id"));
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::IdentityGuard]);
        assert_eq!(sources_of(&report), vec![UNMODELLED]);
    }

    #[test]
    fn an_unresolved_binding_is_reported_as_unmodelled_provenance() {
        let tree = comparison("<", val("nowhere"), self_value());
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(sources_of(&report), vec!["unbound-binding:nowhere"]);
    }

    #[test]
    fn a_val_reference_is_attributed_to_the_source_it_binds() {
        let tree = block(
            vec![("amount", input_register(int(0)))],
            comparison("<=", val("amount"), self_value()),
        );
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(sources_of(&report), vec!["INPUTS(0).R4[Long]"]);
    }

    #[test]
    fn a_lambda_parameter_is_a_source() {
        let body = comparison(
            "<=",
            method(prop(val("b"), "R4[Long]"), "get"),
            self_value(),
        );
        let tree = node(NodeKind::Lambda(vec!["b: Box".into()], Box::new(body)));
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(sources_of(&report), vec!["lambda-parameter:b"]);
    }

    #[test]
    fn a_spender_chosen_index_selects_an_element() {
        let report = flows_of(input(var(1)));
        assert_eq!(kinds(&report), vec![SinkKind::CollectionIndex]);
        assert_eq!(report.flows[0].sink, "INPUTS");
        assert_eq!(sources_of(&report), vec!["getVar[1]:Long"]);
    }

    #[test]
    fn static_indices_are_not_a_flow() {
        assert!(flows_of(input(int(0))).flows.is_empty());
        let tokens = method(output(int(0)), "tokens");
        let report = flows_of(indexed(tokens, int(0)));
        assert!(report.flows.is_empty(), "{:?}", report.flows);
    }

    #[test]
    fn a_dynamic_register_index_is_a_collection_index_sink() {
        let tree = node(NodeKind::GetRegDyn(
            Box::new(self_box()),
            "Long".into(),
            vec![var(2)],
        ));
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::CollectionIndex]);
        assert_eq!(report.flows[0].sink, "a dynamic register index");
    }

    #[test]
    fn execute_from_var_reads_spender_supplied_bytes_whatever_its_id_spells() {
        let tree = node(NodeKind::Global(
            "executeFromVar[Coll[Byte]]".into(),
            vec![int(3)],
        ));
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::CodeBytes]);
        assert_eq!(sources_of(&report), vec!["getVar[3]:Coll[Byte]"]);
    }

    #[test]
    fn a_non_literal_execute_from_var_id_stays_unknown() {
        let tree = node(NodeKind::Global(
            "executeFromVar[Coll[Byte]]".into(),
            vec![var(0)],
        ));
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::CodeBytes]);
        assert!(report.flows[0].bounded);
    }

    #[test]
    fn literal_code_bytes_are_clean_and_a_data_input_register_is_a_source() {
        let bytes = node(NodeKind::Coll("Byte".into(), vec![int(1), int(2)]));
        let literal = node(NodeKind::Global(
            "deserialize[Coll[Byte]]".into(),
            vec![bytes],
        ));
        assert!(flows_of(literal).flows.is_empty());

        let data_inputs = method(leaf("CONTEXT"), "dataInputs");
        let data_register = prop(indexed(data_inputs, int(0)), "R4[Long]");
        let report = flows_of(comparison("<=", data_register, self_value()));
        assert_eq!(kinds(&report), vec![SinkKind::ReserveGuard]);
        assert_eq!(sources_of(&report), vec!["CONTEXT.dataInputs(0).R4[Long]"]);
    }

    #[test]
    fn a_successor_script_equality_is_a_low_identity_guard() {
        let tree = comparison(
            "==",
            prop(output(int(0)), "propositionBytes"),
            prop(self_box(), "propositionBytes"),
        );
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::IdentityGuard]);
        assert_eq!(report.flows[0].sink, "SELF's script bytes");
        assert_eq!(sources_of(&report), vec!["OUTPUTS(0).propositionBytes"]);
    }

    #[test]
    fn a_self_register_ordering_is_a_self_guard() {
        let tree = comparison(">", prop(self_box(), "R4[Int]"), int(5));
        let report = flows_of(tree);
        assert!(
            report.flows.is_empty(),
            "a literal right-hand side is not a source"
        );

        let tree = comparison(
            ">",
            prop(self_box(), "R4[Int]"),
            prop(input(int(0)), "R4[Int]"),
        );
        let report = flows_of(tree);
        assert_eq!(kinds(&report), vec![SinkKind::SelfGuard]);
        assert_eq!(report.flows[0].sink, "SELF.R4[Int]");
    }

    #[test]
    fn the_depth_ceiling_fails_closed_to_explicit_unresolved_flows() {
        let tree = comparison("<=", input_register(int(0)), self_value());
        let bounded = analyze_with(
            &tree,
            Bounds {
                max_depth: 1,
                ..Bounds::default()
            },
        );
        assert!(bounded.limits.depth);
        assert!(!bounded.flows.is_empty());
        for flow in &bounded.flows {
            assert!(flow.bounded, "{flow:?}");
            assert_eq!(flow.source, UNRESOLVED);
        }
        // A budget that reaches the operands names the source instead.
        let named = analyze_with(
            &tree,
            Bounds {
                max_depth: 8,
                ..Bounds::default()
            },
        );
        assert!(!named.limits.any(), "{:?}", named.limits);
        assert_eq!(sources_of(&named), vec!["INPUTS(0).R4[Long]"]);
        assert!(named.flows.iter().all(|f| !f.bounded));
    }

    #[test]
    fn the_sink_budget_is_reported_rather_than_silently_truncating() {
        let box_value = prop(input(int(0)), "value");
        let tree = block(
            vec![
                ("a", comparison("<=", box_value.clone(), self_value())),
                ("b", comparison("<=", box_value.clone(), self_value())),
                ("c", comparison("<=", box_value, self_value())),
            ],
            comparison("<=", input_register(int(0)), self_value()),
        );
        let report = analyze_with(
            &tree,
            Bounds {
                max_sinks: 1,
                ..Bounds::default()
            },
        );
        assert!(report.limits.sinks);
        assert_eq!(report.sinks, 1);
        assert!(report.limits.note().is_some());
    }

    #[test]
    fn the_observation_budget_is_reported() {
        let tree = block(
            vec![("a", prop(input(int(0)), "value"))],
            comparison(
                "&&",
                comparison("<=", val("a"), self_value()),
                comparison("<=", input_register(int(1)), self_value()),
            ),
        );
        let report = analyze_with(
            &tree,
            Bounds {
                max_observations: 1,
                ..Bounds::default()
            },
        );
        assert!(report.limits.observations);
        assert_eq!(report.flows.len(), 1);
    }

    #[test]
    fn alternative_names_are_capped_deterministically() {
        let first = prop(input(int(0)), "value");
        let second = prop(input(int(1)), "value");
        let third = prop(input(int(2)), "value");
        let tree = block(
            vec![("a", first), ("b", second), ("c", third)],
            comparison("<=", val("a"), comparison("+", val("b"), val("c"))),
        );
        let bounded = analyze_with(
            &tree,
            Bounds {
                max_alternatives: 1,
                ..Bounds::default()
            },
        );
        assert!(bounded.limits.alternatives);
        assert!(!bounded.flows.iter().any(|f| f.source.is_empty()));
        let repeat = analyze_with(
            &tree,
            Bounds {
                max_alternatives: 1,
                ..Bounds::default()
            },
        );
        assert_eq!(sources_of(&bounded), sources_of(&repeat));
    }

    #[test]
    fn limits_note_is_absent_only_when_nothing_was_cut() {
        assert!(Limits::default().note().is_none());
        assert!(!Limits::default().any());
        let hit = Limits {
            depth: true,
            ..Limits::default()
        };
        assert!(hit.any());
        assert!(hit.note().is_some_and(|n| n.contains("depth")));
    }
}
