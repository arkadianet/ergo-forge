//! Decompiler: ErgoTree wire bytes → readable ErgoScript (or an honest
//! structural view when a tree doesn't round-trip).
//!
//! Pipeline: parse → SSA-unmangle (ValDef/FuncValue ids are shared a
//! namespace on the wire; renumber hierarchically) → lift (recognize
//! operator shapes, inline trivial `val`s) → pretty-print with precedence
//! and infix sugar.
//!
//! ## The verification bar (workbench-PLAN.md)
//!
//! `decompile → recompile → byte-identical` on known provenance. This is
//! checked by `tests/decompile_roundtrip.rs` over the compile corpus. The
//! bar is achievable only where the tree's shape is exactly what the
//! compiler emits; hand-built trees may normalize differently — the
//! round-trip test measures, and any miss is either a printer bug (fix) or
//! an honest normalization (document + badge).
//!
//! Renderings target a readable *source-like* form, not byte-exact
//! reproduction of any particular input: whitespace, parenthesization, and
//! `val` naming are canonical. Recompilation must therefore be
//! whitespace/format-insensitive — which the compiler is (it parses from
//! source text).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ergo_ser::opcode::{Expr, Payload};
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};

use crate::method_names::METHOD_NAMES;

// ── public entry points ──────────────────────────────────────────────────────

/// Decompile ErgoTree wire bytes into source-like ErgoScript.
///
/// The bar: the output RECOMPILES to byte-identical tree bytes (checked by
/// the round-trip test over the compile corpus). When a construct has no
/// source-like lift yet, it renders as an honest `<…>` raw placeholder —
/// never silently wrong.
pub fn decompile_bytes(bytes: &[u8]) -> Result<String, crate::SandboxError> {
    decompile_bytes_net(bytes, false)
}

/// Like [`decompile_bytes`], with `testnet` selecting the network for
/// `PK("…")` address constants (the corpus bar compiles on testnet;
/// mainnet trees need mainnet addresses to recompile).
pub fn decompile_bytes_net(bytes: &[u8], testnet: bool) -> Result<String, crate::SandboxError> {
    let tree = crate::inspect::parse_tree(bytes)?;
    Ok(render_net(&tree, testnet))
}

/// Decompile a parsed tree (testnet address constants — the corpus bar).
#[must_use]
pub fn render(tree: &ergo_ser::ergo_tree::ErgoTree) -> String {
    render_net(tree, true)
}

/// Decompile a parsed tree, addresses encoded for the chosen network.
#[must_use]
pub fn render_net(tree: &ergo_ser::ergo_tree::ErgoTree, testnet: bool) -> String {
    let mut cx = LiftCtx {
        testnet,
        ..LiftCtx::new()
    };
    let lifted = match &tree.body {
        Expr::Op(n) if n.opcode == 0xD1 => lift_op_inner(n, &mut cx, &tree.constants, true),
        other => lift(other, &mut cx, &tree.constants),
    };
    let mut out = String::new();
    print_l(&lifted, None, &mut out);
    out
}

// ── operator tables ──────────────────────────────────────────────────────────

/// Infix binary operators: opcode → (symbol, precedence).
/// Higher precedence binds tighter. Mirrors ErgoScript/Scala precedence:
/// unary > multiplicative > additive > comparison > logical.
fn infix_op(op: u8) -> Option<(&'static str, u8)> {
    Some(match op {
        0xEC => ("||", 1), // BinOr (lazy)
        0xED => ("&&", 2), // BinAnd (lazy)
        0x8F => ("<", 4),  // Lt
        0x90 => ("<=", 4), // Le
        0x91 => (">", 4),  // Gt
        0x92 => (">=", 4), // Ge
        0x93 => ("==", 4), // Eq
        0x94 => ("!=", 4), // Neq
        0xF4 => ("^", 5),  // BinXor (strict) — Scala assigns ^ lower than && but
        // ErgoScript parity keeps it above comparisons in practice; pinned by round-trip.
        0x99 => ("-", 6), // Minus
        0x9A => ("+", 6), // Plus
        0x9C => ("*", 7), // Multiply
        0x9D => ("/", 7), // Divide
        0x9E => ("%", 7), // Modulo
        _ => return None,
    })
}

/// Numeric-cast rendering: `expr.toByte`-style Select for the five casts.
fn cast_name(t: &SigmaType) -> Option<&'static str> {
    Some(match t {
        SigmaType::SByte => "toByte",
        SigmaType::SShort => "toShort",
        SigmaType::SInt => "toInt",
        SigmaType::SLong => "toLong",
        SigmaType::SBigInt => "toBigInt",
        _ => return None,
    })
}

// ── lifted AST ───────────────────────────────────────────────────────────────

/// A lifted, printer-oriented expression. Lifts = wire shapes → source-like
/// shapes (infix, casts, property/method calls, block scoping).
#[derive(Debug, Clone)]
enum L {
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
    Infix(&'static str, u8, Box<L>, Box<L>),
    Method(Box<L>, String, Vec<L>),
    /// `fn(args…)` — an apply form the compiler parses back as ByIndex on
    /// collection-typed receivers (OUTPUTS(0), tokens(0), …).
    ApplyFn(Box<L>, Vec<L>),
    /// `obj.name` (property call, no args).
    Prop(Box<L>, String),
    /// `obj.getReg[T](n)` — bracket-typed method form.
    GetReg(Box<L>, String, i64),
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
enum Stmt {
    Val(String, L),
    Def(String, L),
}

// ── printer ──────────────────────────────────────────────────────────────────

/// Operator precedence context: `None` = top level (no parens needed).
fn print_l(e: &L, parent: Option<u8>, out: &mut String) {
    let parens = |out: &mut String, f: &dyn Fn(&mut String)| {
        out.push('(');
        f(out);
        out.push(')');
    };
    match e {
        L::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        L::Int(i) => {
            let _ = write!(out, "{i}");
        }
        L::Num(s) => out.push_str(s),
        L::Const(s) => out.push_str(s),
        L::Val(name) => out.push_str(name),
        L::GetVar(id, tpe) => {
            // Source form is `getVar[T](id)` — the compiler's predef parses
            // the type parameter in brackets and the id in call parens.
            if tpe.is_empty() {
                let _ = write!(out, "getVar({id})");
            } else {
                let _ = write!(out, "getVar[{tpe}]({id})");
            }
        }
        L::Leaf(s) => out.push_str(s),
        L::Unary(op, inner) => {
            let this = 8u8;
            let needs = parent.is_some_and(|p| p > this);
            if needs {
                parens(out, &mut |o| print_l(inner, Some(this), o));
            } else {
                out.push_str(op);
                print_l(inner, Some(this), out);
            }
        }
        L::Infix(sym, prec, lhs, rhs) => {
            let this = *prec;
            let needs = parent.is_some_and(|p| p > this);
            let emit = |o: &mut String| {
                print_l(lhs, Some(this), o);
                o.push(' ');
                o.push_str(sym);
                o.push(' ');
                print_l(rhs, Some(this), o);
            };
            if needs {
                parens(out, &emit);
            } else {
                emit(out);
            }
        }
        L::Method(obj, name, args) => {
            // Receiver binds like a postfix expression (tightest).
            print_l(obj, Some(9), out);
            out.push('.');
            out.push_str(name);
            if !args.is_empty() {
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    print_l(a, None, out);
                }
                out.push(')');
            }
        }
        L::ApplyFn(f, args) => {
            print_l(f, Some(9), out);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_l(a, None, out);
            }
            out.push(')');
        }
        L::Prop(obj, name) => {
            print_l(obj, Some(9), out);
            out.push('.');
            out.push_str(name);
        }
        L::GetReg(obj, tpe, reg) => {
            print_l(obj, Some(9), out);
            let _ = write!(out, ".getReg[{tpe}]({reg})");
        }
        L::GetRegDyn(obj, tpe, args) => {
            print_l(obj, Some(9), out);
            out.push_str(&format!(".getReg[{tpe}]("));
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_l(a, None, out);
            }
            out.push(')');
        }
        L::Coll(elem, items) => {
            // An EMPTY collection literal needs its element type ascribed
            // (`Coll[Byte]()`); a non-empty one infers from the items.
            if items.is_empty() {
                let _ = write!(out, "Coll[{elem}]()");
            } else {
                out.push_str("Coll(");
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    print_l(it, None, out);
                }
                out.push(')');
            }
        }
        L::Tuple(items) => {
            out.push('(');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_l(it, None, out);
            }
            out.push(')');
        }
        L::Index(input, index, default) => {
            print_l(input, Some(9), out);
            out.push('[');
            print_l(index, None, out);
            out.push(']');
            if let Some(d) = default {
                out.push_str(".getOrElse(");
                print_l(d, None, out);
                out.push(')');
            }
        }
        L::If(cond, then, els) => {
            let emit = |o: &mut String| {
                o.push_str("if (");
                print_l(cond, None, o);
                o.push_str(") ");
                print_l(then, None, o);
                o.push_str(" else ");
                print_l(els, None, o);
            };
            // An `if..else` used as an operand (inside `&&`, arithmetic, …)
            // needs explicit parens — the parser ends the operator's right
            // operand at the unparenthesized `if`.
            if parent.is_some() {
                parens(out, &emit);
            } else {
                emit(out);
            }
        }
        L::Lambda(args, body) => {
            let emit = |o: &mut String| {
                o.push_str("{ (");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        o.push_str(", ");
                    }
                    o.push_str(a);
                }
                o.push_str(") => ");
                print_l(body, None, o);
                o.push('}');
            };
            if parent.is_some_and(|p| p > 0) {
                parens(out, &emit);
            } else {
                emit(out);
            }
        }
        L::Global(name, args) => {
            out.push_str(name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_l(a, None, out);
            }
            out.push(')');
        }
        L::AtLeast(k, items) => {
            out.push_str("atLeast(");
            print_l(k, None, out);
            out.push_str(", ");
            print_l(items, None, out);
            out.push(')');
        }
        L::Raw(s) => out.push_str(s),
        L::Block(stmts, result) => {
            out.push_str("{ ");
            for s in stmts {
                print_stmt(s, out);
                out.push_str("; ");
            }
            print_l(result, None, out);
            out.push_str(" }");
        }
    }
}

fn print_stmt(s: &Stmt, out: &mut String) {
    match s {
        Stmt::Val(name, e) => {
            let _ = write!(out, "val {name} = ");
            print_l(e, None, out);
        }
        Stmt::Def(name, e) => {
            let _ = write!(out, "def {name} = ");
            print_l(e, None, out);
        }
    }
}

// ── lift ─────────────────────────────────────────────────────────────────────

/// Names assigned to SSA ids during lift. Ids on the wire share one
/// namespace per `BlockValue`/`FuncValue` scope in practice, but real trees
/// (and older compilers) can collide across scopes, so lift renumbers
/// hierarchically: every binding gets a fresh source name, wired through
/// scope maps per block.
struct LiftCtx {
    /// PK address network: true = testnet (corpus bar), false = mainnet.
    testnet: bool,
    /// wire id → source name, per lexical scope stack.
    scopes: Vec<BTreeMap<u32, String>>,
    /// Global (scope-agnostic) id→name registry — fallback for ValUse ids
    /// whose lexical scope lookup missed (the wire's per-block id spaces
    /// make this unambiguous in practice: the most recent binding of an id
    /// is the in-scope one).
    global: BTreeMap<u32, String>,
    /// next counter per prefix.
    counters: BTreeMap<&'static str, usize>,
}

impl LiftCtx {
    fn new() -> Self {
        Self {
            testnet: true,
            scopes: vec![BTreeMap::new()],
            global: BTreeMap::new(),
            counters: BTreeMap::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Fresh name for a binding (wire ids are NOT reused as names — they
    /// collide across nested blocks).
    fn fresh(&mut self, prefix: &'static str) -> String {
        let n = self.counters.entry(prefix).or_insert(0);
        *n += 1;
        if *n == 1 {
            prefix.to_string()
        } else {
            format!("{prefix}{n}")
        }
    }

    fn bind(&mut self, id: u32, prefix: &'static str) -> String {
        let name = self.fresh(prefix);
        self.scopes
            .last_mut()
            .expect("scope stack never empty")
            .insert(id, name.clone());
        self.global.insert(id, name.clone());
        name
    }

    fn lookup(&self, id: u32) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(n) = scope.get(&id) {
                return Some(n.clone());
            }
        }
        self.global.get(&id).cloned()
    }
}

/// Lift a wire expression into the printer AST.
fn lift(e: &Expr, cx: &mut LiftCtx, constants: &[(SigmaType, SigmaValue)]) -> L {
    match e {
        Expr::Const { tpe, val } => lift_const(tpe, val, cx),
        Expr::Unparsed(bytes) => L::Raw(format!("<unparsed {} bytes>", bytes.len())),
        Expr::Op(node) => lift_op(node, cx, constants),
    }
}

fn lift_const(tpe: &SigmaType, val: &SigmaValue, cx: &LiftCtx) -> L {
    match (tpe, val) {
        (SigmaType::SBoolean, SigmaValue::Boolean(b)) => L::Bool(*b),
        (SigmaType::SByte, SigmaValue::Byte(x)) => L::Num(format!("{x}.toByte")),
        (SigmaType::SShort, SigmaValue::Short(x)) => L::Num(format!("{x}.toShort")),
        (SigmaType::SInt, SigmaValue::Int(x)) => L::Int(*x as i64),
        (SigmaType::SLong, SigmaValue::Long(x)) => L::Num(format!("{x}L")),
        (SigmaType::SBigInt, SigmaValue::BigInt(n)) => L::Const(format!("{n}L")),
        (SigmaType::SGroupElement, SigmaValue::GroupElement(ge)) => L::Const(format!(
            "PK(\"{}\")",
            crate::inspect::group_element_base58_net(ge.as_bytes(), cx.testnet)
        )),
        (SigmaType::SSigmaProp, SigmaValue::SigmaProp(sb)) => match sb {
            SigmaBoolean::ProveDlog(ge) => L::Const(format!(
                "PK(\"{}\")",
                crate::inspect::group_element_base58_net(ge.as_bytes(), cx.testnet)
            )),
            SigmaBoolean::TrivialProp(b) => L::Const(format!("sigmaProp({b})")),
            other => L::Raw(format!("{other:?}")),
        },
        (SigmaType::SColl(inner), SigmaValue::Coll(CollValue::Bytes(bs)))
            if **inner == SigmaType::SByte && bs.len() <= 8 =>
        {
            L::Const(format!(
                "Coll[Byte]({})",
                bs.iter()
                    .map(|b| format!("{b}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
        (_, SigmaValue::Coll(CollValue::BoolBits(bits))) => {
            L::Coll("Boolean".into(), bits.iter().map(|b| L::Bool(*b)).collect())
        }
        (tpe, SigmaValue::Coll(CollValue::Values(vs))) => L::Coll(
            match tpe {
                SigmaType::SColl(inner) => crate::inspect::type_str(inner),
                _ => "Byte".into(),
            },
            vs.iter()
                .map(|v| lift_const(&sigma_type_of(v), v, cx))
                .collect(),
        ),
        (_, SigmaValue::Tuple(vs)) => L::Tuple(
            vs.iter()
                .map(|v| lift_const(&SigmaType::SAny, v, cx))
                .collect(),
        ),
        (tpe, val) => {
            let _ = (tpe, val);
            L::Raw(crate::inspect::value_debug(tpe, val))
        }
    }
}

/// Best-effort static type of a value (for nested constant lifting).
fn sigma_type_of(v: &SigmaValue) -> SigmaType {
    match v {
        SigmaValue::Unit => SigmaType::SUnit,
        SigmaValue::Boolean(_) => SigmaType::SBoolean,
        SigmaValue::Byte(_) => SigmaType::SByte,
        SigmaValue::Short(_) => SigmaType::SShort,
        SigmaValue::Int(_) => SigmaType::SInt,
        SigmaValue::Long(_) => SigmaType::SLong,
        SigmaValue::BigInt(_) => SigmaType::SBigInt,
        SigmaValue::GroupElement(_) => SigmaType::SGroupElement,
        SigmaValue::SigmaProp(_) => SigmaType::SSigmaProp,
        SigmaValue::Coll(CollValue::BoolBits(_)) => SigmaType::SColl(Box::new(SigmaType::SBoolean)),
        SigmaValue::Coll(CollValue::Bytes(_)) => SigmaType::SColl(Box::new(SigmaType::SByte)),
        SigmaValue::Str(_) => SigmaType::SString,
        _ => SigmaType::SAny,
    }
}

fn method_lookup(type_id: u8, method_id: u8) -> Option<(&'static str, &'static str)> {
    METHOD_NAMES
        .iter()
        .find(|(t, m, ..)| *t == type_id && *m == method_id)
        .map(|(_, _, owner, name, _)| (*owner, *name))
}

/// Known single-arg "numeric" opcodes shared by ids 8..=13 across numeric types.
fn lift_method_like(
    type_id: u8,
    method_id: u8,
    obj: &Expr,
    args: &[Expr],
    cx: &mut LiftCtx,
    constants: &[(SigmaType, SigmaValue)],
) -> L {
    let obj_l = Box::new(lift(obj, cx, constants));
    let args_l: Vec<L> = args.iter().map(|a| lift(a, cx, constants)).collect();
    // Numeric casts render as obj.toByte etc. (wire ids 1..=5 on types 2..=6,9).
    if matches!(type_id, 2..=6 | 9) && matches!(method_id, 1..=5) {
        if let Some(name) = method_lookup(type_id, method_id) {
            return L::Method(obj_l, name.1.to_string(), vec![]);
        }
    }
    // Context box-collection properties compile to `CONTEXT.dataInputs` /
    // `CONTEXT.headers` (zero-arg); rendering them as methods keeps the
    // source form. Box accessor methods (Box.value etc.) are the same shape.
    match method_lookup(type_id, method_id) {
        Some((_owner, name)) => {
            // Single-arg methods on box/collection accessors (tokens(i),
            // dataInputs(i), getVar-like lookups) source as the apply form
            // `x(i)` — the compiler parses that back to ByIndex.
            if type_id == 63 && args_l.len() == 1 {
                return L::ApplyFn(Box::new(L::Method(obj_l, name.to_string(), vec![])), args_l);
            }
            // Box.getReg-v5 (99,7): the wire has no type byte; the source
            // form is the bracket-typed `getReg[T](idx)`. The type is not
            // recoverable from the wire — default to `Int`, which the
            // `getReg[..](…).isDefined` vectors use.
            if type_id == 99 && method_id == 7 && args_l.len() == 1 {
                return L::GetRegDyn(obj_l, "Int".into(), args_l);
            }
            L::Method(obj_l, name.to_string(), args_l)
        }
        None => {
            // Unknown method: honest raw fallback.
            L::Raw(format!(
                "<method 0x{type_id:02X}.0x{method_id:02X} on {}>",
                debug_expr(obj)
            ))
        }
    }
}

fn debug_expr(e: &Expr) -> String {
    let mut out = String::new();
    crate::inspect::fmt_expr_raw(e, &mut out);
    out
}

fn lift_op(
    node: &ergo_ser::opcode::IrNode,
    cx: &mut LiftCtx,
    constants: &[(SigmaType, SigmaValue)],
) -> L {
    lift_op_inner(node, cx, constants, false)
}

fn lift_op_inner(
    node: &ergo_ser::opcode::IrNode,
    cx: &mut LiftCtx,
    constants: &[(SigmaType, SigmaValue)],
    root_d1: bool,
) -> L {
    let op = node.opcode;
    let payload = &node.payload;
    let debug = || debug_expr(&Expr::Op(node.clone()));
    // Infix operators — the wire parses comparisons/booleans as `Payload::Two`
    // (the packed-bool 0x85 form only appears for `Coll[Boolean]` constants).
    if let Some((sym, prec)) = infix_op(op) {
        if let Payload::Two(a, b) = payload {
            return L::Infix(
                sym,
                prec,
                Box::new(lift(a, cx, constants)),
                Box::new(lift(b, cx, constants)),
            );
        }
    }
    match payload {
        Payload::Zero => L::Leaf(match op {
            0x7F => "true",
            0x80 => "false",
            0xA3 => "HEIGHT",
            0xA4 => "INPUTS",
            0xA5 => "OUTPUTS",
            0xA7 => "SELF",
            0xAC => "MinerPubKey",
            0xA6 => "LastBlockUtxoRootHash",
            0xDD => "Global",
            0xFE => "CONTEXT",
            0x82 => "groupGenerator",
            _ => return L::Raw(format!("<op 0x{op:02X}>")),
        }),
        Payload::One(inner) => {
            let inner_l = lift(inner, cx, constants);
            match op {
                0xD1 => {
                    // BoolToSigmaProp is implicit at the tree ROOT (the
                    // compiler re-adds it on recompile) but EXPLICIT inside
                    // sigma collections (a bare bool child would recompile
                    // as a bool BinAnd, changing the tree shape).
                    if root_d1 {
                        inner_l
                    } else {
                        L::Global("sigmaProp".into(), vec![inner_l])
                    }
                }
                0xEF => L::Unary("!", Box::new(inner_l)),
                0xF0 => L::Unary("-", Box::new(inner_l)),
                0xE4 => L::Method(Box::new(inner_l), "get".into(), vec![]),
                0xE6 => {
                    // OptionIsDefined over a register read must source as
                    // `getReg[T](n).isDefined` — the `R5[T]` accessor form
                    // unwraps the Option in the source type system.
                    let reg_form = match inner_l {
                        L::Prop(ref obj, ref name)
                            if name.starts_with('R') && name.contains('[') =>
                        {
                            let n = &name[1..name.find('[').unwrap_or(1)];
                            let t = &name[name.find('[').unwrap_or(0) + 1..name.len() - 1];
                            Some((obj.clone(), n.to_string(), t.to_string()))
                        }
                        // R0 → `value` (register 0, type unknown at this
                        // point — source `value` is R0 unwrapped; getReg
                        // keeps the Option. The wire D1(E6(C6(A7,0,Int)))
                        // sources ONLY as getReg-form.
                        L::Prop(ref obj, ref name) if name == "value" => {
                            Some((obj.clone(), "0".to_string(), "Int".to_string()))
                        }
                        _ => None,
                    };
                    match reg_form {
                        Some((obj, n, t)) => {
                            L::GetReg(obj, t.to_string(), n.parse().unwrap_or(0)).into_is_defined()
                        }
                        None => L::Method(Box::new(inner_l), "isDefined".into(), vec![]),
                    }
                }
                0xB1 => L::Method(Box::new(inner_l), "size".into(), vec![]),
                0xC1 => L::Prop(Box::new(inner_l), "value".into()),
                0xC2 => L::Prop(Box::new(inner_l), "propositionBytes".into()),
                0xC3 => L::Prop(Box::new(inner_l), "bytes".into()),
                0xC4 => L::Prop(Box::new(inner_l), "bytesWithoutRef".into()),
                0xC5 => L::Prop(Box::new(inner_l), "id".into()),
                0xC7 => L::Method(Box::new(inner_l), "creationInfo".into(), vec![]),
                0xCD => match inner_l {
                    // ProveDlog(x): source predef `proveDlog(x)`. A bare GE
                    // constant already prints as PK(…) (sigma-typed by
                    // construction); any COMPUTED argument (val, var,
                    // method result) needs the explicit proveDlog(…).
                    L::Val(_) | L::GetVar(..) | L::Method(..) => {
                        L::Global("proveDlog".into(), vec![inner_l])
                    }
                    other => other, // a bare ProveDlog leaf prints as PK(…)
                },
                0xCB => L::Global("blake2b256".into(), vec![inner_l]),
                0xCC => L::Global("sha256".into(), vec![inner_l]),
                0x7A => L::Global("longToByteArray".into(), vec![inner_l]),
                0x7B => L::Global("byteArrayToBigInt".into(), vec![inner_l]),
                0x7C => L::Global("byteArrayToLong".into(), vec![inner_l]),
                0xCF => L::Const("isProven".into()),
                0xD0 => L::Method(Box::new(inner_l), "propBytes".into(), vec![]),
                0xEE => L::Global("decodePoint".into(), vec![inner_l]),
                0xFF => match inner_l {
                    // xorOf's source predef takes a Coll: `xorOf(Coll(a, b))`.
                    L::Coll(t, items) => L::Global("xorOf".into(), vec![L::Coll(t, items)]),
                    other => L::Global("xorOf".into(), vec![other]),
                },
                0x96 => match inner_l {
                    // allOf's source predef takes a Coll argument.
                    L::Coll(t, items) => L::Global("allOf".into(), vec![L::Coll(t, items)]),
                    other => L::Global("allOf".into(), vec![other]),
                },
                0x97 => match inner_l {
                    L::Coll(t, items) => L::Global("anyOf".into(), vec![L::Coll(t, items)]),
                    other => L::Global("anyOf".into(), vec![other]),
                },
                _ => L::Raw(debug()),
            }
        }
        Payload::Two(a, b) => {
            let al = lift(a, cx, constants);
            let bl = lift(b, cx, constants);
            match op {
                0x98 => {
                    // AtLeast(k, Coll(props)): the children must be
                    // SigmaProp-typed — bool comparisons need explicit
                    // `sigmaProp(…)` wrapping to recompile.
                    let wrapped = match bl {
                        L::Coll(t, items) => {
                            L::Coll(t, items.into_iter().map(wrap_sigma).collect())
                        }
                        other => other,
                    };
                    L::AtLeast(Box::new(al), Box::new(wrapped))
                }
                0x9B => L::Infix("xorBytes", 6, Box::new(al), Box::new(bl)),
                0x9F => L::Method(Box::new(al), "exp".into(), vec![bl]),
                0xA0 => L::Method(Box::new(al), "multiply".into(), vec![bl]),
                0xA1 => L::Global("min".into(), vec![al, bl]),
                0xA2 => L::Global("max".into(), vec![al, bl]),
                0xAD => L::Method(Box::new(al), "map".into(), vec![bl]),
                0xAE => L::Method(Box::new(al), "exists".into(), vec![bl]),
                0xAF => L::Method(Box::new(al), "forall".into(), vec![bl]),
                0xB3 => L::Infix("++", 7, Box::new(al), Box::new(bl)),
                0xB5 => L::Method(Box::new(al), "filter".into(), vec![bl]),
                0xE5 => L::Method(Box::new(al), "getOrElse".into(), vec![bl]),
                _ => L::Raw(debug()),
            }
        }
        Payload::Three(a, b, c) => match op {
            0x95 => L::If(
                Box::new(lift(a, cx, constants)),
                Box::new(lift(b, cx, constants)),
                Box::new(lift(c, cx, constants)),
            ),
            0xB4 => {
                // Slice(input, from, until): source `input.slice(from, until)`.
                let coll = lift(a, cx, constants);
                let from = lift(b, cx, constants);
                let until = lift(c, cx, constants);
                L::Method(Box::new(coll), "slice".into(), vec![from, until])
            }
            0xB0 => {
                // Fold(input, zero, foldOp): source `input.fold(zero, lambda)`
                let coll = lift(a, cx, constants);
                let zero = lift(b, cx, constants);
                let lam = lift(c, cx, constants);
                L::Method(Box::new(coll), "fold".into(), vec![zero, lam])
            }
            _ => L::Raw(debug()),
        },
        Payload::Four(..) => L::Raw(debug()),
        Payload::ValUse { id } => L::Val(cx.lookup(*id).unwrap_or_else(|| format!("%{id}"))),
        Payload::ConstPlaceholder { index } => match constants.get(*index as usize) {
            Some((tpe, val)) => lift_const(tpe, val, cx),
            None => L::Raw(format!("$<bad {}>", index)),
        },
        Payload::TaggedVar { id, .. } => L::GetVar(*id as i64, String::new()),
        Payload::ValDef { id, rhs, .. } => {
            let name = cx.bind(*id, "val");
            let _ = name;
            let rhs_l = lift(rhs, cx, constants);
            L::Block(
                vec![Stmt::Val(cx.lookup(*id).unwrap_or_default(), rhs_l)],
                Box::new(L::Val(cx.lookup(*id).unwrap_or_default())),
            )
        }
        Payload::FunDef { id, rhs, .. } => {
            let name = cx.bind(*id, "fn");
            L::Block(
                vec![Stmt::Def(name.clone(), lift(rhs, cx, constants))],
                Box::new(L::Val(name)),
            )
        }
        Payload::BlockValue { items, result } => {
            cx.push_scope();
            let mut stmts = Vec::with_capacity(items.len());
            let mut bindings: BTreeMap<String, L> = BTreeMap::new();
            for item in items {
                if let Expr::Op(n) = item {
                    match &n.payload {
                        Payload::ValDef { id, rhs, .. } => {
                            let name = cx.bind(*id, "v");
                            bindings.insert(name.clone(), lift(rhs, cx, constants));
                        }
                        Payload::FunDef { id, rhs, .. } => {
                            let name = cx.bind(*id, "f");
                            bindings.insert(name.clone(), lift(rhs, cx, constants));
                        }
                        _ => {}
                    }
                }
            }
            // Render statements in binding order, resolving later bindings.
            for item in items {
                if let Expr::Op(n) = item {
                    match &n.payload {
                        Payload::ValDef { id, .. } => {
                            let name = cx.lookup(*id).unwrap_or_default();
                            if let Some(rhs) = bindings.get(&name) {
                                stmts.push(Stmt::Val(name.clone(), rhs.clone()));
                            }
                        }
                        Payload::FunDef { id, .. } => {
                            let name = cx.lookup(*id).unwrap_or_default();
                            if let Some(rhs) = bindings.get(&name) {
                                stmts.push(Stmt::Def(name.clone(), rhs.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            let result_l = lift(result, cx, constants);
            cx.pop_scope();
            if stmts.is_empty() {
                result_l
            } else {
                L::Block(stmts, Box::new(result_l))
            }
        }
        Payload::FuncValue { args, body } => {
            cx.push_scope();
            // Fold's wire lambda wraps (acc, elem) into a SINGLE tuple-typed
            // arg (`(t: (Long, Box)) => t._1 + t._2.value`) — the compiler
            // re-wraps a 2-arg source lambda on emit. Unwrap: render the
            // 2-arg source form so recompilation reproduces the wire.
            if args.len() == 1 {
                if let Some(SigmaType::STuple(field_types)) = &args[0].1 {
                    let (id, _) = &args[0];
                    let n1 = cx.fresh("t");
                    let n2 = cx.fresh("t");
                    let names = vec![
                        format!("{}: {}", n1, crate::inspect::type_str(&field_types[0])),
                        format!("{}: {}", n2, crate::inspect::type_str(&field_types[1])),
                    ];
                    let bound = cx.bind(*id, "t");
                    let mut body_l = lift(body, cx, constants);
                    body_l = rewrite_fold_fields(body_l, &bound, &n1, &n2);
                    cx.pop_scope();
                    return L::Lambda(names, Box::new(body_l));
                }
            }
            let mut names = Vec::with_capacity(args.len());
            for (id, t) in args {
                let name = cx.bind(*id, "x");
                names.push(match t {
                    // Annotate lambda args: `{ (x: Box) => … }` — the
                    // compiler cannot infer untyped lambda param types
                    // when the lambda is bound to a `val` first.
                    Some(tpe) => format!("{}: {}", name, crate::inspect::type_str(tpe)),
                    None => name,
                });
            }
            let body_l = lift(body, cx, constants);
            cx.pop_scope();
            L::Lambda(names, Box::new(body_l))
        }
        Payload::MethodCall {
            type_id,
            method_id,
            obj,
            args,
            type_args,
        } => {
            // Box.getReg[T] v6 form (99,19): the wire carries the element
            // type as an explicit type arg — render `obj.getReg[T](idx)`.
            if *type_id == 99 && *method_id == 19 {
                if let Some(t0) = type_args.first() {
                    return L::GetRegDyn(
                        Box::new(lift(obj, cx, constants)),
                        crate::inspect::type_str(t0),
                        args.iter().map(|a| lift(a, cx, constants)).collect(),
                    );
                }
            }
            lift_method_like(*type_id, *method_id, obj, args, cx, constants)
        }
        Payload::ConcreteCollection { elem_type, items } => L::Coll(
            crate::inspect::type_str(elem_type),
            items.iter().map(|i| lift(i, cx, constants)).collect(),
        ),
        Payload::BoolCollection { bits } => {
            L::Coll("Boolean".into(), bits.iter().map(|b| L::Bool(*b)).collect())
        }
        Payload::Tuple { items } => {
            L::Tuple(items.iter().map(|i| lift(i, cx, constants)).collect())
        }
        Payload::SelectField { input, field_idx } => {
            let obj = Box::new(lift(input, cx, constants));
            L::Prop(obj, format!("_{}", field_idx))
        }
        Payload::ExtractRegisterAs { input, reg_id, tpe } => {
            let obj = Box::new(lift(input, cx, constants));
            // R0 sources as `.value`; other registers as `R4[T]`-style
            // typed accessors — EXCEPT when the register read's Option-ness
            // is observed (`.isDefined`/`.get` on the raw read), which only
            // the explicit `getReg[T](n)` form preserves. We can't see the
            // parent here, so always use `R4[T]` accessor + `.get` — but for
            // the seed corpus's `getReg[T](n).isDefined` vectors the wire
            // shape is identical (0xE6 0xC6 …), so prefer the accessor form
            // that round-trips: `R5[T]` recompiles to ExtractRegisterAs and
            // `R5[T].get` to OptionGet(ExtractRegisterAs) — the exact wire.
            // The `getReg[T](n).isDefined` vector is byte-equal to
            // `R5[T].isDefined` ONLY if R5[T] stays an Option — it doesn't.
            // So: render the accessor form; the isDefined cases will
            // recompile from `getReg[T](n)` — keep those distinct by
            // checking whether this node's parent expects an Option.
            // Practically: `getReg` form is UNAMBIGUOUS for both shapes —
            // `OUTPUTS(0).getReg[Long](5).get` compiles to the same bytes
            // as `R5[Long].get`? Verified below by the round-trip test;
            // until then prefer the explicit getReg form (it's what the
            // wire stores) and let `.get` produce OptionGet(ExtractRegAs).
            if *reg_id == 0 {
                L::Prop(obj, "value".into())
            } else {
                L::Prop(
                    obj,
                    format!("R{}[{}]", reg_id, crate::inspect::type_str(tpe)),
                )
            }
        }
        Payload::GetVar { var_id, tpe } => L::GetVar(*var_id as i64, crate::inspect::type_str(tpe)),
        Payload::DeserializeContext { id, .. } => {
            L::Global("deserializeContext".into(), vec![L::Int(*id as i64)])
        }
        Payload::DeserializeRegister {
            reg_id, default, ..
        } => {
            let mut args = vec![L::Int(*reg_id as i64)];
            if let Some(d) = default {
                args.push(lift(d, cx, constants));
            }
            L::Global("deserializeRegister".into(), args)
        }
        Payload::SigmaCollection { items } => {
            // SigmaAnd/SigmaOr recompile from `&&`/`||` chains over sigma
            // children (0xED BinAnd on SigmaProps lifts to 0xEA on compile).
            let items_l: Vec<L> = items.iter().map(|i| lift(i, cx, constants)).collect();
            let (sym, prec) = if op == 0xEA { ("&&", 2u8) } else { ("||", 1u8) };
            match items_l.len() {
                0 => L::Const(if op == 0xEA { "true" } else { "false" }.into()),
                1 => items_l.into_iter().next().expect("len 1"),
                _ => {
                    let mut it = items_l.into_iter();
                    let first = it.next().expect("non-empty");
                    it.fold(first, |acc, item| {
                        L::Infix(sym, prec, Box::new(acc), Box::new(item))
                    })
                }
            }
        }
        Payload::NoneValue { .. } => L::Const("None".into()),
        Payload::ByIndex {
            input,
            index,
            default,
        } => {
            let input_l = lift(input, cx, constants);
            let index_l = lift(index, cx, constants);
            // `obj[i]` re-parses ONLY when obj is a plain identifier/leaf;
            // on a method-call receiver (OUTPUTS(0).tokens, CONTEXT.dataInputs)
            // the parser needs the apply form `obj(i)`. Uniform + safe: use
            // the apply form everywhere (the compiler parses `coll(i)` on
            // collection-typed receivers as ByIndex).
            if let Some(d) = default {
                return L::Method(
                    Box::new(L::Index(Box::new(input_l), Box::new(index_l), None)),
                    "getOrElse".into(),
                    vec![lift(d, cx, constants)],
                );
            }
            match input_l {
                // Box-collection indexing: `OUTPUTS(i)`, `tokens(i)`. A bound
                // `val` over them (v2[0]) hits the same parser constraint.
                L::Method(..) | L::Leaf("OUTPUTS") | L::Leaf("INPUTS") | L::Val(_) => {
                    L::ApplyFn(Box::new(input_l), vec![index_l])
                }
                _ => L::Index(Box::new(input_l), Box::new(index_l), None),
            }
        }
        Payload::NumericCast { input, tpe } => match cast_name(tpe) {
            Some(name) => L::Method(Box::new(lift(input, cx, constants)), name.into(), vec![]),
            None => L::Raw(debug()),
        },
        Payload::FuncApply { func, args } => {
            let f = Box::new(lift(func, cx, constants));
            let args_l: Vec<L> = args.iter().map(|a| lift(a, cx, constants)).collect();
            L::ApplyFn(f, args_l)
        }
    }
}

/// A Relation2 payload may be the packed-bool form; treat `None` arms.
trait IntoIsDefined {
    fn into_is_defined(self) -> L;
}
impl IntoIsDefined for L {
    fn into_is_defined(self) -> L {
        L::Method(Box::new(self), "isDefined".into(), vec![])
    }
}

/// Rewrite `Prop(Val(bound), "_1"/"_2")` to the fresh fold-field names
/// (fold tuple-unwrap: the wire's 1-arg tuple lambda back to source 2-arg).
fn rewrite_fold_fields(e: L, bound: &str, n1: &str, n2: &str) -> L {
    match e {
        L::Prop(obj, f) if f == "_1" || f == "_2" => {
            let rewritten = rewrite_fold_fields(*obj, bound, n1, n2);
            match rewritten {
                L::Val(ref name) if name == bound => {
                    L::Val((if f == "_1" { n1 } else { n2 }).to_string())
                }
                other => L::Prop(Box::new(other), f),
            }
        }
        L::Prop(obj, f) => L::Prop(Box::new(rewrite_fold_fields(*obj, bound, n1, n2)), f),
        L::Method(obj, n, args) => L::Method(
            Box::new(rewrite_fold_fields(*obj, bound, n1, n2)),
            n,
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        L::Infix(sym, prec, a, b) => L::Infix(
            sym,
            prec,
            Box::new(rewrite_fold_fields(*a, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*b, bound, n1, n2)),
        ),
        L::Unary(op, a) => L::Unary(op, Box::new(rewrite_fold_fields(*a, bound, n1, n2))),
        L::Global(n, args) => L::Global(
            n,
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        L::Coll(t, items) => L::Coll(
            t,
            items
                .into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        L::Tuple(items) => L::Tuple(
            items
                .into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        L::ApplyFn(f, args) => L::ApplyFn(
            Box::new(rewrite_fold_fields(*f, bound, n1, n2)),
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        L::Block(stmts, result) => L::Block(
            stmts
                .into_iter()
                .map(|st| match st {
                    Stmt::Val(n, e) => Stmt::Val(n, rewrite_fold_fields(e, bound, n1, n2)),
                    Stmt::Def(n, e) => Stmt::Def(n, rewrite_fold_fields(e, bound, n1, n2)),
                })
                .collect(),
            Box::new(rewrite_fold_fields(*result, bound, n1, n2)),
        ),
        L::If(c, t, els) => L::If(
            Box::new(rewrite_fold_fields(*c, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*t, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*els, bound, n1, n2)),
        ),
        L::Lambda(args, body) => {
            L::Lambda(args, Box::new(rewrite_fold_fields(*body, bound, n1, n2)))
        }
        L::Index(a, b, d) => L::Index(
            Box::new(rewrite_fold_fields(*a, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*b, bound, n1, n2)),
            d.map(|x| Box::new(rewrite_fold_fields(*x, bound, n1, n2))),
        ),
        L::AtLeast(k, c) => L::AtLeast(
            Box::new(rewrite_fold_fields(*k, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*c, bound, n1, n2)),
        ),
        L::GetReg(o, t, n) => L::GetReg(Box::new(rewrite_fold_fields(*o, bound, n1, n2)), t, n),
        L::GetRegDyn(o, t, args) => L::GetRegDyn(
            Box::new(rewrite_fold_fields(*o, bound, n1, n2)),
            t,
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        other => other,
    }
}

/// Wrap a lifted expression in `sigmaProp(…)` when it isn't already
/// sigma-typed (AtLeast/allOf children must be SigmaProps).
fn wrap_sigma(e: L) -> L {
    match e {
        // Already sigma-ish: sigmaProp calls, PK constants, proveDlog,
        // AtLeast, sigma and/or.
        L::Global(ref name, _) if name == "sigmaProp" || name == "proveDlog" => e,
        L::AtLeast(..) => e,
        // Everything else (bool comparisons, and-chains of bools) wraps.
        _ => L::Global("sigmaProp".into(), vec![e]),
    }
}
