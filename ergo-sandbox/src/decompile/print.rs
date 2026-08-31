//! The printer: lifted AST → source-like ErgoScript text.
//!
//! Parenthesization is precedence-driven; `parent` is the enclosing
//! operator's precedence, `None` at top level.

use std::fmt::Write as _;

use super::ast::{Stmt, L};

// ── printer ──────────────────────────────────────────────────────────────────

/// Operator precedence context: `None` = top level (no parens needed).
pub(crate) fn print_l(e: &L, parent: Option<u8>, out: &mut String) {
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
            // Binder quirk (upstream): `!v.R5[T].isDefined` types the `!`
            // operand as the UNAPPLIED generic accessor (SFunc → Option[T]).
            // Parenthesizing the operand keeps it intact, so a logical-not
            // always renders as `(!(operand))`.
            let emit_inner = |o: &mut String| {
                if *op == "!" {
                    o.push('(');
                    print_l(inner, Some(this), o);
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
                    match inner.as_ref() {
                        L::Int(n) => {
                            o.push_str("(0 + ");
                            let _ = write!(o, "{n}");
                            o.push(')');
                        }
                        L::Num(t) if t.ends_with('L') => {
                            o.push_str("(0 + ");
                            o.push_str(t);
                            o.push(')');
                        }
                        _ => print_l(inner, Some(this), o),
                    }
                } else {
                    print_l(inner, Some(this), o);
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
        L::Infix(sym, prec, lhs, rhs) => {
            let this = *prec;
            let needs = parent.is_some_and(|p| p > this);
            let emit = |o: &mut String| {
                print_l(lhs, Some(this), o);
                o.push(' ');
                o.push_str(sym);
                o.push(' ');
                // Right operand of a left-associative operator needs parens
                // at EQUAL precedence, or the parser re-associates left:
                // Minus(a, Minus(b, c)) must print `a - (b - c)`, not
                // `a - b - c`.
                print_l(rhs, Some(this + 1), o);
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
