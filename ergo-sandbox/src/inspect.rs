//! Structural inspection of ErgoTrees: wire bytes → readable IR view.
//!
//! This is the P0 spike (`ergo-compiler/examples/decompile.rs`, now
//! superseded) productized as a library module. It prints the honest
//! structural view — opcode tree with constants inlined — not (yet) the
//! decompiler's lifted source. Renderings are for human shells (CLI/UI);
//! no consumer should parse them.
//!
//! Opcode names are cross-checked against `ergo-ser`'s
//! `opcode_pattern` table (the parse-accept set) and the evaluator's
//! `dispatch/eval.rs` arms (the executable set).

use ergo_primitives::reader::VlqReader;
use ergo_primitives::writer::VlqWriter;
use ergo_ser::ergo_tree::{read_ergo_tree, write_ergo_tree, ErgoTree};
use ergo_ser::opcode::{Expr, Payload};
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};

use crate::SandboxError;

/// Mnemonic for an opcode byte, or `None` when outside the known set.
#[must_use]
pub fn opcode_name(op: u8) -> Option<&'static str> {
    Some(match op {
        0x71 => "TaggedVar",
        0x72 => "ValUse",
        0x73 => "ConstantPlaceholder",
        0x74 => "SubstConstants",
        0x7A => "LongToByteArray",
        0x7B => "ByteArrayToBigInt",
        0x7C => "ByteArrayToLong",
        0x7D => "Downcast",
        0x7E => "Upcast",
        0x7F => "True",
        0x80 => "False",
        0x82 => "GroupGenerator",
        0x83 => "ConcreteCollection",
        0x85 => "BoolCollection",
        0x86 => "Tuple",
        0x8C => "SelectField",
        0x8F => "LT",
        0x90 => "LE",
        0x91 => "GT",
        0x92 => "GE",
        0x93 => "EQ",
        0x94 => "NEQ",
        0x95 => "If",
        0x96 => "AND",
        0x97 => "OR",
        0x98 => "AtLeast",
        0x99 => "Minus",
        0x9A => "Plus",
        0x9B => "Xor",
        0x9C => "Multiply",
        0x9D => "Divide",
        0x9E => "Modulo",
        0x9F => "Exponentiate",
        0xA0 => "MultiplyGroup",
        0xA1 => "Min",
        0xA2 => "Max",
        0xA3 => "HEIGHT",
        0xA4 => "INPUTS",
        0xA5 => "OUTPUTS",
        0xA6 => "LastBlockUtxoRootHash",
        0xA7 => "SELF",
        0xAC => "MinerPubKey",
        0xAD => "Map",
        0xAE => "Exists",
        0xAF => "ForAll",
        0xB0 => "Fold",
        0xB1 => "SizeOf",
        0xB2 => "ByIndex",
        0xB3 => "Append",
        0xB4 => "Slice",
        0xB5 => "Filter",
        0xB6 => "AvlTree",
        0xB7 => "TreeLookup",
        0xC1 => "ExtractAmount",
        0xC2 => "ExtractScriptBytes",
        0xC3 => "ExtractBytes",
        0xC4 => "ExtractBytesWithNoRef",
        0xC5 => "ExtractId",
        0xC6 => "ExtractRegisterAs",
        0xC7 => "ExtractCreationInfo",
        0xCB => "CalcBlake2b256",
        0xCC => "CalcSha256",
        0xCD => "ProveDlog",
        0xCE => "ProveDHTuple",
        0xCF => "SigmaPropIsProven",
        0xD0 => "SigmaPropBytes",
        0xD1 => "BoolToSigmaProp",
        0xD4 => "DeserializeContext",
        0xD5 => "DeserializeRegister",
        0xD6 => "ValDef",
        0xD7 => "FunDef",
        0xD8 => "Block",
        0xD9 => "FuncValue",
        0xDA => "FuncApply",
        0xDB => "PropertyCall",
        0xDC => "MethodCall",
        0xDD => "Global",
        0xE3 => "GetVar",
        0xE4 => "OptionGet",
        0xE5 => "OptionGetOrElse",
        0xE6 => "OptionIsDefined",
        0xE7 => "ModQ",
        0xE8 => "PlusModQ",
        0xE9 => "MinusModQ",
        0xEA => "SigmaAnd",
        0xEB => "SigmaOr",
        0xEC => "BinOr",
        0xED => "BinAnd",
        0xEE => "DecodePoint",
        0xEF => "LogicalNot",
        0xF0 => "Negation",
        0xF1 => "BitInversion",
        0xF2 => "BitOr",
        0xF3 => "BitAnd",
        0xF4 => "BinXor",
        0xF5 => "BitXor",
        0xF6 => "BitShiftRight",
        0xF7 => "BitShiftLeft",
        0xF8 => "BitShiftRightZeroed",
        0xFE => "CONTEXT",
        0xFF => "XorOf",
        _ => return None,
    })
}

// ── public renderers ─────────────────────────────────────────────────────────

/// Render a reduced `SigmaBoolean` in the same structural notation as the
/// body printer (used by [`crate::eval`] for `reducedTo`).
#[must_use]
pub fn sigma_boolean_pretty(sb: &SigmaBoolean) -> String {
    match sb {
        SigmaBoolean::TrivialProp(b) => format!("{b}"),
        SigmaBoolean::ProveDlog(ge) => format!("ProveDlog({})", ge_short(ge.as_bytes())),
        SigmaBoolean::ProveDHTuple { g, h, u, v } => format!(
            "DHT({}, {}, {}, {})",
            ge_short(g.as_bytes()),
            ge_short(h.as_bytes()),
            ge_short(u.as_bytes()),
            ge_short(v.as_bytes())
        ),
        SigmaBoolean::Cand(xs) => join_sigma("AND", xs),
        SigmaBoolean::Cor(xs) => join_sigma("OR", xs),
        other => format!("{other:?}"),
    }
}

/// Parse ErgoTree wire bytes (public shim for the decompile module).
pub(crate) fn parse_tree(bytes: &[u8]) -> Result<ErgoTree, crate::SandboxError> {
    let mut r = VlqReader::new(bytes);
    read_ergo_tree(&mut r).map_err(|e| crate::SandboxError::Tree(e.to_string()))
}

/// Render an ErgoTree's wire bytes as a human-readable report: header info,
/// the constants table, the structural body, and a byte-identity note.
pub fn tree_report(bytes: &[u8]) -> Result<String, SandboxError> {
    let mut r = VlqReader::new(bytes);
    let tree = read_ergo_tree(&mut r).map_err(|e| SandboxError::Tree(e.to_string()))?;
    let mut out = String::new();
    out.push_str(&format!(
        "version={} segregated={} constants={}\n",
        tree.version,
        tree.constant_segregation,
        tree.constants.len()
    ));
    for (i, (tpe, val)) in tree.constants.iter().enumerate() {
        out.push_str(&format!(
            "  ${i}: {} = {}\n",
            type_str(tpe),
            const_str(tpe, val)
        ));
    }
    out.push_str(&format!("body: {}\n", tree_structure(&tree)));
    let mut w = VlqWriter::new();
    match write_ergo_tree(&mut w, &tree) {
        Ok(()) if w.result() == bytes => out.push_str("re-serializes byte-identical\n"),
        Ok(()) => out.push_str("re-serializes DIFFERENTLY (non-canonical input)\n"),
        Err(e) => out.push_str(&format!("re-serialization failed: {e}\n")),
    }
    Ok(out)
}

/// Render just the structural body expression (no header/constants).
#[must_use]
pub fn tree_structure(tree: &ErgoTree) -> String {
    let mut out = String::new();
    fmt_expr(&tree.body, &mut out);
    out
}

/// Encode a compressed SEC1 point as a P2PK (base58) address — the form
/// ErgoScript's `PK("…")` predef accepts. `testnet` selects the header the
/// compile corpus was captured with (header 0x11 = testnet 0x10 | P2PK 0x01);
/// mainnet uses 0x01.
#[must_use]
pub fn group_element_base58_net(bytes: &[u8; 33], testnet: bool) -> String {
    // Address = base58(header ‖ pubkey ‖ checksum[0..4])
    let mut body = Vec::with_capacity(1 + 33 + 4);
    body.push(if testnet { 0x11 } else { 0x01 });
    body.extend_from_slice(bytes);
    let hash = ergo_primitives::digest::blake2b256(&body);
    body.extend_from_slice(&hash.as_bytes()[..4]);
    bs58::encode(body).into_string()
}

/// Testnet shorthand (the corpus round-trip bar's network).
#[must_use]
pub fn group_element_base58(bytes: &[u8; 33]) -> String {
    group_element_base58_net(bytes, true)
}

/// Fallback debug form for a constant the lifted printer can't render.
#[must_use]
pub fn value_debug(tpe: &SigmaType, val: &SigmaValue) -> String {
    format!("<const {} = {}>", type_str(tpe), value_str(val))
}

/// Raw structural print of a single expression (public for decompile's
/// fallback rendering).
pub(crate) fn fmt_expr_raw(e: &Expr, out: &mut String) {
    fmt_expr(e, out);
}

// ── structural printer ───────────────────────────────────────────────────────

fn ge_short(bytes: &[u8; 33]) -> String {
    format!("{}…", hex::encode(&bytes[..8]))
}

fn join_sigma(op: &str, xs: &[SigmaBoolean]) -> String {
    let inner: Vec<String> = xs.iter().map(sigma_boolean_pretty).collect();
    format!("{op}({inner})", inner = inner.join(", "))
}

pub(crate) fn type_str(t: &SigmaType) -> String {
    match t {
        SigmaType::SBoolean => "Boolean".into(),
        SigmaType::SByte => "Byte".into(),
        SigmaType::SShort => "Short".into(),
        SigmaType::SInt => "Int".into(),
        SigmaType::SLong => "Long".into(),
        SigmaType::SBigInt => "BigInt".into(),
        SigmaType::SGroupElement => "GroupElement".into(),
        SigmaType::SSigmaProp => "SigmaProp".into(),
        SigmaType::SBox => "Box".into(),
        SigmaType::SAvlTree => "AvlTree".into(),
        SigmaType::SContext => "Context".into(),
        SigmaType::SHeader => "Header".into(),
        SigmaType::SPreHeader => "PreHeader".into(),
        SigmaType::SUnsignedBigInt => "UnsignedBigInt".into(),
        SigmaType::SOption(inner) => format!("Option[{}]", type_str(inner)),
        SigmaType::SColl(inner) => format!("Coll[{}]", type_str(inner)),
        SigmaType::STuple(items) => format!(
            "({})",
            items.iter().map(type_str).collect::<Vec<_>>().join(", ")
        ),
        other => format!("{other:?}"),
    }
}

fn value_str(v: &SigmaValue) -> String {
    match v {
        SigmaValue::Boolean(b) => format!("{b}"),
        SigmaValue::Byte(x) => format!("{x}y"),
        SigmaValue::Short(x) => format!("{x}s"),
        SigmaValue::Int(x) => format!("{x}"),
        SigmaValue::Long(x) => format!("{x}L"),
        SigmaValue::BigInt(x) => format!("{x}N"),
        SigmaValue::Str(s) => format!("\"{s}\""),
        SigmaValue::GroupElement(ge) => format!("GE({})", ge_short(ge.as_bytes())),
        SigmaValue::SigmaProp(sb) => sigma_boolean_pretty(sb),
        SigmaValue::Coll(CollValue::BoolBits(bits)) => format!(
            "Coll[Boolean]({})",
            bits.iter()
                .map(|b| if *b { '1' } else { '0' })
                .collect::<String>()
        ),
        SigmaValue::Coll(CollValue::Bytes(bs)) if bs.len() <= 32 => {
            format!("Coll[Byte]({})", hex::encode(bs))
        }
        other => format!("{other:?}"),
    }
}

fn const_str(tpe: &SigmaType, val: &SigmaValue) -> String {
    match (tpe, val) {
        (SigmaType::SSigmaProp, SigmaValue::SigmaProp(sb)) => sigma_boolean_pretty(sb),
        (SigmaType::SGroupElement, SigmaValue::GroupElement(ge)) => {
            format!("GE({})", ge_short(ge.as_bytes()))
        }
        (SigmaType::SColl(inner), SigmaValue::Coll(CollValue::Bytes(bs))) => {
            let _ = inner;
            format!("Coll[Byte]({})", hex::encode(bs))
        }
        _ => value_str(val),
    }
}

/// Structural printer: inline constants render as literals, segregated
/// constants as `$index` references.
fn fmt_expr(e: &Expr, out: &mut String) {
    match e {
        Expr::Const { tpe, val } => {
            out.push_str(&const_str(tpe, val));
        }
        Expr::Unparsed(bytes) => {
            out.push_str(&format!("<UNPARSED {} bytes>", bytes.len()));
        }
        Expr::Op(node) => {
            let name = opcode_name(node.opcode).unwrap_or("OP_?");
            match &node.payload {
                Payload::Zero => out.push_str(name),
                Payload::One(a) => {
                    out.push_str(&format!("({name} "));
                    fmt_expr(a, out);
                    out.push(')');
                }
                Payload::Two(a, b) => {
                    out.push_str(&format!("({name} "));
                    fmt_expr(a, out);
                    out.push(' ');
                    fmt_expr(b, out);
                    out.push(')');
                }
                Payload::Three(a, b, c) => {
                    out.push_str(&format!("({name} "));
                    for (i, x) in [a, b, c].iter().enumerate() {
                        if i > 0 {
                            out.push(' ');
                        }
                        fmt_expr(x, out);
                    }
                    out.push(')');
                }
                Payload::Four(a, b, c, d) => {
                    out.push_str(&format!("({name} "));
                    for (i, x) in [a, b, c, d].iter().enumerate() {
                        if i > 0 {
                            out.push(' ');
                        }
                        fmt_expr(x, out);
                    }
                    out.push(')');
                }
                Payload::ValUse { id } => out.push_str(&format!("%{id}")),
                Payload::ConstPlaceholder { index } => out.push_str(&format!("${index}")),
                Payload::TaggedVar { id, tpe: _ } => out.push_str(&format!("(var {id})")),
                Payload::ValDef { id, rhs, .. } => {
                    out.push_str(&format!("(val %{id} = "));
                    fmt_expr(rhs, out);
                    out.push(')');
                }
                Payload::FunDef { id, rhs, .. } => {
                    out.push_str(&format!("(def %{id} = "));
                    fmt_expr(rhs, out);
                    out.push(')');
                }
                Payload::BlockValue { items, result } => {
                    out.push_str("(block ");
                    for item in items {
                        fmt_expr(item, out);
                        out.push_str("; ");
                    }
                    fmt_expr(result, out);
                    out.push(')');
                }
                Payload::FuncValue { args, body } => {
                    out.push_str("(fn (");
                    for (i, (id, _t)) in args.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&format!("%{id}"));
                    }
                    out.push_str(") ");
                    fmt_expr(body, out);
                    out.push(')');
                }
                Payload::MethodCall {
                    type_id,
                    method_id,
                    obj,
                    args,
                    ..
                } => {
                    out.push_str(&format!("(method 0x{type_id:x}.0x{method_id:x} "));
                    fmt_expr(obj, out);
                    for a in args {
                        out.push(' ');
                        fmt_expr(a, out);
                    }
                    out.push(')');
                }
                Payload::ConcreteCollection { elem_type, items } => {
                    out.push_str(&format!("[{}; ", type_str(elem_type)));
                    for (i, it) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        fmt_expr(it, out);
                    }
                    out.push(']');
                }
                Payload::BoolCollection { bits } => {
                    out.push_str(&format!(
                        "[{}]",
                        bits.iter()
                            .map(|b| if *b { '1' } else { '0' })
                            .collect::<String>()
                    ));
                }
                Payload::Tuple { items } => {
                    out.push('(');
                    for (i, it) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        fmt_expr(it, out);
                    }
                    out.push(')');
                }
                Payload::SelectField { input, field_idx } => {
                    fmt_expr(input, out);
                    out.push_str(&format!("._{field_idx}"));
                }
                Payload::ExtractRegisterAs { input, reg_id, tpe } => {
                    fmt_expr(input, out);
                    out.push_str(&format!(".R{}:{}", reg_id, type_str(tpe)));
                }
                Payload::GetVar { var_id, tpe } => {
                    out.push_str(&format!("(getVar {var_id}:{})", type_str(tpe)));
                }
                Payload::DeserializeContext { id, tpe } => {
                    out.push_str(&format!("(deserializeContext var{id}:{})", type_str(tpe)));
                }
                Payload::DeserializeRegister {
                    reg_id,
                    tpe,
                    default,
                } => {
                    out.push_str(&format!("(deserializeRegister R{reg_id}:{}", type_str(tpe)));
                    if let Some(d) = default {
                        out.push(' ');
                        fmt_expr(d, out);
                    }
                    out.push(')');
                }
                Payload::SigmaCollection { items } => {
                    out.push('(');
                    out.push_str(name);
                    out.push(' ');
                    for (i, it) in items.iter().enumerate() {
                        if i > 0 {
                            out.push(' ');
                        }
                        fmt_expr(it, out);
                    }
                    out.push(')');
                }
                Payload::NoneValue { tpe } => {
                    out.push_str(&format!("None[{}]", type_str(tpe)));
                }
                Payload::ByIndex {
                    input,
                    index,
                    default,
                } => {
                    fmt_expr(input, out);
                    out.push('[');
                    fmt_expr(index, out);
                    out.push(']');
                    if let Some(d) = default {
                        out.push_str(" ?? ");
                        fmt_expr(d, out);
                    }
                }
                Payload::NumericCast { input, tpe } => {
                    fmt_expr(input, out);
                    out.push_str(&format!(":{}", type_str(tpe)));
                }
                Payload::FuncApply { func, args } => {
                    fmt_expr(func, out);
                    out.push('(');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        fmt_expr(a, out);
                    }
                    out.push(')');
                }
            }
        }
    }
}
