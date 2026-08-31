//! The lifted AST: source-like expression shapes recovered from ErgoTree
//! wire bytes. Produced by [`super::lift`], consumed by [`super::print`] and
//! (later) the audit layer.

// ── lifted AST ───────────────────────────────────────────────────────────────

/// A lifted, printer-oriented expression. Lifts = wire shapes → source-like
/// shapes (infix, casts, property/method calls, block scoping).
#[derive(Debug, Clone)]
pub enum NodeKind {
    Bool(bool),
    Int(i64),
    /// Any other literal, printed via its value form.
    Const(String),
    /// Reference to a `val` binding by its assigned source name.
    Val(String),
    /// Context variable read `getVar[T](id)` — carries the var id plus the
    /// source-form type parameter (empty when unknown).
    GetVar(i64, String),
    /// Integer literal with its source-form suffix (`1`, `1L`, `7y`).
    Num(String),
    /// Height/output/input/… leaves.
    Leaf(&'static str),
    Unary(&'static str, Box<Node>),
    /// Infix binary operator application. Precedence is derived from the
    /// symbol at print time (`print::prec_of`) — it is a rendering concern,
    /// not part of the recovered structure.
    Infix(&'static str, Box<Node>, Box<Node>),
    Method(Box<Node>, String, Vec<Node>),
    /// `fn(args…)` — an apply form the compiler parses back as ByIndex on
    /// collection-typed receivers (OUTPUTS(0), tokens(0), …).
    ApplyFn(Box<Node>, Vec<Node>),
    /// `obj.name` (property call, no args).
    Prop(Box<Node>, String),
    /// `obj.getReg[T](expr)` — dynamic register index.
    GetRegDyn(Box<Node>, String, Vec<Node>),
    /// `Coll(a, b, …)` literal, with the element type name for the empty
    /// case (empty literals must be type-ascribed to recompile).
    Coll(String, Vec<Node>),
    /// `(a, b, …)` tuple literal.
    Tuple(Vec<Node>),
    /// `obj[i]` (optional default renders as `obj[i].getOrElse(default)`-like
    /// via ByIndex's getOrElse form below).
    Index(Box<Node>, Box<Node>, Option<Box<Node>>),
    /// `{ stmts; result }` block.
    Block(Vec<Stmt>, Box<Node>),
    /// `if (c) t else e`.
    If(Box<Node>, Box<Node>, Box<Node>),
    /// `fn (a, b) -> body` lambda.
    Lambda(Vec<String>, Box<Node>),
    /// Global function call: `name(args…)`.
    Global(String, Vec<Node>),
    /// AtLeast(k, Coll[...]) — k-of-n signature threshold.
    AtLeast(Box<Node>, Box<Node>),
    /// Fallback: fully-parenthesized structural form for anything not yet
    /// lifted (renders via the inspect printer).
    Raw(String),
}

/// A statement in a block: `val <name> = <expr>` or `def <name> = <expr>`.
#[derive(Debug, Clone)]
pub enum Stmt {
    Val(String, Node),
    Def(String, Node),
}

/// A lifted node: recovered source-like structure plus its identity.
#[derive(Debug, Clone)]
pub struct Node {
    /// Identity of the IR node this was lifted from.
    ///
    /// **Lift-local.** Assigned by this crate's own walk of the ErgoTree IR,
    /// in visit order. Stable and unique within one decompilation — which is
    /// what lints need to dedupe and cross-reference findings — but NOT yet
    /// the shared IR preorder index the compiler's source map will key on.
    ///
    /// Correlating with `ergo_compiler`'s source map requires both sides to
    /// take ids from one shared `ergo_ser::preorder` walk, which does not
    /// exist yet. Independently-derived indices misalign silently: `lift`
    /// returns `NodeKind::Raw` at `MAX_LIFT_DEPTH` WITHOUT descending, so a
    /// counter drifts by the skipped subtree's size. See
    /// `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.
    pub id: u64,
    /// The recovered shape.
    pub kind: NodeKind,
}

/// Count `NodeKind::Raw` nodes in a lifted tree.
pub fn count_raw(e: &Node) -> usize {
    let mut n = usize::from(matches!(&e.kind, NodeKind::Raw(_)));
    match &e.kind {
        NodeKind::Unary(_, a) => n += count_raw(a),
        NodeKind::Infix(_, a, b) => n += count_raw(a) + count_raw(b),
        NodeKind::Method(o, _, args)
        | NodeKind::ApplyFn(o, args)
        | NodeKind::GetRegDyn(o, _, args) => {
            n += count_raw(o) + args.iter().map(count_raw).sum::<usize>()
        }
        NodeKind::Prop(o, _) => n += count_raw(o),
        NodeKind::Coll(_, items) | NodeKind::Tuple(items) => {
            n += items.iter().map(count_raw).sum::<usize>()
        }
        NodeKind::Global(_, args) => n += args.iter().map(count_raw).sum::<usize>(),
        NodeKind::Lambda(_, b) => n += count_raw(b),
        NodeKind::If(c, t, els) => n += count_raw(c) + count_raw(t) + count_raw(els),
        NodeKind::AtLeast(k, c) => n += count_raw(k) + count_raw(c),
        NodeKind::Index(a, b, d) => {
            n += count_raw(a) + count_raw(b) + d.as_deref().map(count_raw).unwrap_or(0)
        }
        NodeKind::Block(stmts, result) => {
            n += stmts
                .iter()
                .map(|s| match s {
                    Stmt::Val(_, e) | Stmt::Def(_, e) => count_raw(e),
                })
                .sum::<usize>()
                + count_raw(result)
        }
        NodeKind::Raw(_)
        | NodeKind::Bool(_)
        | NodeKind::Int(_)
        | NodeKind::Num(_)
        | NodeKind::Const(_)
        | NodeKind::Val(_)
        | NodeKind::GetVar(..)
        | NodeKind::Leaf(_) => {}
    }
    n
}
