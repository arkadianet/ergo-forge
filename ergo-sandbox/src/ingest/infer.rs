//! Usage constraints over the compiler's AST. This is deliberately a partial
//! inference pass, not a replacement type checker. The real compiler validates
//! every proposed environment. Method and predef signatures come from it too.
use std::collections::BTreeMap;

use ergo_compiler::{typer, Expr, SType};

type Id = usize;

#[derive(Clone, Debug)]
enum Shape {
    Unknown,
    Atom(SType),
    Coll(Id),
    Option(Id),
    Tuple(Vec<Id>),
    Func(Vec<Id>, Id),
    Conflict(String),
}

#[derive(Clone)]
enum Pending {
    Member {
        obj: Id,
        name: String,
        types: Vec<SType>,
        args: Option<Vec<Id>>,
        out: Id,
    },
    Apply {
        func: Id,
        args: Vec<Id>,
        out: Id,
    },
    Logical {
        left: Id,
        right: Id,
        out: Id,
    },
}

pub(super) struct Inference {
    parents: Vec<Id>,
    shapes: Vec<Shape>,
    pending: Vec<Pending>,
    pub names: BTreeMap<String, (Id, Vec<usize>)>,
    version: u8,
}

impl Inference {
    pub fn new(
        source: &str,
        version: u8,
        overrides: &BTreeMap<String, SType>,
    ) -> Result<Self, String> {
        let ast = ergo_compiler::parse(source, version).map_err(|e| e.to_string())?;
        let mut s = Self {
            parents: vec![],
            shapes: vec![],
            pending: vec![],
            names: BTreeMap::new(),
            version,
        };
        s.expr(&ast, &BTreeMap::new());
        for (name, t) in overrides {
            if let Some((id, _)) = s
                .names
                .get(name)
                .or_else(|| s.names.get(&format!("${name}")))
            {
                s.hint(*id, t.clone());
            }
        }
        // Each resolved constraint adds finite type information; retain
        // unresolved constraints (unknown receivers, overloads) for reporting.
        loop {
            let pending = std::mem::take(&mut s.pending);
            let mut progressed = false;
            for p in pending {
                if s.resolve(&p) {
                    progressed = true;
                } else {
                    s.pending.push(p);
                }
            }
            if !progressed {
                break;
            }
        }
        Ok(s)
    }

    fn node(&mut self, shape: Shape) -> Id {
        let id = self.parents.len();
        self.parents.push(id);
        self.shapes.push(shape);
        id
    }
    fn unknown(&mut self) -> Id {
        self.node(Shape::Unknown)
    }
    fn root(&self, mut id: Id) -> Id {
        while self.parents[id] != id {
            id = self.parents[id];
        }
        id
    }
    fn shape(&self, id: Id) -> Shape {
        self.shapes[self.root(id)].clone()
    }

    fn instantiate_type(&mut self, t: &SType, vars: &mut BTreeMap<String, Id>) -> Id {
        let shape = match t {
            SType::NoType | SType::SAny => Shape::Unknown,
            SType::STypeVar(n) => {
                if let Some(id) = vars.get(n) {
                    return *id;
                }
                let id = self.unknown();
                vars.insert(n.clone(), id);
                return id;
            }
            SType::SColl(t) => Shape::Coll(self.instantiate_type(t, vars)),
            SType::SOption(t) => Shape::Option(self.instantiate_type(t, vars)),
            SType::STuple(ts) => {
                Shape::Tuple(ts.iter().map(|t| self.instantiate_type(t, vars)).collect())
            }
            SType::SFunc { dom, range, .. } => Shape::Func(
                dom.iter().map(|t| self.instantiate_type(t, vars)).collect(),
                self.instantiate_type(range, vars),
            ),
            t => Shape::Atom(t.clone()),
        };
        self.node(shape)
    }
    fn known(&mut self, t: SType) -> Id {
        self.instantiate_type(&t, &mut BTreeMap::new())
    }

    fn contains(&self, id: Id, target: Id, depth: usize) -> bool {
        if self.root(id) == target || depth > 64 {
            return true;
        }
        match self.shape(id) {
            Shape::Coll(x) | Shape::Option(x) => self.contains(x, target, depth + 1),
            Shape::Tuple(xs) => xs.iter().any(|x| self.contains(*x, target, depth + 1)),
            Shape::Func(xs, r) => {
                self.contains(r, target, depth + 1)
                    || xs.iter().any(|x| self.contains(*x, target, depth + 1))
            }
            _ => false,
        }
    }

    fn join(&mut self, a: Id, b: Id) {
        let a = self.root(a);
        let b = self.root(b);
        if a == b {
            return;
        }
        if self.contains(a, b, 0) || self.contains(b, a, 0) {
            self.shapes[a] = Shape::Conflict("recursive/over-deep type constraint".into());
            return;
        }
        let sa = self.shapes[a].clone();
        let sb = self.shapes[b].clone();
        self.parents[b] = a;
        self.shapes[a] = match (sa, sb) {
            (Shape::Unknown, s) | (s, Shape::Unknown) => s,
            (Shape::Conflict(e), _) | (_, Shape::Conflict(e)) => Shape::Conflict(e),
            (Shape::Atom(a), Shape::Atom(b)) if a == b => Shape::Atom(a),
            (Shape::Atom(a), Shape::Atom(b)) if typer::is_numeric(&a) && typer::is_numeric(&b) => {
                // Numeric operands can widen. Use the widest observed context,
                // not a claim about the deployer's original constant width.
                let ladder = [
                    SType::SByte,
                    SType::SShort,
                    SType::SInt,
                    SType::SLong,
                    SType::SBigInt,
                    SType::SUnsignedBigInt,
                ];
                let rank = |t: &SType| ladder.iter().position(|x| x == t).unwrap_or(0);
                Shape::Atom(if rank(&a) >= rank(&b) { a } else { b })
            }
            (Shape::Coll(a), Shape::Coll(b)) => {
                self.join(a, b);
                Shape::Coll(a)
            }
            (Shape::Option(a), Shape::Option(b)) => {
                self.join(a, b);
                Shape::Option(a)
            }
            (Shape::Tuple(a), Shape::Tuple(b)) if a.len() == b.len() => {
                for (a, b) in a.iter().zip(b) {
                    self.join(*a, b);
                }
                Shape::Tuple(a)
            }
            (Shape::Func(a, ar), Shape::Func(b, br)) if a.len() == b.len() => {
                for (a, b) in a.iter().zip(b) {
                    self.join(*a, b);
                }
                self.join(ar, br);
                Shape::Func(a, ar)
            }
            (a, b) => Shape::Conflict(format!(
                "incompatible usage constraints: {a:?} versus {b:?}"
            )),
        };
    }

    fn hint(&mut self, id: Id, t: SType) {
        let t = self.known(t);
        self.join(id, t);
    }

    pub fn inferred(&self, name: &str) -> Result<SType, String> {
        let (id, _) = self
            .names
            .get(name)
            .or_else(|| self.names.get(&format!("${name}")))
            .ok_or("no inference rule for this parameter")?;
        self.render(*id, 0)
    }
    fn render(&self, id: Id, depth: usize) -> Result<SType, String> {
        if depth > 64 {
            return Err("type nesting exceeds 64".into());
        }
        Ok(match self.shape(id) {
            Shape::Unknown => SType::NoType,
            Shape::Conflict(e) => return Err(e),
            Shape::Atom(t) => t,
            Shape::Coll(i) => SType::SColl(Box::new(self.render(i, depth + 1)?)),
            Shape::Option(i) => SType::SOption(Box::new(self.render(i, depth + 1)?)),
            Shape::Tuple(xs) => SType::STuple(
                xs.iter()
                    .map(|x| self.render(*x, depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
            Shape::Func(xs, r) => SType::SFunc {
                dom: xs
                    .iter()
                    .map(|x| self.render(*x, depth + 1))
                    .collect::<Result<_, _>>()?,
                range: Box::new(self.render(r, depth + 1)?),
                tpe_params: vec![],
            },
        })
    }

    fn expr(&mut self, e: &Expr, scope: &BTreeMap<String, Id>) -> Id {
        match e {
            Expr::IntConst { .. } => self.known(SType::SInt),
            Expr::LongConst { .. } => self.known(SType::SLong),
            Expr::BoolConst { .. } => self.known(SType::SBoolean),
            Expr::StringConst { .. } => self.known(SType::SString),
            Expr::UnitConst { .. } => self.known(SType::SUnit),
            Expr::Ident { name, pos, .. } => {
                if let Some(id) = scope.get(name) {
                    return *id;
                }
                let builtin = match name.as_str() {
                    "SELF" => Some(SType::SBox),
                    "HEIGHT" => Some(SType::SInt),
                    "INPUTS" | "OUTPUTS" => Some(SType::SColl(Box::new(SType::SBox))),
                    "CONTEXT" => Some(SType::SContext),
                    "Global" => Some(SType::SGlobal),
                    "LastBlockUtxoRootHash" => Some(SType::SAvlTree),
                    "MinerPubkey" => Some(SType::SColl(Box::new(SType::SByte))),
                    "groupGenerator" => Some(SType::SGroupElement),
                    _ => typer::predefined_env(self.version).get(name).cloned(),
                };
                if let Some(t) = builtin {
                    return self.known(t);
                }
                let id = self
                    .names
                    .get(name)
                    .map(|(id, _)| *id)
                    .unwrap_or_else(|| self.unknown());
                self.names
                    .entry(name.clone())
                    .or_insert_with(|| (id, vec![]))
                    .1
                    .push(*pos as usize);
                id
            }
            Expr::Block {
                bindings, result, ..
            } => {
                let mut scope = scope.clone();
                for b in bindings {
                    let id = self.expr(&b.body, &scope);
                    if b.given_type != SType::NoType {
                        self.hint(id, b.given_type.clone());
                    }
                    scope.insert(b.name.clone(), id);
                }
                self.expr(result, &scope)
            }
            Expr::Val(b) => {
                let id = self.expr(&b.body, scope);
                self.hint(id, b.given_type.clone());
                id
            }
            Expr::Lambda {
                args,
                given_res_type,
                body,
                ..
            } => {
                let mut scope = scope.clone();
                let ids = args
                    .iter()
                    .map(|(n, t)| {
                        let id = self.known(t.clone());
                        scope.insert(n.clone(), id);
                        id
                    })
                    .collect();
                let body = self.expr(body, &scope);
                self.hint(body, given_res_type.clone());
                self.node(Shape::Func(ids, body))
            }
            Expr::Tuple { items, .. } => {
                let ids = items.iter().map(|e| self.expr(e, scope)).collect();
                self.node(Shape::Tuple(ids))
            }
            Expr::If {
                condition,
                true_branch,
                false_branch,
                ..
            } => {
                let c = self.expr(condition, scope);
                self.hint(c, SType::SBoolean);
                let a = self.expr(true_branch, scope);
                let b = self.expr(false_branch, scope);
                self.join(a, b);
                a
            }
            Expr::Relation { left, right, .. } => {
                let a = self.expr(left, scope);
                let b = self.expr(right, scope);
                self.join(a, b);
                self.known(SType::SBoolean)
            }
            Expr::ArithOp { left, right, .. } | Expr::BitOp { left, right, .. } => {
                let a = self.expr(left, scope);
                let b = self.expr(right, scope);
                self.join(a, b);
                a
            }
            Expr::LogicalNot { input, .. } => {
                let id = self.expr(input, scope);
                self.hint(id, SType::SBoolean);
                id
            }
            Expr::Negation { input, .. } | Expr::BitInversion { input, .. } => {
                self.expr(input, scope)
            }
            Expr::Select { obj, field, .. } => self.member(obj, field, &[], None, scope),
            Expr::ApplyTypes {
                input, type_args, ..
            } => self.typed_call(input, type_args, None, scope),
            Expr::Apply { func, args, .. } => {
                let ids = args.iter().map(|e| self.expr(e, scope)).collect::<Vec<_>>();
                self.typed_call(func, &[], Some(ids), scope)
            }
            Expr::MethodCallLike {
                obj, name, args, ..
            } => {
                let ids = args.iter().map(|e| self.expr(e, scope)).collect::<Vec<_>>();
                if ["+", "*", "-", "/", "%", "min", "max", "^", "|", "&"].contains(&name.as_str())
                    && ids.len() == 1
                {
                    let a = self.expr(obj, scope);
                    self.join(a, ids[0]);
                    return a;
                }
                if ["||", "&&"].contains(&name.as_str()) && ids.len() == 1 {
                    let left = self.expr(obj, scope);
                    let out = self.unknown();
                    self.pending.push(Pending::Logical {
                        left,
                        right: ids[0],
                        out,
                    });
                    return out;
                }
                self.member(obj, name, &[], Some(ids), scope)
            }
        }
    }

    fn typed_call(
        &mut self,
        e: &Expr,
        types: &[SType],
        args: Option<Vec<Id>>,
        scope: &BTreeMap<String, Id>,
    ) -> Id {
        match e {
            Expr::ApplyTypes {
                input, type_args, ..
            } => self.typed_call(input, type_args, args, scope),
            Expr::Select { obj, field, .. } => self.member(obj, field, types, args, scope),
            Expr::Ident { name, .. } if name == "Coll" && !scope.contains_key(name) => {
                let elem = self.known(types.first().cloned().unwrap_or(SType::NoType));
                for id in args.unwrap_or_default() {
                    self.join(elem, id);
                }
                self.node(Shape::Coll(elem))
            }
            _ => {
                let f = if let Expr::Ident { name, .. } = e {
                    if !scope.contains_key(name) {
                        if let Some(t) = typer::predefined_env(self.version).get(name) {
                            let mut vars = BTreeMap::new();
                            if let SType::SFunc { tpe_params, .. } = t {
                                for (n, t) in tpe_params.iter().zip(types) {
                                    let id = self.known(t.clone());
                                    vars.insert(n.clone(), id);
                                }
                            }
                            self.instantiate_type(t, &mut vars)
                        } else {
                            self.expr(e, scope)
                        }
                    } else {
                        self.expr(e, scope)
                    }
                } else {
                    self.expr(e, scope)
                };
                let Some(args) = args else {
                    return f;
                };
                let out = self.unknown();
                self.pending.push(Pending::Apply { func: f, args, out });
                out
            }
        }
    }

    fn member(
        &mut self,
        obj: &Expr,
        name: &str,
        types: &[SType],
        args: Option<Vec<Id>>,
        scope: &BTreeMap<String, Id>,
    ) -> Id {
        let obj = self.expr(obj, scope);
        let out = self.unknown();
        self.pending.push(Pending::Member {
            obj,
            name: name.into(),
            types: types.to_vec(),
            args,
            out,
        });
        out
    }

    fn resolve(&mut self, p: &Pending) -> bool {
        match p {
            Pending::Logical { left, right, out } => {
                let l = self.render(*left, 0).ok();
                let r = self.render(*right, 0).ok();
                let t = if l == Some(SType::SSigmaProp) || r == Some(SType::SSigmaProp) {
                    SType::SSigmaProp
                } else if l == Some(SType::SBoolean) || r == Some(SType::SBoolean) {
                    SType::SBoolean
                } else {
                    return false;
                };
                for id in [left, right] {
                    if matches!(self.shape(*id), Shape::Unknown) {
                        self.hint(*id, t.clone());
                    }
                }
                self.hint(*out, t);
                true
            }
            Pending::Apply { func, args, out } => {
                match self.shape(*func) {
                    Shape::Func(dom, r) if dom.len() == args.len() => {
                        for (d, a) in dom.iter().zip(args) {
                            self.join(*d, *a);
                        }
                        self.join(*out, r);
                    }
                    Shape::Coll(elem) if args.len() == 1 => {
                        self.hint(args[0], SType::SInt);
                        self.join(*out, elem);
                    }
                    Shape::Unknown if args.len() == 1 => {
                        // Bare application of an environment value is indexing:
                        // ScriptEnv cannot carry function values.
                        if !self
                            .names
                            .values()
                            .any(|(id, _)| self.root(*id) == self.root(*func))
                        {
                            return false;
                        }
                        self.hint(args[0], SType::SInt);
                        let coll = self.node(Shape::Coll(*out));
                        self.join(*func, coll);
                    }
                    _ => return false,
                }
                true
            }
            Pending::Member {
                obj,
                name,
                types,
                args,
                out,
            } => {
                if let Some(i) = name
                    .strip_prefix('_')
                    .and_then(|n| n.parse::<usize>().ok())
                    .and_then(|i| i.checked_sub(1))
                {
                    if let Shape::Tuple(xs) = self.shape(*obj) {
                        if let Some(id) = xs.get(i) {
                            self.join(*out, *id);
                            return true;
                        }
                    }
                    return false;
                }
                if matches!(self.shape(*obj), Shape::Unknown)
                    && ["size", "slice", "append", "getOrElse"].contains(&name.as_str())
                {
                    // Only assume a collection for a free environment receiver;
                    // an unresolved local may instead turn out to be an Option.
                    if self
                        .names
                        .values()
                        .any(|(id, _)| self.root(*id) == self.root(*obj))
                        && name != "getOrElse"
                    {
                        self.hint(*obj, SType::SColl(Box::new(SType::NoType)));
                    }
                }
                let Ok(t) = self.render(*obj, 0) else {
                    return false;
                };
                let Some(desc) = typer::get_method(&t, name, self.version) else {
                    return false;
                };
                let mut vars = BTreeMap::new();
                for (n, t) in desc.stype.tpe_params.iter().zip(types) {
                    let id = self.known(t.clone());
                    vars.insert(n.clone(), id);
                }
                let dom: Vec<_> = desc
                    .stype
                    .dom
                    .iter()
                    .map(|t| self.instantiate_type(t, &mut vars))
                    .collect();
                let r = self.instantiate_type(&desc.stype.range, &mut vars);
                if let Some(receiver) = dom.first() {
                    self.join(*obj, *receiver);
                }
                match args {
                    Some(args) if args.len() == dom.len().saturating_sub(1) => {
                        for (d, a) in dom.iter().skip(1).zip(args) {
                            self.join(*d, *a);
                        }
                        self.join(*out, r);
                    }
                    Some(args) if dom.len() == 1 => {
                        // `box.tokens(0)` applies an index to the collection
                        // returned by the nullary `tokens` property.
                        self.pending.push(Pending::Apply {
                            func: r,
                            args: args.clone(),
                            out: *out,
                        });
                    }
                    None if dom.len() == 1 => self.join(*out, r),
                    None => {
                        let f = self.node(Shape::Func(dom.into_iter().skip(1).collect(), r));
                        self.join(*out, f);
                    }
                    _ => return false,
                }
                true
            }
        }
    }
}
