//! The lifted AST: source-like expression shapes recovered from ErgoTree
//! wire bytes. Produced by [`super::lift`], consumed by [`super::print`] and
//! (later) the audit layer.

// ── lifted AST ───────────────────────────────────────────────────────────────

/// A lifted, printer-oriented expression. Lifts = wire shapes → source-like
/// shapes (infix, casts, property/method calls, block scoping).
#[derive(Debug, Clone)]
pub(crate) enum L {
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
    Unary(&'static str, Box<L>),
    /// Infix binary operator application. Precedence is derived from the
    /// symbol at print time (`print::prec_of`) — it is a rendering concern,
    /// not part of the recovered structure.
    Infix(&'static str, Box<L>, Box<L>),
    Method(Box<L>, String, Vec<L>),
    /// `fn(args…)` — an apply form the compiler parses back as ByIndex on
    /// collection-typed receivers (OUTPUTS(0), tokens(0), …).
    ApplyFn(Box<L>, Vec<L>),
    /// `obj.name` (property call, no args).
    Prop(Box<L>, String),
    /// `obj.getReg[T](expr)` — dynamic register index.
    GetRegDyn(Box<L>, String, Vec<L>),
    /// `Coll(a, b, …)` literal, with the element type name for the empty
    /// case (empty literals must be type-ascribed to recompile).
    Coll(String, Vec<L>),
    /// `(a, b, …)` tuple literal.
    Tuple(Vec<L>),
    /// `obj[i]` (optional default renders as `obj[i].getOrElse(default)`-like
    /// via ByIndex's getOrElse form below).
    Index(Box<L>, Box<L>, Option<Box<L>>),
    /// `{ stmts; result }` block.
    Block(Vec<Stmt>, Box<L>),
    /// `if (c) t else e`.
    If(Box<L>, Box<L>, Box<L>),
    /// `fn (a, b) -> body` lambda.
    Lambda(Vec<String>, Box<L>),
    /// Global function call: `name(args…)`.
    Global(String, Vec<L>),
    /// AtLeast(k, Coll[...]) — k-of-n signature threshold.
    AtLeast(Box<L>, Box<L>),
    /// Fallback: fully-parenthesized structural form for anything not yet
    /// lifted (renders via the inspect printer).
    Raw(String),
}

/// A statement in a block: `val <name> = <expr>` or `def <name> = <expr>`.
#[derive(Debug, Clone)]
pub(crate) enum Stmt {
    Val(String, L),
    Def(String, L),
}

/// Count `L::Raw` nodes in a lifted tree.
pub(crate) fn count_raw(e: &L) -> usize {
    let mut n = usize::from(matches!(e, L::Raw(_)));
    match e {
        L::Unary(_, a) => n += count_raw(a),
        L::Infix(_, a, b) => n += count_raw(a) + count_raw(b),
        L::Method(o, _, args) | L::ApplyFn(o, args) | L::GetRegDyn(o, _, args) => {
            n += count_raw(o) + args.iter().map(count_raw).sum::<usize>()
        }
        L::Prop(o, _) => n += count_raw(o),
        L::Coll(_, items) | L::Tuple(items) => n += items.iter().map(count_raw).sum::<usize>(),
        L::Global(_, args) => n += args.iter().map(count_raw).sum::<usize>(),
        L::Lambda(_, b) => n += count_raw(b),
        L::If(c, t, els) => n += count_raw(c) + count_raw(t) + count_raw(els),
        L::AtLeast(k, c) => n += count_raw(k) + count_raw(c),
        L::Index(a, b, d) => {
            n += count_raw(a) + count_raw(b) + d.as_deref().map(count_raw).unwrap_or(0)
        }
        L::Block(stmts, result) => {
            n += stmts
                .iter()
                .map(|s| match s {
                    Stmt::Val(_, e) | Stmt::Def(_, e) => count_raw(e),
                })
                .sum::<usize>()
                + count_raw(result)
        }
        L::Raw(_)
        | L::Bool(_)
        | L::Int(_)
        | L::Num(_)
        | L::Const(_)
        | L::Val(_)
        | L::GetVar(..)
        | L::Leaf(_) => {}
    }
    n
}
