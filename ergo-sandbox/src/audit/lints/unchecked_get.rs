//! `Option.get` with no `isDefined` guard.
//!
//! If the option is empty at validation the script throws and the spend
//! fails; for a contract whose only spending path runs through that read,
//! the box becomes permanently unspendable.
//!
//! Severity follows **who controls the receiver**, found by walking the
//! receiver chain to its root:
//!
//! - a context variable (`getVar[T](n).get`) is supplied by the spender with
//!   the proof — a missing one fails that spend, it cannot lock the box: Low;
//! - a lambda parameter (an element of the collection being iterated —
//!   inputs, outputs, data inputs, or something derived from them) is an
//!   element the spender chose: Medium, fragile rather than locking;
//! - anything else (`SELF`, `OUTPUTS(0)`, a `val`) can be an unmovable fact
//!   about the box: High.
//!
//! Guards held in a `val` are followed: a block statement `val ok = …` whose
//! expression proves receivers defined makes `ok` prove the same receivers
//! wherever it appears in a guard position after its definition. (Single-use
//! vals are inlined by the compiler; this is for the multi-use ones that
//! survive to the tree.)
//!
//! Box aliases such as `val successor = OUTPUTS(0)` are resolved before
//! comparing guards with reads. Structural receiver keys capture bindings at
//! declaration time; block and lambda shadowing cannot reuse an outer proof.
//! Unsupported expressions remain unproven and are reported conservatively.
//!
//! `||` and negation are handled by the dual rule: the right operand of
//! `a || b` runs only when `a` is false, and the else branch of `if (c)`
//! runs only when `c` is false — so what `a`'s (or `c`'s) *falsity* proves
//! defined guards there. Falsity of `!(e)` proves what `e` proves; falsity
//! of `a || b` proves what both sides' falsity proves.
//!
//! Known gap (deliberate — see the P3a spec): cross-branch reasoning (an
//! earlier `if` that already returned) is not recognised.

use std::collections::HashMap;

use crate::audit::{children, snippet, Finding, Severity};
use crate::decompile::Stmt;
use crate::{Node, NodeKind};

/// Boolean `val`s in scope that prove receivers defined, by name.
type GuardVals = HashMap<String, Vec<Receiver>>;

/// Structural keys keep node kinds, arguments and register types distinct.
/// Binding identities are allocated per lexical declaration, never per spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Receiver {
    Atom(&'static str, String),
    Binding(usize),
    GetVar(i64, String),
    Expr(&'static str, String, Vec<Receiver>),
}

#[derive(Default)]
struct Scope {
    guards: GuardVals,
    bindings: HashMap<String, Receiver>,
    next_binding: usize,
}

impl Scope {
    fn bind(&mut self, name: &str, alias: Option<Receiver>, guards: Vec<Receiver>) {
        let identity = Receiver::Binding(self.next_binding);
        self.next_binding += 1;
        self.bindings.insert(name.into(), alias.unwrap_or(identity));
        // An empty guard set must also hide an outer Boolean guard.
        self.guards.insert(name.into(), guards);
    }
}

/// Canonicalise supported receiver structure in the current lexical scope.
/// Unknown/lambda/block expressions deliberately have no comparable key.
fn receiver(n: &Node, scope: &Scope) -> Option<Receiver> {
    let atom = |tag, value| Some(Receiver::Atom(tag, value));
    let expr = |tag, name, nodes: Vec<&Node>| {
        Some(Receiver::Expr(
            tag,
            name,
            nodes
                .into_iter()
                .map(|n| receiver(n, scope))
                .collect::<Option<_>>()?,
        ))
    };
    match &n.kind {
        NodeKind::Val(name) => scope
            .bindings
            .get(name)
            .cloned()
            .or_else(|| atom("free", name.clone())),
        NodeKind::Leaf(name) => atom("leaf", (*name).into()),
        NodeKind::Bool(value) => atom("bool", value.to_string()),
        NodeKind::Int(value) => atom("int", value.to_string()),
        NodeKind::Num(value) => atom("num", value.clone()),
        NodeKind::Const(value) => atom("const", value.clone()),
        NodeKind::GetVar(id, tpe) => Some(Receiver::GetVar(*id, tpe.clone())),
        NodeKind::Prop(obj, name) => expr("prop", name.clone(), vec![obj]),
        NodeKind::Method(obj, name, args) => expr(
            "method",
            name.clone(),
            std::iter::once(obj.as_ref()).chain(args).collect(),
        ),
        NodeKind::GetRegDyn(obj, tpe, args) => expr(
            "register",
            tpe.clone(),
            std::iter::once(obj.as_ref()).chain(args).collect(),
        ),
        NodeKind::ApplyFn(obj, args) => expr(
            "apply",
            String::new(),
            std::iter::once(obj.as_ref()).chain(args).collect(),
        ),
        NodeKind::Index(obj, index, default) => expr(
            "index",
            String::new(),
            [obj.as_ref(), index.as_ref()]
                .into_iter()
                .chain(default.as_deref())
                .collect(),
        ),
        NodeKind::Unary(op, inner) => expr("unary", (*op).into(), vec![inner]),
        NodeKind::Infix(op, a, b) => expr("infix", (*op).into(), vec![a, b]),
        NodeKind::Global(name, args) => expr("global", name.clone(), args.iter().collect()),
        NodeKind::Coll(tpe, items) => expr("coll", tpe.clone(), items.iter().collect()),
        NodeKind::Tuple(items) => expr("tuple", String::new(), items.iter().collect()),
        NodeKind::AtLeast(k, items) => expr("atLeast", String::new(), vec![k, items]),
        NodeKind::Raw(_) | NodeKind::Lambda(..) | NodeKind::Block(..) | NodeKind::If(..) => None,
    }
}

/// Only expand known box selectors and direct immutable binding aliases.
/// Freeze their keys at declaration time so later shadowing cannot retarget them.
fn box_alias(n: &Node, scope: &Scope) -> Option<Receiver> {
    fn boxes(n: &Node) -> bool {
        matches!(&n.kind, NodeKind::Leaf("OUTPUTS" | "INPUTS"))
            || matches!(&n.kind, NodeKind::Prop(obj, name)
                if name == "dataInputs" && matches!(obj.kind, NodeKind::Leaf("CONTEXT")))
            || matches!(&n.kind, NodeKind::Method(obj, name, args)
                if name == "dataInputs" && args.is_empty()
                    && matches!(obj.kind, NodeKind::Leaf("CONTEXT")))
    }
    match &n.kind {
        NodeKind::Val(_) | NodeKind::Leaf("SELF") => receiver(n, scope),
        NodeKind::ApplyFn(obj, args) if boxes(obj) && args.len() == 1 => receiver(n, scope),
        NodeKind::Index(obj, _, None) if boxes(obj) => receiver(n, scope),
        _ => None,
    }
}

/// Report every unguarded `Option.get` in `root`.
#[must_use]
pub fn unchecked_get(root: &Node) -> Vec<Finding> {
    let mut out = Vec::new();
    walk(
        root,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Scope::default(),
        &mut out,
    );
    out
}

/// Is this node `x.get` with no arguments — i.e. `Option::get`?
///
/// Arity is the discriminator: `SCollection::get` (method table 0x0C/0x21)
/// and `AvlTree::get` (0x64/0x0A) both take an index argument.
fn as_option_get(n: &Node) -> Option<&Node> {
    match &n.kind {
        NodeKind::Method(recv, name, args) if name == "get" && args.is_empty() => Some(recv),
        _ => None,
    }
}

/// Receivers this expression proves non-empty, as canonical structural keys.
///
/// Only conjunctive structure proves anything: a direct `isDefined`, the
/// operands of nested `&&` (a `&&` chain that evaluated makes every operand
/// true), and the content of a `sigmaProp(…)` wrapper — the lift renders
/// every non-root `BoolToSigmaProp` as `Global("sigmaProp", [x])`, which is
/// definitionally transparent (it holds iff `x` holds). `isDefined` under
/// `||` or negation proves nothing — `x.isDefined || y.isDefined` can hold
/// with `x` empty — so those subtrees are not entered: documented
/// false-positive gaps stay false positives, never false negatives.
fn proves_defined(n: &Node, vals: &Scope, out: &mut Vec<Receiver>) {
    match &n.kind {
        NodeKind::Method(recv, name, args) if name == "isDefined" && args.is_empty() => {
            if let Some(key) = receiver(recv, vals) {
                out.push(key);
            }
        }
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => {
            proves_defined(&args[0], vals, out);
        }
        NodeKind::Infix(op, a, b) if *op == "&&" => {
            proves_defined(a, vals, out);
            proves_defined(b, vals, out);
        }
        NodeKind::Val(name) => {
            if let Some(rs) = vals.guards.get(name) {
                out.extend(rs.iter().cloned());
            }
        }
        _ => {}
    }
}

/// Receivers proven non-empty by this expression being **false** — the
/// guard set for the right operand of `||` and the else branch of `if`.
fn falsity_proves_defined(n: &Node, vals: &Scope, out: &mut Vec<Receiver>) {
    match &n.kind {
        NodeKind::Unary(op, inner) if *op == "!" => proves_defined(inner, vals, out),
        NodeKind::Global(name, args) if name == "sigmaProp" && args.len() == 1 => {
            falsity_proves_defined(&args[0], vals, out);
        }
        // `a || b` false ⇒ both false.
        NodeKind::Infix(op, a, b) if *op == "||" => {
            falsity_proves_defined(a, vals, out);
            falsity_proves_defined(b, vals, out);
        }
        _ => {}
    }
}

/// The leaf a receiver chain bottoms out in, looking through method calls,
/// properties, indexing and dynamic register reads.
fn receiver_root(n: &Node) -> &Node {
    match &n.kind {
        NodeKind::Method(o, _, _)
        | NodeKind::Prop(o, _)
        | NodeKind::ApplyFn(o, _)
        | NodeKind::GetRegDyn(o, _, _)
        | NodeKind::Index(o, _, _) => receiver_root(o),
        _ => n,
    }
}

/// Severity and message for an unguarded `get` on `recv`, given the lambda
/// parameters in scope.
fn classify(recv: &Node, params: &[String]) -> (Severity, String) {
    match &receiver_root(recv).kind {
        NodeKind::GetVar(id, _) => (
            Severity::Low,
            format!(
                "Option.get on context variable {id} with no isDefined guard — the spender must \
                 supply it or the script throws; not a lock, since the spender controls the \
                 extension."
            ),
        ),
        NodeKind::Val(name) if params.contains(name) => (
            Severity::Medium,
            "Option.get on a collection element with no isDefined guard — the script throws \
             for any element lacking the value; elements are chosen by the spender, so this \
             is fragile rather than locking (unless the element is SELF)."
                .into(),
        ),
        _ => (
            Severity::High,
            "Option.get with no isDefined guard — the script throws if the option is empty, \
             making this spending path unusable."
                .into(),
        ),
    }
}

fn walk(
    n: &Node,
    guarded: &mut Vec<Receiver>,
    params: &mut Vec<String>,
    vals: &mut Scope,
    out: &mut Vec<Finding>,
) {
    // Scope-introducing shapes: everything the left/condition proves is
    // available to the right/then branch.
    match &n.kind {
        NodeKind::Infix(op, lhs, rhs) if *op == "&&" => {
            walk(lhs, guarded, params, vals, out);
            let depth = guarded.len();
            proves_defined(lhs, vals, guarded);
            walk(rhs, guarded, params, vals, out);
            guarded.truncate(depth);
            return;
        }
        NodeKind::Infix(op, lhs, rhs) if *op == "||" => {
            walk(lhs, guarded, params, vals, out);
            let depth = guarded.len();
            falsity_proves_defined(lhs, vals, guarded);
            walk(rhs, guarded, params, vals, out);
            guarded.truncate(depth);
            return;
        }
        NodeKind::If(cond, then_b, else_b) => {
            walk(cond, guarded, params, vals, out);
            let depth = guarded.len();
            proves_defined(cond, vals, guarded);
            walk(then_b, guarded, params, vals, out);
            guarded.truncate(depth);
            falsity_proves_defined(cond, vals, guarded);
            walk(else_b, guarded, params, vals, out);
            guarded.truncate(depth);
            return;
        }
        NodeKind::Lambda(names, body) => {
            let depth = params.len();
            let saved_guards = vals.guards.clone();
            let saved_bindings = vals.bindings.clone();
            // Parameters are rendered `name: Type`; references are bare names.
            for param in names {
                let name = param.split(':').next().unwrap_or(param).trim();
                params.push(name.to_string());
                vals.bind(name, None, Vec::new());
            }
            walk(body, guarded, params, vals, out);
            vals.guards = saved_guards;
            vals.bindings = saved_bindings;
            params.truncate(depth);
            return;
        }
        NodeKind::Block(stmts, result) => {
            // Statements bind in order: a val is a guard for everything
            // after its definition in this block, and nothing before.
            let saved_guards = vals.guards.clone();
            let saved_bindings = vals.bindings.clone();
            for st in stmts {
                let (name, expr) = match st {
                    Stmt::Val(n, e) | Stmt::Def(n, e) => (n, e),
                };
                walk(expr, guarded, params, vals, out);
                let mut proves = Vec::new();
                let alias = if matches!(st, Stmt::Val(..)) {
                    proves_defined(expr, vals, &mut proves);
                    box_alias(expr, vals)
                } else {
                    None
                };
                vals.bind(name, alias, proves);
            }
            walk(result, guarded, params, vals, out);
            vals.guards = saved_guards;
            vals.bindings = saved_bindings;
            return;
        }
        _ => {}
    }

    if let Some(recv) = as_option_get(n) {
        if !receiver(recv, vals).is_some_and(|key| guarded.contains(&key)) {
            let (severity, message) = classify(recv, params);
            out.push(Finding {
                triage: Default::default(),
                lint: "unchecked-get",
                severity,
                node_id: n.id,
                ir_id: None,
                message,
                snippet: snippet(n),
            });
        }
    }

    for c in children(n) {
        walk(c, guarded, params, vals, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(kind: NodeKind) -> Node {
        Node { id: 0, kind }
    }
    fn val(name: &str) -> Node {
        node(NodeKind::Val(name.into()))
    }
    fn output(index: Node) -> Node {
        node(NodeKind::ApplyFn(
            Box::new(node(NodeKind::Leaf("OUTPUTS"))),
            vec![index],
        ))
    }
    fn out(index: i64) -> Node {
        output(node(NodeKind::Int(index)))
    }
    fn reg(obj: Node, name: &str, method: &str) -> Node {
        node(NodeKind::Method(
            Box::new(node(NodeKind::Prop(Box::new(obj), name.into()))),
            method.into(),
            vec![],
        ))
    }
    fn guard(obj: Node) -> Node {
        reg(obj, "R4[Long]", "isDefined")
    }
    fn read(obj: Node) -> Node {
        reg(obj, "R4[Long]", "get")
    }
    fn and(a: Node, b: Node) -> Node {
        node(NodeKind::Infix("&&", Box::new(a), Box::new(b)))
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
    fn count(root: Node) -> usize {
        unchecked_get(&root).len()
    }

    #[test]
    fn box_alias_chains_match_guards_in_both_directions() {
        for (guarded, read_from) in [(out(0), val("b")), (val("b"), out(0))] {
            assert_eq!(
                count(block(
                    vec![("a", out(0)), ("b", val("a"))],
                    and(guard(guarded), read(read_from))
                )),
                0
            );
        }
    }

    #[test]
    fn known_box_selectors_are_canonicalised() {
        let data_inputs = node(NodeKind::Method(
            Box::new(node(NodeKind::Leaf("CONTEXT"))),
            "dataInputs".into(),
            vec![],
        ));
        let selectors = [
            node(NodeKind::Leaf("SELF")),
            node(NodeKind::ApplyFn(
                Box::new(node(NodeKind::Leaf("INPUTS"))),
                vec![node(NodeKind::Int(0))],
            )),
            node(NodeKind::ApplyFn(
                Box::new(data_inputs),
                vec![node(NodeKind::Int(0))],
            )),
            node(NodeKind::Index(
                Box::new(node(NodeKind::Leaf("OUTPUTS"))),
                Box::new(node(NodeKind::Int(0))),
                None,
            )),
        ];
        for selector in selectors {
            assert_eq!(
                count(block(
                    vec![("a", selector.clone())],
                    and(guard(selector), read(val("a")))
                )),
                0
            );
        }
    }

    #[test]
    fn unguarded_aliases_and_different_receivers_are_reported() {
        for condition in [
            node(NodeKind::Bool(true)),
            guard(out(1)),
            reg(out(0), "R5[Long]", "isDefined"),
            reg(out(0), "R4[Int]", "isDefined"),
        ] {
            assert_eq!(
                count(block(
                    vec![("a", out(0)), ("b", val("a"))],
                    and(condition, read(val("b")))
                )),
                1
            );
        }
    }

    #[test]
    fn box_shadowing_does_not_inherit_guards_and_restores_outer_alias() {
        let inner = block(vec![("a", out(1))], read(val("a")));
        assert_eq!(
            count(block(
                vec![("a", out(0))],
                and(guard(val("a")), and(inner, read(val("a"))))
            )),
            1
        );
    }

    #[test]
    fn aliases_capture_index_bindings_before_shadowing() {
        let inner = block(
            vec![("i", node(NodeKind::Int(1)))],
            and(read(output(val("i"))), read(val("b"))),
        );
        assert_eq!(
            count(block(
                vec![
                    ("i", node(NodeKind::Int(0))),
                    ("a", output(val("i"))),
                    ("b", val("a"))
                ],
                and(guard(output(val("i"))), inner)
            )),
            1
        );
    }

    #[test]
    fn boolean_shadowing_hides_and_restores_outer_proofs() {
        for replacement in [node(NodeKind::Bool(true)), guard(out(1))] {
            let inner = block(vec![("ok", replacement)], and(val("ok"), read(out(0))));
            assert_eq!(
                count(block(
                    vec![("ok", guard(out(0)))],
                    and(inner, and(val("ok"), read(out(0))))
                )),
                1
            );
        }
    }

    #[test]
    fn lambda_parameters_hide_box_aliases_and_boolean_guards() {
        let body = and(val("ok"), and(read(val("a")), read(out(0))));
        let lambda = node(NodeKind::Lambda(
            vec!["a: Box".into(), "ok: Boolean".into()],
            Box::new(body),
        ));
        assert_eq!(
            count(block(
                vec![("a", out(0)), ("ok", guard(out(0)))],
                and(lambda, and(val("ok"), read(val("a"))))
            )),
            2
        );
    }

    #[test]
    fn active_outer_guard_cannot_guard_a_shadowing_lambda_parameter() {
        let inner = node(NodeKind::Lambda(
            vec!["a: Box".into()],
            Box::new(read(val("a"))),
        ));
        let outer = node(NodeKind::Lambda(
            vec!["a: Box".into()],
            Box::new(and(guard(val("a")), inner)),
        ));
        assert_eq!(count(outer), 1);
    }

    #[test]
    fn unsupported_expressions_are_not_assumed_equal() {
        let unknown = node(NodeKind::Raw("unknown box".into()));
        assert_eq!(count(and(guard(unknown.clone()), read(unknown.clone()))), 1);
        assert_eq!(
            count(block(
                vec![("a", unknown.clone()), ("b", unknown)],
                and(guard(val("a")), read(val("b")))
            )),
            1
        );
    }

    #[test]
    fn unknown_box_binding_can_still_be_guarded_through_a_direct_alias() {
        assert_eq!(
            count(block(
                vec![
                    ("a", node(NodeKind::Raw("unknown".into()))),
                    ("b", val("a"))
                ],
                and(guard(val("a")), read(val("b")))
            )),
            0
        );
    }
}
