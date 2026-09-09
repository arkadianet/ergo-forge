//! The printer: lifted AST → source-like ErgoScript text.
//!
//! Parenthesization is precedence-driven; `parent` is the enclosing
//! operator's precedence, `None` at top level.

use std::fmt::Write as _;

use super::ast::{Node, NodeKind, Stmt};

// ── printer ──────────────────────────────────────────────────────────────────

/// Precedence for an infix operator symbol. Higher binds tighter. Mirrors
/// the pinned compiler's `parse::operators::precedence_of`, including the
/// distinct precedence of `>` versus `<`, and XOR below AND.
pub(crate) fn prec_of(sym: &str) -> u8 {
    match sym {
        "xorBytes" => 0,
        "||" => 1,
        "^" => 2,
        "&&" => 3,
        "==" | "!=" => 4,
        "<" | "<=" => 5,
        ">" | ">=" => 6,
        "-" | "+" | "++" => 7,
        "*" | "/" | "%" => 8,
        other => unreachable!("unknown infix operator {other:?}"),
    }
}

/// Operator precedence context: `None` = top level (no parens needed).
pub(crate) fn print_node(n: &Node, parent: Option<u8>, out: &mut String) {
    let parens = |out: &mut String, f: &dyn Fn(&mut String)| {
        out.push('(');
        f(out);
        out.push(')');
    };
    match &n.kind {
        NodeKind::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        NodeKind::Int(i) => {
            let _ = write!(out, "{i}");
        }
        NodeKind::Num(s) => out.push_str(s),
        NodeKind::Const(s) => out.push_str(s),
        NodeKind::Val(name) => out.push_str(name),
        NodeKind::GetVar(id, tpe) => {
            // Source form is `getVar[T](id)` — the compiler's predef parses
            // the type parameter in brackets and the id in call parens.
            if tpe.is_empty() {
                let _ = write!(out, "getVar({id})");
            } else {
                let _ = write!(out, "getVar[{tpe}]({id})");
            }
        }
        NodeKind::Leaf(s) => out.push_str(s),
        NodeKind::Unary(op, inner) => {
            let this = 9u8;
            let needs = parent.is_some_and(|p| p > this);
            // Binder quirk (upstream): `!v.R5[T].isDefined` types the `!`
            // operand as the UNAPPLIED generic accessor (SFunc → Option[T]).
            // Parenthesizing the operand keeps it intact, so a logical-not
            // always renders as `(!(operand))`.
            let emit_inner = |o: &mut String| {
                if *op == "!" {
                    o.push('(');
                    print_node(inner, Some(this), o);
                    o.push(')');
                } else if *op == "-" {
                    // Negation over a numeric literal must NOT render as a
                    // negative literal: the binder folds `-n` into
                    // `Const(-n)`, collapsing the Negation op — and at the
                    // Int bound the surrounding `Minus(Const(-2147483647), 2)`
                    // then overflows the constant folder. `(0 + n)` after the
                    // unary minus re-parses to Negation(Const(n)) —
                    // byte-identical to the wire (the `0 + n` identity fold is
                    // what the reference compiler does with the original
                    // source). Verified against the JVM TyperOracle
                    // (sigma-state 6.0.2).
                    match &inner.kind {
                        NodeKind::Int(n) => {
                            o.push_str("(0 + ");
                            let _ = write!(o, "{n}");
                            o.push(')');
                        }
                        NodeKind::Num(t) if t.ends_with('L') => {
                            o.push_str("(0 + ");
                            o.push_str(t);
                            o.push(')');
                        }
                        _ => print_node(inner, Some(this), o),
                    }
                } else {
                    print_node(inner, Some(this), o);
                }
            };
            if needs {
                parens(out, &|o| {
                    o.push_str(op);
                    emit_inner(o);
                });
            } else {
                out.push_str(op);
                emit_inner(out);
            }
        }
        NodeKind::Infix(sym, lhs, rhs) => {
            if matches!(*sym, "+" | "*") && (needs_binding(lhs) || needs_binding(rhs)) {
                // The compiler re-types +/* operands; some already lowered
                // forms cannot be re-typed. Source temporaries avoid that
                // without changing the lifted AST consumed by other tools.
                let left = temporary_name(n, lhs.id, "dcLeft");
                let right = temporary_name(n, rhs.id, "dcRight");
                let _ = write!(out, "{{ val {left} = ");
                print_node(lhs, None, out);
                let _ = write!(out, "; val {right} = ");
                print_node(rhs, None, out);
                let _ = write!(out, "; {left} {sym} {right} }}");
                return;
            }
            let this = prec_of(sym);
            let needs = parent.is_some_and(|p| p > this);
            let emit = |o: &mut String| {
                print_node(lhs, Some(this), o);
                o.push(' ');
                o.push_str(sym);
                o.push(' ');
                // Right operand of a left-associative operator needs parens
                // at EQUAL precedence, or the parser re-associates left:
                // Minus(a, Minus(b, c)) must print `a - (b - c)`, not
                // `a - b - c`.
                print_node(rhs, Some(this + 1), o);
            };
            if needs {
                parens(out, &emit);
            } else {
                emit(out);
            }
        }
        NodeKind::Method(obj, name, args) => {
            // Receiver binds like a postfix expression (tightest).
            print_node(obj, Some(10), out);
            out.push('.');
            out.push_str(name);
            if !args.is_empty() {
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    print_node(a, None, out);
                }
                out.push(')');
            }
        }
        NodeKind::ApplyFn(f, args) => {
            print_node(f, Some(10), out);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_node(a, None, out);
            }
            out.push(')');
        }
        NodeKind::Prop(obj, name) => {
            print_node(obj, Some(10), out);
            out.push('.');
            out.push_str(name);
        }
        NodeKind::GetRegDyn(obj, tpe, args) => {
            print_node(obj, Some(10), out);
            out.push_str(&format!(".getReg[{tpe}]("));
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_node(a, None, out);
            }
            out.push(')');
        }
        NodeKind::Coll(elem, items) => {
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
                    print_node(it, None, out);
                }
                out.push(')');
            }
        }
        NodeKind::Tuple(items) => {
            out.push('(');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_node(it, None, out);
            }
            out.push(')');
        }
        NodeKind::Index(input, index, default) => {
            print_node(input, Some(10), out);
            out.push('[');
            print_node(index, None, out);
            out.push(']');
            if let Some(d) = default {
                out.push_str(".getOrElse(");
                print_node(d, None, out);
                out.push(')');
            }
        }
        NodeKind::If(cond, then, els) => {
            let emit = |o: &mut String| {
                o.push_str("if (");
                print_node(cond, None, o);
                o.push_str(") ");
                print_node(then, None, o);
                o.push_str(" else ");
                print_node(els, None, o);
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
        NodeKind::Lambda(args, body) => {
            let emit = |o: &mut String| {
                o.push_str("{ (");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        o.push_str(", ");
                    }
                    o.push_str(a);
                }
                o.push_str(") => ");
                print_node(body, None, o);
                o.push('}');
            };
            if parent.is_some_and(|p| p > 0) {
                parens(out, &emit);
            } else {
                emit(out);
            }
        }
        NodeKind::Global(name, args) => {
            out.push_str(name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_node(a, None, out);
            }
            out.push(')');
        }
        NodeKind::AtLeast(k, items) => {
            out.push_str("atLeast(");
            print_node(k, None, out);
            out.push_str(", ");
            print_node(items, None, out);
            out.push(')');
        }
        NodeKind::Raw(s) => out.push_str(s),
        NodeKind::Block(stmts, result) => {
            out.push_str("{ ");
            for s in stmts {
                print_stmt(s, out);
                out.push_str("; ");
            }
            print_node(result, None, out);
            out.push_str(" }");
        }
    }
}

fn print_stmt(s: &Stmt, out: &mut String) {
    match s {
        Stmt::Val(name, e) => {
            let _ = write!(out, "val {name} = ");
            print_node(e, None, out);
        }
        Stmt::Def(name, e) => {
            let _ = write!(out, "def {name} = ");
            print_node(e, None, out);
        }
    }
}

fn needs_binding(n: &Node) -> bool {
    match &n.kind {
        NodeKind::If(..) | NodeKind::ApplyFn(..) | NodeKind::Block(..) => true,
        NodeKind::Method(_, name, _) if name == "getOrElse" || name == "fold" => true,
        NodeKind::Infix(_, a, b) => needs_binding(a) || needs_binding(b),
        _ => false,
    }
}

fn temporary_name(expression: &Node, id: u64, prefix: &str) -> String {
    let mut name = format!("{prefix}{id}");
    // Lift-generated names never use these prefixes; still avoid capture when print()
    // is called with a caller-built AST containing such a reference.
    while references_name(expression, &name) {
        name.push('_');
    }
    name
}

fn references_name(n: &Node, name: &str) -> bool {
    let has = |n: &Node| references_name(n, name);
    match &n.kind {
        NodeKind::Val(v) => v == name,
        NodeKind::Unary(_, a) | NodeKind::Prop(a, _) | NodeKind::Lambda(_, a) => has(a),
        NodeKind::Infix(_, a, b) | NodeKind::AtLeast(a, b) => has(a) || has(b),
        NodeKind::Method(o, _, args)
        | NodeKind::ApplyFn(o, args)
        | NodeKind::GetRegDyn(o, _, args) => has(o) || args.iter().any(has),
        NodeKind::Coll(_, items) | NodeKind::Tuple(items) | NodeKind::Global(_, items) => {
            items.iter().any(has)
        }
        NodeKind::Index(a, b, d) => has(a) || has(b) || d.as_deref().is_some_and(has),
        NodeKind::If(c, t, e) => has(c) || has(t) || has(e),
        NodeKind::Block(stmts, result) => {
            has(result)
                || stmts.iter().any(|s| match s {
                    Stmt::Val(_, n) | Stmt::Def(_, n) => has(n),
                })
        }
        _ => false,
    }
}
