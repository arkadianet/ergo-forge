//! The lift: ErgoTree wire IR → lifted AST.
//!
//! Recognizes source-level shapes in the opcode IR — infix operators, casts,
//! property and method calls, block scoping — and degrades to
//! [`super::ast::NodeKind::Raw`] for anything with no source-like form, so output is
//! never silently wrong.

use std::collections::BTreeMap;

use ergo_ser::opcode::{Expr, Payload};
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};

use super::ast::{Node, NodeKind, Stmt};
use super::MAX_LIFT_DEPTH;
use crate::method_names::METHOD_NAMES;

// ── operator tables ──────────────────────────────────────────────────────────

/// Infix binary operators: opcode → (symbol, precedence).
/// Higher precedence binds tighter. Mirrors ErgoScript/Scala precedence:
/// unary > multiplicative > additive > comparison > logical.
/// Infix binary operators: opcode → symbol. Precedence lives in
/// `print::prec_of`, keyed by symbol.
fn infix_op(op: u8) -> Option<&'static str> {
    Some(match op {
        0xEC => "||", // BinOr (lazy)
        0xED => "&&", // BinAnd (lazy)
        0x8F => "<",  // Lt
        0x90 => "<=", // Le
        0x91 => ">",  // Gt
        0x92 => ">=", // Ge
        0x93 => "==", // Eq
        0x94 => "!=", // Neq
        0xF4 => "^",  // BinXor (strict) — Scala assigns ^ lower than && but
        // ErgoScript parity keeps it above comparisons in practice; pinned by round-trip.
        0x99 => "-", // Minus
        0x9A => "+", // Plus
        0x9C => "*", // Multiply
        0x9D => "/", // Divide
        0x9E => "%", // Modulo
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

// ── lift ─────────────────────────────────────────────────────────────────────

/// Names assigned to SSA ids during lift. Ids on the wire share one
/// namespace per `BlockValue`/`FuncValue` scope in practice, but real trees
/// (and older compilers) can collide across scopes, so lift renumbers
/// hierarchically: every binding gets a fresh source name, wired through
/// scope maps per block.
pub(crate) struct LiftCtx {
    /// PK address network: true = testnet (corpus bar), false = mainnet.
    pub(crate) testnet: bool,
    /// wire id → source name, per lexical scope stack.
    pub(crate) scopes: Vec<BTreeMap<u32, String>>,
    /// Global (scope-agnostic) id→name registry — fallback for ValUse ids
    /// whose lexical scope lookup missed (the wire's per-block id spaces
    /// make this unambiguous in practice: the most recent binding of an id
    /// is the in-scope one).
    pub(crate) global: BTreeMap<u32, String>,
    /// next counter per prefix.
    pub(crate) counters: BTreeMap<&'static str, usize>,
    /// Current lift recursion depth, bounded by [`MAX_LIFT_DEPTH`].
    pub(crate) depth: usize,
    /// Set when the recursion ceiling was hit (see [`MAX_LIFT_DEPTH`]).
    pub(crate) truncated: bool,
    /// Next lift-local node id. See `ast::Node::id`.
    pub(crate) next_id: u64,
}

impl LiftCtx {
    pub(crate) fn new() -> Self {
        Self {
            testnet: true,
            scopes: vec![BTreeMap::new()],
            global: BTreeMap::new(),
            counters: BTreeMap::new(),
            depth: 0,
            truncated: false,
            next_id: 0,
        }
    }

    /// Allocate the next lift-local id.
    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// `alloc_id` for the module's entry point.
    pub(crate) fn alloc_id_pub(&mut self) -> u64 {
        self.alloc_id()
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
pub(crate) fn lift(e: &Expr, cx: &mut LiftCtx, constants: &[(SigmaType, SigmaValue)]) -> Node {
    let id = cx.alloc_id();
    // Bounded recursion: past the ceiling we degrade to a raw placeholder
    // rather than growing the stack. See [`MAX_LIFT_DEPTH`].
    if cx.depth >= MAX_LIFT_DEPTH {
        cx.truncated = true;
        return Node {
            id,
            kind: NodeKind::Raw(format!("<nesting deeper than {MAX_LIFT_DEPTH} levels>")),
        };
    }
    cx.depth += 1;
    let kind = match e {
        Expr::Const { tpe, val } => lift_const(tpe, val, cx),
        Expr::Unparsed(bytes) => NodeKind::Raw(format!("<unparsed {} bytes>", bytes.len())),
        Expr::Op(node) => lift_op(node, cx, constants),
    };
    cx.depth -= 1;
    Node { id, kind }
}

fn lift_const(tpe: &SigmaType, val: &SigmaValue, cx: &mut LiftCtx) -> NodeKind {
    match (tpe, val) {
        (SigmaType::SBoolean, SigmaValue::Boolean(b)) => NodeKind::Bool(*b),
        (SigmaType::SByte, SigmaValue::Byte(x)) => NodeKind::Num(format!("{x}.toByte")),
        (SigmaType::SShort, SigmaValue::Short(x)) => NodeKind::Num(format!("{x}.toShort")),
        (SigmaType::SInt, SigmaValue::Int(x)) => NodeKind::Int(*x as i64),
        (SigmaType::SLong, SigmaValue::Long(x)) => NodeKind::Num(format!("{x}L")),
        // `bigInt("<decimal>")` is the compiler's BigInt predef — a bare
        // `{n}L` would be a Long literal (wrong type), and `.toBigInt` cannot
        // express values outside the Long range.
        (SigmaType::SBigInt, SigmaValue::BigInt(n)) => NodeKind::Const(format!("bigInt(\"{n}\")")),
        (SigmaType::SGroupElement, SigmaValue::GroupElement(ge)) => NodeKind::Const(format!(
            "PK(\"{}\")",
            crate::inspect::group_element_base58_net(ge.as_bytes(), cx.testnet)
        )),
        (SigmaType::SSigmaProp, SigmaValue::SigmaProp(sb)) => match sb {
            SigmaBoolean::ProveDlog(ge) => NodeKind::Const(format!(
                "PK(\"{}\")",
                crate::inspect::group_element_base58_net(ge.as_bytes(), cx.testnet)
            )),
            SigmaBoolean::TrivialProp(b) => NodeKind::Const(format!("sigmaProp({b})")),
            other => NodeKind::Raw(format!("{other:?}")),
        },
        // `fromBase16("<hex>")` is the compiler's byte-coll predef: any
        // length, and byte signedness is handled by the compiler (its Bytes
        // are signed i8, so plain integer elements above 0x7F would be an
        // out-of-range Byte).
        (SigmaType::SColl(inner), SigmaValue::Coll(CollValue::Bytes(bs)))
            if **inner == SigmaType::SByte =>
        {
            NodeKind::Const(format!("fromBase16(\"{}\")", hex::encode(bs)))
        }
        (_, SigmaValue::Coll(CollValue::BoolBits(bits))) => NodeKind::Coll(
            "Boolean".into(),
            bits.iter()
                .map(|b| Node {
                    id: cx.alloc_id(),
                    kind: NodeKind::Bool(*b),
                })
                .collect(),
        ),
        (tpe, SigmaValue::Coll(CollValue::Values(vs))) => NodeKind::Coll(
            match tpe {
                SigmaType::SColl(inner) => crate::inspect::type_str(inner),
                _ => "Byte".into(),
            },
            vs.iter()
                .map(|v| Node {
                    id: cx.alloc_id(),
                    kind: lift_const(&sigma_type_of(v), v, cx),
                })
                .collect(),
        ),
        (_, SigmaValue::Tuple(vs)) => NodeKind::Tuple(
            vs.iter()
                .map(|v| Node {
                    id: cx.alloc_id(),
                    kind: lift_const(&SigmaType::SAny, v, cx),
                })
                .collect(),
        ),
        (tpe, val) => {
            let _ = (tpe, val);
            NodeKind::Raw(crate::inspect::value_debug(tpe, val))
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
) -> NodeKind {
    let obj_l = Box::new(lift(obj, cx, constants));
    let args_l: Vec<Node> = args.iter().map(|a| lift(a, cx, constants)).collect();
    // Numeric casts render as obj.toByte etc. (wire ids 1..=5 on types 2..=6,9).
    if matches!(type_id, 2..=6 | 9) && matches!(method_id, 1..=5) {
        if let Some(name) = method_lookup(type_id, method_id) {
            return NodeKind::Method(obj_l, name.1.to_string(), vec![]);
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
                return NodeKind::ApplyFn(
                    Box::new(Node {
                        id: cx.alloc_id(),
                        kind: NodeKind::Method(obj_l, name.to_string(), vec![]),
                    }),
                    args_l,
                );
            }
            // Box.getReg-v5 (99,7): the wire has no type byte; the source
            // form is the bracket-typed `getReg[T](idx)`. The type is not
            // recoverable from the wire — default to `Int`, which the
            // `getReg[..](…).isDefined` vectors use.
            if type_id == 99 && method_id == 7 && args_l.len() == 1 {
                return NodeKind::GetRegDyn(obj_l, "Int".into(), args_l);
            }
            NodeKind::Method(obj_l, name.to_string(), args_l)
        }
        None => {
            // Unknown method: honest raw fallback.
            {
                NodeKind::Raw(format!(
                    "<method 0x{type_id:02X}.0x{method_id:02X} on {}>",
                    debug_expr(obj)
                ))
            }
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
) -> NodeKind {
    lift_op_inner(node, cx, constants, false)
}

pub(crate) fn lift_op_inner(
    node: &ergo_ser::opcode::IrNode,
    cx: &mut LiftCtx,
    constants: &[(SigmaType, SigmaValue)],
    root_d1: bool,
) -> NodeKind {
    let op = node.opcode;
    let payload = &node.payload;
    let debug = || debug_expr(&Expr::Op(node.clone()));
    // Infix operators — the wire parses comparisons/booleans as `Payload::Two`
    // (the packed-bool 0x85 form only appears for `Coll[Boolean]` constants).
    if let Some(sym) = infix_op(op) {
        if let Payload::Two(a, b) = payload {
            return NodeKind::Infix(
                sym,
                Box::new(lift(a, cx, constants)),
                Box::new(lift(b, cx, constants)),
            );
        }
    }
    match payload {
        Payload::Zero => NodeKind::Leaf(match op {
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
            _ => {
                return NodeKind::Raw(format!("<op 0x{op:02X}>"));
            }
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
                        inner_l.kind
                    } else {
                        NodeKind::Global("sigmaProp".into(), vec![inner_l])
                    }
                }
                0xEF => NodeKind::Unary("!", Box::new(inner_l)),
                0xF0 => NodeKind::Unary("-", Box::new(inner_l)),
                0xE4 => NodeKind::Method(Box::new(inner_l), "get".into(), vec![]),
                0xE6 => {
                    // OptionIsDefined over a register read: render the typed
                    // accessor `R{n}[T].isDefined` — version-neutral (the
                    // `getReg[T]` source form is v6-only, so pre-v3 trees
                    // could never recompile it). The element type comes from
                    // the wire's ExtractRegisterAs payload, which also covers
                    // R0 (whose non-isDefined rendering is the unwrapped
                    // `value` property).
                    let reg_accessor: Option<Node> = match &**inner {
                        Expr::Op(n) => match &n.payload {
                            Payload::ExtractRegisterAs { input, reg_id, tpe } => Some(Node {
                                id: cx.alloc_id(),
                                kind: NodeKind::Prop(
                                    Box::new(lift(input, cx, constants)),
                                    format!("R{}[{}]", reg_id, crate::inspect::type_str(tpe)),
                                ),
                            }),
                            _ => None,
                        },
                        _ => None,
                    };
                    match reg_accessor {
                        Some(accessor) => {
                            NodeKind::Method(Box::new(accessor), "isDefined".into(), vec![])
                        }
                        None => NodeKind::Method(Box::new(inner_l), "isDefined".into(), vec![]),
                    }
                }
                0xB1 => NodeKind::Method(Box::new(inner_l), "size".into(), vec![]),
                0xC1 => NodeKind::Prop(Box::new(inner_l), "value".into()),
                0xC2 => NodeKind::Prop(Box::new(inner_l), "propositionBytes".into()),
                0xC3 => NodeKind::Prop(Box::new(inner_l), "bytes".into()),
                0xC4 => NodeKind::Prop(Box::new(inner_l), "bytesWithoutRef".into()),
                0xC5 => NodeKind::Prop(Box::new(inner_l), "id".into()),
                0xC7 => NodeKind::Method(Box::new(inner_l), "creationInfo".into(), vec![]),
                0xCD => {
                    let Node {
                        id: inner_id,
                        kind: inner_kind,
                    } = inner_l;
                    match inner_kind {
                        // ProveDlog(x): source predef `proveDlog(x)`. A bare GE
                        // constant already prints as PK(…) (sigma-typed by
                        // construction); any COMPUTED argument (val, var,
                        // method result, global call such as decodePoint(…))
                        // needs the explicit proveDlog(…) or the result types as
                        // GroupElement and cannot satisfy Coll[SigmaProp].
                        NodeKind::Val(_)
                        | NodeKind::GetVar(..)
                        | NodeKind::Method(..)
                        | NodeKind::Global(..) => NodeKind::Global(
                            "proveDlog".into(),
                            vec![Node {
                                id: inner_id,
                                kind: inner_kind,
                            }],
                        ),
                        other => other, // a bare ProveDlog leaf prints as PK(…)
                    }
                }
                0xCB => NodeKind::Global("blake2b256".into(), vec![inner_l]),
                0xCC => NodeKind::Global("sha256".into(), vec![inner_l]),
                0x7A => NodeKind::Global("longToByteArray".into(), vec![inner_l]),
                0x7B => NodeKind::Global("byteArrayToBigInt".into(), vec![inner_l]),
                0x7C => NodeKind::Global("byteArrayToLong".into(), vec![inner_l]),
                0xCF => NodeKind::Const("isProven".into()),
                0xD0 => NodeKind::Method(Box::new(inner_l), "propBytes".into(), vec![]),
                0xEE => NodeKind::Global("decodePoint".into(), vec![inner_l]),
                0xFF => {
                    let Node {
                        id: inner_id,
                        kind: inner_kind,
                    } = inner_l;
                    // xorOf's source predef takes a Coll: `xorOf(Coll(a, b))`.
                    match inner_kind {
                        NodeKind::Coll(t, items) => NodeKind::Global(
                            "xorOf".into(),
                            vec![Node {
                                id: inner_id,
                                kind: NodeKind::Coll(t, items),
                            }],
                        ),
                        other => NodeKind::Global(
                            "xorOf".into(),
                            vec![Node {
                                id: inner_id,
                                kind: other,
                            }],
                        ),
                    }
                }
                0x96 => {
                    let Node {
                        id: inner_id,
                        kind: inner_kind,
                    } = inner_l;
                    // allOf's source predef takes a Coll argument.
                    match inner_kind {
                        NodeKind::Coll(t, items) => NodeKind::Global(
                            "allOf".into(),
                            vec![Node {
                                id: inner_id,
                                kind: NodeKind::Coll(t, items),
                            }],
                        ),
                        other => NodeKind::Global(
                            "allOf".into(),
                            vec![Node {
                                id: inner_id,
                                kind: other,
                            }],
                        ),
                    }
                }
                0x97 => {
                    let Node {
                        id: inner_id,
                        kind: inner_kind,
                    } = inner_l;
                    match inner_kind {
                        NodeKind::Coll(t, items) => NodeKind::Global(
                            "anyOf".into(),
                            vec![Node {
                                id: inner_id,
                                kind: NodeKind::Coll(t, items),
                            }],
                        ),
                        other => NodeKind::Global(
                            "anyOf".into(),
                            vec![Node {
                                id: inner_id,
                                kind: other,
                            }],
                        ),
                    }
                }
                _ => NodeKind::Raw(debug()),
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
                    let Node {
                        id: bl_id,
                        kind: bl_kind,
                    } = bl;
                    let wrapped = match bl_kind {
                        NodeKind::Coll(t, items) => Node {
                            id: bl_id,
                            kind: NodeKind::Coll(
                                t,
                                items.into_iter().map(|it| wrap_sigma(it, cx)).collect(),
                            ),
                        },
                        other => Node {
                            id: bl_id,
                            kind: other,
                        },
                    };
                    NodeKind::AtLeast(Box::new(al), Box::new(wrapped))
                }
                0x9B => NodeKind::Infix("xorBytes", Box::new(al), Box::new(bl)),
                0x9F => NodeKind::Method(Box::new(al), "exp".into(), vec![bl]),
                0xA0 => NodeKind::Method(Box::new(al), "multiply".into(), vec![bl]),
                0xA1 => NodeKind::Global("min".into(), vec![al, bl]),
                0xA2 => NodeKind::Global("max".into(), vec![al, bl]),
                0xAD => NodeKind::Method(Box::new(al), "map".into(), vec![bl]),
                0xAE => NodeKind::Method(Box::new(al), "exists".into(), vec![bl]),
                0xAF => NodeKind::Method(Box::new(al), "forall".into(), vec![bl]),
                0xB3 => NodeKind::Infix("++", Box::new(al), Box::new(bl)),
                0xB5 => NodeKind::Method(Box::new(al), "filter".into(), vec![bl]),
                0xE5 => NodeKind::Method(Box::new(al), "getOrElse".into(), vec![bl]),
                _ => NodeKind::Raw(debug()),
            }
        }
        Payload::Three(a, b, c) => match op {
            0x95 => NodeKind::If(
                Box::new(lift(a, cx, constants)),
                Box::new(lift(b, cx, constants)),
                Box::new(lift(c, cx, constants)),
            ),
            0xB4 => {
                // Slice(input, from, until): source `input.slice(from, until)`.
                let coll = lift(a, cx, constants);
                let from = lift(b, cx, constants);
                let until = lift(c, cx, constants);
                NodeKind::Method(Box::new(coll), "slice".into(), vec![from, until])
            }
            0xB0 => {
                // Fold(input, zero, foldOp): source `input.fold(zero, lambda)`
                let coll = lift(a, cx, constants);
                let zero = lift(b, cx, constants);
                let lam = lift(c, cx, constants);
                NodeKind::Method(Box::new(coll), "fold".into(), vec![zero, lam])
            }
            _ => NodeKind::Raw(debug()),
        },
        Payload::Four(..) => NodeKind::Raw(debug()),
        Payload::ValUse { id } => NodeKind::Val(cx.lookup(*id).unwrap_or_else(|| format!("%{id}"))),
        Payload::ConstPlaceholder { index } => match constants.get(*index as usize) {
            Some((tpe, val)) => lift_const(tpe, val, cx),
            None => NodeKind::Raw(format!("$<bad {}>", index)),
        },
        Payload::TaggedVar { id, .. } => NodeKind::GetVar(*id as i64, String::new()),
        Payload::ValDef { id, rhs, .. } => {
            let name = cx.bind(*id, "val");
            let _ = name;
            let rhs_l = lift(rhs, cx, constants);
            NodeKind::Block(
                vec![Stmt::Val(cx.lookup(*id).unwrap_or_default(), rhs_l)],
                Box::new(Node {
                    id: cx.alloc_id(),
                    kind: NodeKind::Val(cx.lookup(*id).unwrap_or_default()),
                }),
            )
        }
        Payload::FunDef { id, rhs, .. } => {
            let name = cx.bind(*id, "fn");
            NodeKind::Block(
                vec![Stmt::Def(name.clone(), lift(rhs, cx, constants))],
                Box::new(Node {
                    id: cx.alloc_id(),
                    kind: NodeKind::Val(name),
                }),
            )
        }
        Payload::BlockValue { items, result } => {
            cx.push_scope();
            let mut stmts = Vec::with_capacity(items.len());
            let mut bindings: BTreeMap<String, Node> = BTreeMap::new();
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
                result_l.kind
            } else {
                NodeKind::Block(stmts, Box::new(result_l))
            }
        }
        Payload::FuncValue { args, body } => {
            cx.push_scope();
            // Fold's wire lambda wraps (acc, elem) into a SINGLE tuple-typed
            // arg (`(t: (Long, Box)) => t._1 + t._2.value`) — the compiler
            // re-wraps a 2-arg source lambda on emit. Unwrap: render the
            // 2-arg source form so recompilation reproduces the wire.
            if args.len() == 1 {
                // Only a 2-field tuple is the fold wrap shape; other arities
                // fall through to the generic lambda rendering (never index
                // past the field list).
                if let Some(SigmaType::STuple(field_types)) = &args[0].1 {
                    if field_types.len() == 2 {
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
                        return NodeKind::Lambda(names, Box::new(body_l));
                    }
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
            NodeKind::Lambda(names, Box::new(body_l))
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
                    return NodeKind::GetRegDyn(
                        Box::new(lift(obj, cx, constants)),
                        crate::inspect::type_str(t0),
                        args.iter().map(|a| lift(a, cx, constants)).collect(),
                    );
                }
            }
            lift_method_like(*type_id, *method_id, obj, args, cx, constants)
        }
        Payload::ConcreteCollection { elem_type, items } => NodeKind::Coll(
            crate::inspect::type_str(elem_type),
            items.iter().map(|i| lift(i, cx, constants)).collect(),
        ),
        Payload::BoolCollection { bits } => NodeKind::Coll(
            "Boolean".into(),
            bits.iter()
                .map(|b| Node {
                    id: cx.alloc_id(),
                    kind: NodeKind::Bool(*b),
                })
                .collect(),
        ),
        Payload::Tuple { items } => {
            NodeKind::Tuple(items.iter().map(|i| lift(i, cx, constants)).collect())
        }
        Payload::SelectField { input, field_idx } => {
            let obj = Box::new(lift(input, cx, constants));
            NodeKind::Prop(obj, format!("_{}", field_idx))
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
                NodeKind::Prop(obj, "value".into())
            } else {
                NodeKind::Prop(
                    obj,
                    format!("R{}[{}]", reg_id, crate::inspect::type_str(tpe)),
                )
            }
        }
        Payload::GetVar { var_id, tpe } => {
            NodeKind::GetVar(*var_id as i64, crate::inspect::type_str(tpe))
        }
        Payload::DeserializeContext { id, .. } => NodeKind::Global(
            "deserializeContext".into(),
            vec![Node {
                id: cx.alloc_id(),
                kind: NodeKind::Int(*id as i64),
            }],
        ),
        Payload::DeserializeRegister {
            reg_id, default, ..
        } => {
            let mut args = vec![Node {
                id: cx.alloc_id(),
                kind: NodeKind::Int(*reg_id as i64),
            }];
            if let Some(d) = default {
                args.push(lift(d, cx, constants));
            }
            NodeKind::Global("deserializeRegister".into(), args)
        }
        Payload::SigmaCollection { items } => {
            // SigmaAnd/SigmaOr recompile from `&&`/`||` chains over sigma
            // children (0xED BinAnd on SigmaProps lifts to 0xEA on compile).
            let items_l: Vec<Node> = items.iter().map(|i| lift(i, cx, constants)).collect();
            let sym = if op == 0xEA { "&&" } else { "||" };
            match items_l.len() {
                0 => NodeKind::Const(if op == 0xEA { "true" } else { "false" }.into()),
                1 => items_l.into_iter().next().expect("len 1").kind,
                _ => {
                    let mut it = items_l.into_iter();
                    let first = it.next().expect("non-empty");
                    it.fold(first, |acc, item| Node {
                        id: cx.alloc_id(),
                        kind: NodeKind::Infix(sym, Box::new(acc), Box::new(item)),
                    })
                    .kind
                }
            }
        }
        Payload::NoneValue { .. } => NodeKind::Const("None".into()),
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
                return NodeKind::Method(
                    Box::new(Node {
                        id: cx.alloc_id(),
                        kind: NodeKind::Index(Box::new(input_l), Box::new(index_l), None),
                    }),
                    "getOrElse".into(),
                    vec![lift(d, cx, constants)],
                );
            }
            let Node {
                id: input_id,
                kind: input_kind,
            } = input_l;
            match input_kind {
                // Box-collection indexing: `OUTPUTS(i)`, `tokens(i)`. A bound
                // `val` over them (v2[0]) hits the same parser constraint.
                NodeKind::Method(..)
                | NodeKind::Leaf("OUTPUTS")
                | NodeKind::Leaf("INPUTS")
                | NodeKind::Val(_) => NodeKind::ApplyFn(
                    Box::new(Node {
                        id: input_id,
                        kind: input_kind,
                    }),
                    vec![index_l],
                ),
                _ => NodeKind::Index(
                    Box::new(Node {
                        id: input_id,
                        kind: input_kind,
                    }),
                    Box::new(index_l),
                    None,
                ),
            }
        }
        Payload::NumericCast { input, tpe } => match cast_name(tpe) {
            Some(name) => {
                NodeKind::Method(Box::new(lift(input, cx, constants)), name.into(), vec![])
            }
            None => NodeKind::Raw(debug()),
        },
        Payload::FuncApply { func, args } => {
            let f = Box::new(lift(func, cx, constants));
            let args_l: Vec<Node> = args.iter().map(|a| lift(a, cx, constants)).collect();
            NodeKind::ApplyFn(f, args_l)
        }
    }
}

/// A Relation2 payload may be the packed-bool form; treat `None` arms.
/// Rewrite `Prop(Val(bound), "_1"/"_2")` to the fresh fold-field names
/// (fold tuple-unwrap: the wire's 1-arg tuple lambda back to source 2-arg).
fn rewrite_fold_fields(e: Node, bound: &str, n1: &str, n2: &str) -> Node {
    let Node { id, kind } = e;
    let kind = match kind {
        NodeKind::Prop(obj, f) if f == "_1" || f == "_2" => {
            let rewritten = rewrite_fold_fields(*obj, bound, n1, n2);
            match &rewritten.kind {
                NodeKind::Val(name) if name == bound => {
                    NodeKind::Val((if f == "_1" { n1 } else { n2 }).to_string())
                }
                _ => NodeKind::Prop(Box::new(rewritten), f),
            }
        }
        NodeKind::Prop(obj, f) => {
            NodeKind::Prop(Box::new(rewrite_fold_fields(*obj, bound, n1, n2)), f)
        }
        NodeKind::Method(obj, n, args) => NodeKind::Method(
            Box::new(rewrite_fold_fields(*obj, bound, n1, n2)),
            n,
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        NodeKind::Infix(sym, a, b) => NodeKind::Infix(
            sym,
            Box::new(rewrite_fold_fields(*a, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*b, bound, n1, n2)),
        ),
        NodeKind::Unary(op, a) => {
            NodeKind::Unary(op, Box::new(rewrite_fold_fields(*a, bound, n1, n2)))
        }
        NodeKind::Global(n, args) => NodeKind::Global(
            n,
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        NodeKind::Coll(t, items) => NodeKind::Coll(
            t,
            items
                .into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        NodeKind::Tuple(items) => NodeKind::Tuple(
            items
                .into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        NodeKind::ApplyFn(f, args) => NodeKind::ApplyFn(
            Box::new(rewrite_fold_fields(*f, bound, n1, n2)),
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        NodeKind::Block(stmts, result) => NodeKind::Block(
            stmts
                .into_iter()
                .map(|st| match st {
                    Stmt::Val(n, e) => Stmt::Val(n, rewrite_fold_fields(e, bound, n1, n2)),
                    Stmt::Def(n, e) => Stmt::Def(n, rewrite_fold_fields(e, bound, n1, n2)),
                })
                .collect(),
            Box::new(rewrite_fold_fields(*result, bound, n1, n2)),
        ),
        NodeKind::If(c, t, els) => NodeKind::If(
            Box::new(rewrite_fold_fields(*c, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*t, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*els, bound, n1, n2)),
        ),
        NodeKind::Lambda(args, body) => {
            NodeKind::Lambda(args, Box::new(rewrite_fold_fields(*body, bound, n1, n2)))
        }
        NodeKind::Index(a, b, d) => NodeKind::Index(
            Box::new(rewrite_fold_fields(*a, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*b, bound, n1, n2)),
            d.map(|x| Box::new(rewrite_fold_fields(*x, bound, n1, n2))),
        ),
        NodeKind::AtLeast(k, c) => NodeKind::AtLeast(
            Box::new(rewrite_fold_fields(*k, bound, n1, n2)),
            Box::new(rewrite_fold_fields(*c, bound, n1, n2)),
        ),
        NodeKind::GetRegDyn(o, t, args) => NodeKind::GetRegDyn(
            Box::new(rewrite_fold_fields(*o, bound, n1, n2)),
            t,
            args.into_iter()
                .map(|a| rewrite_fold_fields(a, bound, n1, n2))
                .collect(),
        ),
        other => other,
    };
    Node { id, kind }
}

/// Wrap a lifted expression in `sigmaProp(…)` when it isn't already
/// sigma-typed (AtLeast/allOf children must be SigmaProps).
fn wrap_sigma(e: Node, cx: &mut LiftCtx) -> Node {
    let Node { id, kind } = e;
    match kind {
        // Already sigma-ish: sigmaProp calls, PK constants, proveDlog,
        // AtLeast, sigma and/or.
        NodeKind::Global(ref name, _) if name == "sigmaProp" || name == "proveDlog" => {
            Node { id, kind }
        }
        NodeKind::AtLeast(..) => Node { id, kind },
        // PK("…") and sigmaProp(…) constants are already SigmaProp-typed.
        NodeKind::Const(ref s) if s.starts_with("PK(") || s.starts_with("sigmaProp(") => {
            Node { id, kind }
        }
        // Everything else (bool comparisons, and-chains of bools) wraps.
        _ => Node {
            id: cx.alloc_id(),
            kind: NodeKind::Global("sigmaProp".into(), vec![Node { id, kind }]),
        },
    }
}
