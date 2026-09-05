//! Clause recognition: the lifted AST of a contract put back into the
//! composer's language — "the key 9fSg…, from block 900000" — so a reader
//! can say what an address means without reading ErgoScript. Every path
//! (an OR at the top) becomes one sentence; a shape the recognizer does
//! not know is quoted as code and marks the result incomplete, never
//! paraphrased.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::decompile::ast::{Node, NodeKind, Stmt};
use crate::decompile::Lifted;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plain {
    /// One sentence per way to spend, in tree order.
    pub paths: Vec<String>,
    /// False when any clause had to be quoted as code.
    pub complete: bool,
}

struct Cx<'a> {
    env: BTreeMap<String, &'a Node>,
    complete: bool,
}

/// Put a lifted contract into words.
pub fn plain(lifted: &Lifted) -> Plain {
    let mut cx = Cx {
        env: BTreeMap::new(),
        complete: lifted.raw_placeholders == 0 && !lifted.truncated,
    };
    let root = cx.enter(&lifted.node);
    let mut paths = Vec::new();
    for alt in flatten(root, "||") {
        paths.push(cx.path(alt));
    }
    Plain {
        paths,
        complete: cx.complete,
    }
}

/// `a || b || c` → [a, b, c]; anything else → [node].
fn flatten<'a>(n: &'a Node, op: &str) -> Vec<&'a Node> {
    match &n.kind {
        NodeKind::Infix(o, a, b) if *o == op => {
            let mut v = flatten(a, op);
            v.extend(flatten(b, op));
            v
        }
        _ => vec![n],
    }
}

impl<'a> Cx<'a> {
    /// Bind a block's vals; return its result node.
    fn enter(&mut self, n: &'a Node) -> &'a Node {
        match &n.kind {
            NodeKind::Block(stmts, result) => {
                for s in stmts {
                    match s {
                        Stmt::Val(name, v) | Stmt::Def(name, v) => {
                            self.env.insert(name.clone(), v);
                        }
                    }
                }
                self.enter(result)
            }
            _ => n,
        }
    }

    fn resolve(&self, n: &'a Node) -> &'a Node {
        match &n.kind {
            NodeKind::Val(name) => self.env.get(name).map(|v| self.resolve(v)).unwrap_or(n),
            _ => n,
        }
    }

    /// One way to spend: who, then the conditions.
    fn path(&mut self, n: &'a Node) -> String {
        let mut keys: Vec<String> = Vec::new();
        let mut conds: Vec<String> = Vec::new();
        for part in flatten(self.resolve(n), "&&") {
            let part = self.resolve(part);
            if !self.is_sigma(part) {
                // A bare boolean at this level: the compiler folds the
                // `sigmaProp` wrapper away when the whole script is one.
                let s = self.cond(part);
                conds.push(s);
                continue;
            }
            match &part.kind {
                NodeKind::Global(f, args) if f == "sigmaProp" && args.len() == 1 => {
                    let c = self.resolve(&args[0]);
                    match &c.kind {
                        NodeKind::Bool(true) => {}
                        NodeKind::Bool(false) => conds.push("never".into()),
                        _ => {
                            for cc in flatten(c, "&&") {
                                let s = self.cond(cc);
                                conds.push(s);
                            }
                        }
                    }
                }
                _ => keys.push(self.who(part)),
            }
        }
        let who = match keys.len() {
            0 => "anyone".to_string(),
            1 => keys.remove(0),
            _ => format!("all of {}", keys.join(" and ")),
        };
        if conds.is_empty() {
            who
        } else {
            format!("{who}, if {}", conds.join(" and "))
        }
    }

    /// Does this node stand for a sigma proposition (a key, a threshold, a
    /// `sigmaProp(...)`, or a combination of those)?
    fn is_sigma(&self, n: &'a Node) -> bool {
        let n = self.resolve(n);
        match &n.kind {
            NodeKind::Const(c) => {
                c.starts_with("PK(")
                    || c.starts_with("proveDlog(")
                    || c == "true"
                    || c == "false"
                    || c.starts_with("sigmaProp(")
            }
            NodeKind::AtLeast(..) => true,
            NodeKind::Global(f, _) => matches!(
                f.as_str(),
                "sigmaProp" | "proveDlog" | "proveDHTuple" | "atLeast"
            ),
            NodeKind::Infix(op, a, b) if *op == "||" || *op == "&&" => {
                self.is_sigma(a) && self.is_sigma(b)
            }
            NodeKind::Bool(_) => true,
            _ => false,
        }
    }

    /// A sigma-typed node.
    fn who(&mut self, n: &'a Node) -> String {
        let n = self.resolve(n);
        match &n.kind {
            NodeKind::Const(c) if c == "false" || c == "sigmaProp(false)" => "nobody, ever".into(),
            NodeKind::Const(c) if c == "true" || c == "sigmaProp(true)" => "anyone".into(),
            NodeKind::Bool(false) => "nobody, ever".into(),
            NodeKind::Bool(true) => "anyone".into(),
            NodeKind::Const(c) => key_name(c),
            NodeKind::AtLeast(k, coll) => {
                let k = self.resolve(k);
                let coll = self.resolve(coll);
                let items: Vec<String> = match &coll.kind {
                    NodeKind::Coll(_, items) => items.iter().map(|i| self.who(i)).collect(),
                    _ => vec![self.quote(coll)],
                };
                format!(
                    "{} of these {} keys ({})",
                    self.text(k),
                    items.len(),
                    items.join(", ")
                )
            }
            NodeKind::Infix("||", ..) => {
                let alts: Vec<String> = flatten(n, "||").into_iter().map(|a| self.who(a)).collect();
                format!("any of {}", alts.join(" or "))
            }
            NodeKind::Infix("&&", ..) => {
                let alts: Vec<String> = flatten(n, "&&").into_iter().map(|a| self.who(a)).collect();
                format!("all of {}", alts.join(" and "))
            }
            NodeKind::Method(reg, m, args) if m == "get" || m == "getOrElse" => {
                let reg = self.resolve(reg);
                if let NodeKind::Prop(bx, r) = &reg.kind {
                    if r.starts_with('R') {
                        let base =
                            format!("the key recorded in {}'s {}", self.place(bx), reg_name(r));
                        return if m == "getOrElse" && args.len() == 1 {
                            format!("{base} (or {} if unset)", self.who(&args[0]))
                        } else {
                            base
                        };
                    }
                }
                self.quote(n)
            }
            NodeKind::Global(f, args) if f == "proveDlog" && args.len() == 1 => {
                format!("the key {}", self.text(&args[0]))
            }
            NodeKind::Global(f, args) if f == "proveDHTuple" && args.len() == 4 => {
                "whoever knows the Diffie-Hellman secret behind this box".to_string()
            }
            NodeKind::Global(f, args) if f == "sigmaProp" && args.len() == 1 => {
                match &self.resolve(&args[0]).kind {
                    NodeKind::Bool(true) => "anyone".into(),
                    NodeKind::Bool(false) => "nobody, ever".into(),
                    _ => format!("anyone, if {}", self.cond(&args[0])),
                }
            }
            _ => self.quote(n),
        }
    }

    /// A boolean clause.
    fn cond(&mut self, n: &'a Node) -> String {
        let n = self.resolve(n);
        use NodeKind::*;
        match &n.kind {
            Infix("||", ..) => {
                let alts: Vec<String> = flatten(n, "||")
                    .into_iter()
                    .map(|a| {
                        let s = self.cond(a);
                        if s.contains(" and ") {
                            format!("({s})")
                        } else {
                            s
                        }
                    })
                    .collect();
                format!("either {}", alts.join(", or "))
            }
            Infix("&&", ..) => {
                let alts: Vec<String> =
                    flatten(n, "&&").into_iter().map(|a| self.cond(a)).collect();
                alts.join(" and ")
            }
            Infix(op, a, b) => {
                let a = self.resolve(a);
                let b = self.resolve(b);
                if let Some(s) = self.compare(op, a, b) {
                    return s;
                }
                self.quote(n)
            }
            Method(obj, m, args) if m == "isDefined" && args.is_empty() => {
                let obj = self.resolve(obj);
                match &obj.kind {
                    GetVar(i, _) => format!("the spender attaches variable {i}"),
                    Prop(bx, reg) if reg.starts_with('R') => {
                        format!("{} has {}", self.place(bx), reg_name(reg))
                    }
                    _ => self.quote(n),
                }
            }
            Unary("!", inner) => format!("not ({})", self.cond(inner)),
            Bool(true) => "always".into(),
            Bool(false) => "never".into(),
            _ => self.quote(n),
        }
    }

    /// `a <op> b` in words, for the shapes the composer produces.
    fn compare(&mut self, op: &str, a: &'a Node, b: &'a Node) -> Option<String> {
        use NodeKind::*;
        // `x.size` may lift as a property or a method; treat both alike.
        if let Prop(obj, f) = &a.kind {
            if f == "size" {
                let n = self.count_of(obj);
                if let Some(what) = n {
                    let rhs = self.text(b);
                    return Some(match op {
                        ">" => format!("{what} is more than {rhs}"),
                        ">=" => format!("{what} is at least {rhs}"),
                        "==" => format!("{what} is exactly {rhs}"),
                        "<" => format!("{what} is less than {rhs}"),
                        _ => format!("{what} is not {rhs}"),
                    });
                }
            }
        }
        let rel: &str = match op {
            ">=" => "is at least",
            ">" => "is more than",
            "<=" => "is at most",
            "<" => "is less than",
            "==" => "is",
            "!=" => "is not",
            _ => return None,
        };
        // HEIGHT
        if matches!(&a.kind, Leaf("HEIGHT")) {
            let h = self.text(b);
            return Some(match op {
                ">=" => format!("from block {h}"),
                ">" => format!("after block {h}"),
                "<" => format!("before block {h}"),
                "<=" => format!("until block {h}"),
                _ => format!("the height {rel} {h}"),
            });
        }
        // timestamp
        if let Prop(inner, f) = &a.kind {
            if f == "timestamp" {
                let t = self.text(b);
                let _ = inner;
                return Some(match op {
                    ">=" | ">" => format!("from the block time {t} (ms)"),
                    "<" | "<=" => format!("until the block time {t} (ms)"),
                    _ => format!("the block time {rel} {t}"),
                });
            }
        }
        // key equality through propBytes
        if let Prop(ka, f) | Method(ka, f, _) = &a.kind {
            if f == "propBytes" && !matches!(self.resolve(ka).kind, Leaf("SELF")) {
                let b = self.resolve(b);
                if let Prop(kb, g) | Method(kb, g, _) = &b.kind {
                    if g == "propBytes" {
                        let (l, r) = (self.who(ka), self.who(kb));
                        return Some(format!("{l} {rel} {r}"));
                    }
                }
            }
        }
        // script equality
        if let Prop(bx, f) = &a.kind {
            if f == "propositionBytes" {
                let place = self.place(bx);
                let b = self.resolve(b);
                return Some(match &b.kind {
                    Prop(other, g)
                        if g == "propositionBytes"
                            && matches!(self.resolve(other).kind, Leaf("SELF")) =>
                    {
                        format!("{place} stays under this contract")
                    }
                    Method(key, m, _) if m == "propBytes" => {
                        format!("{place} goes to {}", self.who(key))
                    }
                    Const(c) => format!("{place} goes to {}", key_name(c)),
                    _ => format!("{place}'s script {rel} {}", self.quote(b)),
                });
            }
            if f == "value" {
                let place = self.place(bx);
                return Some(format!(
                    "{place} holds {} {}",
                    value_rel(op),
                    self.amount(b)
                ));
            }
            if f == "tokens" {
                let place = self.place(bx);
                let b = self.resolve(b);
                if let Prop(other, g) | Method(other, g, _) = &b.kind {
                    if g == "tokens" && matches!(self.resolve(other).kind, Leaf("SELF")) {
                        return Some(format!("{place} carries exactly this box's tokens"));
                    }
                }
            }
            // tokens(i)._1 / ._2
            if (f == "_1" || f == "_2") && op == "==" || f == "_2" {
                if let ApplyFn(tok, idx) = &self.resolve(bx).kind {
                    if let Method(bx2, m, _) = &self.resolve(tok).kind {
                        if m == "tokens" && idx.len() == 1 {
                            let place = self.place(bx2);
                            let which = ordinal(&self.text(&idx[0]));
                            return Some(if f == "_1" {
                                format!("{place}'s {which} token is {}", self.text(b))
                            } else {
                                format!("{place}'s {which} token amount {rel} {}", self.text(b))
                            });
                        }
                    }
                }
            }
        }
        if let Method(bx, f, _) = &a.kind {
            if f == "tokens" {
                let b = self.resolve(b);
                if let Prop(other, g) | Method(other, g, _) = &b.kind {
                    if g == "tokens" && matches!(self.resolve(other).kind, Leaf("SELF")) {
                        return Some(format!(
                            "{} carries exactly this box's tokens",
                            self.place(bx)
                        ));
                    }
                }
            }
        }
        // register reads: box.R4[T].get / getOrElse(d)
        if let Method(reg, m, _) = &a.kind {
            if m == "get" || m == "getOrElse" {
                if let Prop(bx, r) = &self.resolve(reg).kind {
                    if r.starts_with('R') {
                        let place = self.place(bx);
                        let b = self.resolve(b);
                        if matches!(b.kind, Leaf("HEIGHT")) && op == "==" {
                            return Some(format!(
                                "{place}'s {} records the current height",
                                reg_name(r)
                            ));
                        }
                        return Some(format!("{place}'s {} {rel} {}", reg_name(r), self.text(b)));
                    }
                }
            }
            if m == "size" {
                let obj = self.resolve(reg);
                let what = match &obj.kind {
                    Leaf("OUTPUTS") => Some("outputs"),
                    Leaf("INPUTS") => Some("inputs"),
                    Method(c, d, _)
                        if d == "dataInputs" && matches!(self.resolve(c).kind, Leaf("CONTEXT")) =>
                    {
                        Some("data inputs")
                    }
                    _ => None,
                };
                if let Some(what) = what {
                    let n = self.text(b);
                    return Some(match op {
                        ">" => format!("there are more than {n} {what}"),
                        ">=" => format!("there are at least {n} {what}"),
                        "==" => format!("there are exactly {n} {what}"),
                        "<" => format!("there are fewer than {n} {what}"),
                        _ => format!("the number of {what} {rel} {n}"),
                    });
                }
                if let Prop(bx, t) | Method(bx, t, _) = &obj.kind {
                    if t == "tokens" {
                        let place = self.place(bx);
                        let n = self.text(b);
                        return Some(if op == "==" && n == "0" {
                            format!("{place} carries no tokens")
                        } else {
                            format!("{place}'s token count {rel} {n}")
                        });
                    }
                }
            }
        }
        // hashes of an attached secret
        if let Global(h, args) = &a.kind {
            if (h == "blake2b256" || h == "sha256") && args.len() == 1 && op == "==" {
                if let Method(v, m, _) = &self.resolve(&args[0]).kind {
                    if m == "get" {
                        if let GetVar(i, _) = &self.resolve(v).kind {
                            return Some(format!(
                                "the spender reveals a secret (variable {i}) whose {h} hash is {}",
                                self.text(b)
                            ));
                        }
                    }
                }
            }
        }
        // attached variable equals
        if let Method(v, m, _) = &a.kind {
            if m == "get" {
                if let GetVar(i, _) = &self.resolve(v).kind {
                    return Some(format!(
                        "the spender attaches variable {i} that {rel} {}",
                        self.text(b)
                    ));
                }
            }
            if m == "minerPubKey" {
                return Some(format!("the block is mined by {}", self.text(b)));
            }
        }
        // Anything else: both sides in words (unknown pieces come out quoted).
        let (l, r) = (self.text(a), self.text(b));
        Some(format!("{l} {rel} {r}"))
    }

    /// "the number of outputs", "output 1's token count", … for `x.size`.
    fn count_of(&mut self, obj: &'a Node) -> Option<String> {
        use NodeKind::*;
        let obj = self.resolve(obj);
        Some(match &obj.kind {
            Leaf("OUTPUTS") => "the number of outputs".into(),
            Leaf("INPUTS") => "the number of inputs".into(),
            Method(c, d, _)
                if d == "dataInputs" && matches!(self.resolve(c).kind, Leaf("CONTEXT")) =>
            {
                "the number of data inputs".into()
            }
            Method(c, d, _)
                if d == "headers" && matches!(self.resolve(c).kind, Leaf("CONTEXT")) =>
            {
                "the number of headers".into()
            }
            Prop(bx, t) | Method(bx, t, _) if t == "tokens" => {
                format!("{}'s token count", self.place(bx))
            }
            _ => return None,
        })
    }

    /// Where a box lives, in words.
    fn place(&mut self, bx: &'a Node) -> String {
        let bx = self.resolve(bx);
        use NodeKind::*;
        match &bx.kind {
            Leaf("SELF") => "this box".into(),
            ApplyFn(list, idx) if idx.len() == 1 => {
                let i = self.text(&idx[0]);
                match &self.resolve(list).kind {
                    Leaf("OUTPUTS") => format!("output {i}"),
                    Leaf("INPUTS") => format!("input {i}"),
                    Method(c, d, _)
                        if d == "dataInputs" && matches!(self.resolve(c).kind, Leaf("CONTEXT")) =>
                    {
                        format!("data input {i}")
                    }
                    _ => self.quote(bx),
                }
            }
            _ => self.quote(bx),
        }
    }

    /// A value in words: a number, a key, a hash, or quoted code.
    fn text(&mut self, n: &'a Node) -> String {
        let n = self.resolve(n);
        use NodeKind::*;
        match &n.kind {
            Int(i) => i.to_string(),
            Num(s) => s.trim_end_matches(['L', 'y', 's']).to_string(),
            Bool(b) => b.to_string(),
            Const(c) => const_name(c),
            Leaf(l) => l.to_string(),
            Infix(op, a, b) if matches!(*op, "+" | "-" | "*" | "/") => {
                let (a, b) = (self.text(a), self.text(b));
                let word = match *op {
                    "+" => "plus",
                    "-" => "minus",
                    "*" => "times",
                    _ => "divided by",
                };
                format!("{a} {word} {b}")
            }
            Prop(bx, f) if f == "value" => format!("{}'s value", self.place(bx)),
            Prop(obj, f) if f == "size" => match self.count_of(obj) {
                Some(w) => w,
                None => self.quote(n),
            },
            Method(obj, f, args) if f == "size" && args.is_empty() => match self.count_of(obj) {
                Some(w) => w,
                None => self.quote(n),
            },
            Prop(inner, f) if f == "_1" || f == "_2" => {
                let inner = self.resolve(inner);
                if let ApplyFn(tok, idx) = &inner.kind {
                    if let Method(bx, m, _) = &self.resolve(tok).kind {
                        if m == "tokens" && idx.len() == 1 {
                            let which = ordinal(&self.text(&idx[0]));
                            let place = self.place(bx);
                            return if f == "_1" {
                                format!("{place}'s {which} token id")
                            } else {
                                format!("{place}'s {which} token amount")
                            };
                        }
                    }
                }
                self.quote(n)
            }
            Method(inner, m, args)
                if args.is_empty()
                    && matches!(
                        m.as_str(),
                        "toLong" | "toInt" | "toBigInt" | "toShort" | "toByte"
                    ) =>
            {
                self.text(inner)
            }
            If(c, t, e) => {
                let (c, t, e) = (self.cond(c), self.text(t), self.text(e));
                format!("({t} if {c}, else {e})")
            }
            Method(reg, m, args) if (m == "get" || m == "getOrElse") => {
                let reg = self.resolve(reg);
                if let Prop(bx, r) = &reg.kind {
                    if r.starts_with('R') {
                        let base = format!("{}'s {}", self.place(bx), reg_name(r));
                        return if m == "getOrElse" && args.len() == 1 {
                            format!("{base} (or {} if unset)", self.text(&args[0]))
                        } else {
                            base
                        };
                    }
                }
                self.quote(n)
            }
            _ => self.quote(n),
        }
    }

    /// nanoERG amounts as ERG when they look like amounts.
    fn amount(&mut self, n: &'a Node) -> String {
        let n = self.resolve(n);
        match &n.kind {
            NodeKind::Num(s) | NodeKind::Const(s) => match s.trim_end_matches('L').parse::<i64>() {
                Ok(v) => format!("{} ERG", v as f64 / 1e9),
                Err(_) => self.text(n),
            },
            NodeKind::Int(i) => format!("{} ERG", *i as f64 / 1e9),
            _ => self.text(n),
        }
    }

    fn quote(&mut self, n: &Node) -> String {
        self.complete = false;
        format!("`{}`", crate::decompile::print(n))
    }
}

fn value_rel(op: &str) -> &'static str {
    match op {
        ">=" => "at least",
        ">" => "more than",
        "<=" => "at most",
        "<" => "less than",
        "==" => "exactly",
        _ => "not",
    }
}

fn reg_name(r: &str) -> String {
    r.split('[').next().unwrap_or(r).to_string()
}

fn ordinal(i: &str) -> &'static str {
    match i {
        "0" => "first",
        "1" => "second",
        "2" => "third",
        _ => "nth",
    }
}

/// `PK("9f…")` → "the key 9fSgJ7…jAV"; other constants shortened.
fn key_name(c: &str) -> String {
    if let Some(addr) = c.strip_prefix("PK(\"").and_then(|s| s.strip_suffix("\")")) {
        return format!("the key {}", short(addr));
    }
    format!("the key {}", const_name(c))
}

fn const_name(c: &str) -> String {
    if let Some(hex) = c
        .strip_prefix("fromBase16(\"")
        .and_then(|s| s.strip_suffix("\")"))
    {
        return short(hex);
    }
    if let Some(addr) = c.strip_prefix("PK(\"").and_then(|s| s.strip_suffix("\")")) {
        return short(addr);
    }
    c.to_string()
}

fn short(s: &str) -> String {
    if s.len() > 16 {
        format!("{}…{}", &s[..8], &s[s.len() - 4..])
    } else {
        s.to_string()
    }
}
