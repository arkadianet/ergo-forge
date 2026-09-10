//! Direct structural necessity over the pinned wire IR. No lifted source,
//! evaluator, discovery authority, transitive closure, or action construction.
use super::{
    necessity::{Anchor, Derivation},
    relations::*,
};
use crate::evidence::{
    case::engine_revision,
    validate::{BlockContext, NetworkRules, Parameters, Policy},
    wire::WireBox,
};
use ergo_ser::{
    ergo_tree::ErgoTree,
    opcode::{children, node_opcode, preorder, Expr, Payload},
    sigma_type::SigmaType,
    sigma_value::{CollValue, SigmaValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// A supplied-state restriction, not an assumed co-spend. Unknown fields fail
/// closed. Height is fixed for replay; branching rules use only P.guard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StateDomain {
    pub block_context: BlockContext,
    pub parameters: Parameters,
    pub network_rules: NetworkRules,
    pub headers: Vec<String>,
    pub prior_block_cost: u64,
    pub local_policy: Policy,
    /// State restriction checked independently on every supplied replay input.
    pub positive_input_tokens: bool,
}
/// Produced only by fresh checking; reports cannot deserialize into authority.
#[derive(Debug)]
pub struct Established {
    claim: RelationProposal,
    derivation: Derivation,
    rule: String,
}
impl Established {
    pub fn claim(&self) -> &RelationProposal {
        &self.claim
    }
    pub fn report(&self) -> Value {
        json!({"version":"required-relations:v1", "status":"established-under-premises", "method":"static-exact-tree-derivation", "claimDigest":self.claim.claim_digest(), "premiseDigest":self.claim.premises.digest(), "claim":self.claim, "derivation":self.derivation, "rule":self.rule})
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum BoxRef {
    Input(u32),
    Data(u32),
    Bound(u64),
    SelfBox,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Field {
    Id(BoxRef),
    Script(BoxRef),
    Hash(BoxRef),
    Token(BoxRef, u32),
    Register(BoxRef, u8),
    Distinct(BoxRef),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Fact {
    field: Field,
    bytes: Vec<u8>,
}
#[derive(Clone)]
enum Binding<'a> {
    Alias(&'a Expr),
    Box(u64),
}
type Env<'a> = BTreeMap<u32, Binding<'a>>;
struct Walk<'a> {
    tree: &'a ErgoTree,
    guard: &'a Guard,
    visits: usize,
    cap: usize,
}
fn op(e: &Expr, code: u8) -> Option<&Payload> {
    match e {
        Expr::Op(n) if n.opcode == code => Some(&n.payload),
        _ => None,
    }
}
fn resolve<'a>(mut e: &'a Expr, env: &Env<'a>) -> Result<&'a Expr, String> {
    for _ in 0..110 {
        match op(e, 0x72) {
            Some(Payload::ValUse { id }) => match env.get(id) {
                Some(Binding::Alias(rhs)) => e = rhs,
                Some(Binding::Box(_)) => return Ok(e),
                None => return Err("unbound wire variable".into()),
            },
            _ => return Ok(e),
        }
    }
    Err("ambiguous or cyclic alias".into())
}
impl<'a> Walk<'a> {
    fn constant(&self, e: &'a Expr, env: &Env<'a>) -> Option<(&SigmaType, &SigmaValue)> {
        match resolve(e, env).ok()? {
            Expr::Const { tpe, val } => Some((tpe, val)),
            Expr::Op(n) if n.opcode == 0x73 => match n.payload {
                Payload::ConstPlaceholder { index } => {
                    self.tree.constants.get(index as usize).map(|(t, v)| (t, v))
                }
                _ => None,
            },
            _ => None,
        }
    }
    fn int(&self, e: &'a Expr, env: &Env<'a>) -> Option<i32> {
        match self.constant(e, env)? {
            (SigmaType::SInt, SigmaValue::Int(i)) => Some(*i),
            _ => None,
        }
    }
    fn bytes(&self, e: &'a Expr, env: &Env<'a>) -> Option<Vec<u8>> {
        match self.constant(e, env)? {
            (SigmaType::SColl(t), SigmaValue::Coll(CollValue::Bytes(b)))
                if **t == SigmaType::SByte =>
            {
                Some(b.clone())
            }
            _ => None,
        }
    }
    fn index(&self, e: &'a Expr, env: &Env<'a>) -> Option<(&'a Expr, u32)> {
        match op(resolve(e, env).ok()?, 0xB2)? {
            Payload::ByIndex {
                input,
                index,
                default: None,
            } => Some((
                resolve(input, env).ok()?,
                self.int(index, env)?.try_into().ok()?,
            )),
            _ => None,
        }
    }
    fn box_ref(&self, e: &'a Expr, env: &Env<'a>) -> Option<BoxRef> {
        let e = resolve(e, env).ok()?;
        if matches!(op(e, 0xA7), Some(Payload::Zero)) {
            return Some(BoxRef::SelfBox);
        }
        if let Some(Payload::ValUse { id }) = op(e, 0x72) {
            if let Some(Binding::Box(scope)) = env.get(id) {
                return Some(BoxRef::Bound(*scope));
            }
        }
        let (collection, index) = self.index(e, env)?;
        if matches!(op(collection, 0xA4), Some(Payload::Zero)) {
            return Some(BoxRef::Input(index));
        }
        match op(collection, 0xDB) {
            Some(Payload::MethodCall {
                type_id: 101,
                method_id: 1,
                obj,
                args,
                type_args,
            }) if args.is_empty()
                && type_args.is_empty()
                && matches!(op(obj, 0xFE), Some(Payload::Zero)) =>
            {
                Some(BoxRef::Data(index))
            }
            _ => None,
        }
    }
    fn field(&self, e: &'a Expr, env: &Env<'a>) -> Option<Field> {
        let e = resolve(e, env).ok()?;
        for (code, kind) in [(0xC5, 0), (0xC2, 1)] {
            if let Some(Payload::One(b)) = op(e, code) {
                let b = self.box_ref(b, env)?;
                return Some(if kind == 0 {
                    Field::Id(b)
                } else {
                    Field::Script(b)
                });
            }
        }
        if let Some(Payload::One(inner)) = op(e, 0xCB) {
            if let Some(Field::Script(b)) = self.field(inner, env) {
                return Some(Field::Hash(b));
            }
        }
        if let Some(Payload::SelectField {
            input,
            field_idx: 1,
        }) = op(e, 0x8C)
        {
            let (tokens, index) = self.index(input, env)?;
            if let Some(Payload::MethodCall {
                type_id: 99,
                method_id: 8,
                obj,
                args,
                type_args,
            }) = op(tokens, 0xDB)
            {
                if args.is_empty() && type_args.is_empty() {
                    return Some(Field::Token(self.box_ref(obj, env)?, index));
                }
            }
        }
        if let Some(Payload::One(inner)) = op(e, 0xE4) {
            if let Some(Payload::ExtractRegisterAs { input, reg_id, tpe }) =
                op(resolve(inner, env).ok()?, 0xC6)
            {
                if *tpe == SigmaType::SColl(Box::new(SigmaType::SByte)) && (4..=9).contains(reg_id)
                {
                    return Some(Field::Register(self.box_ref(input, env)?, *reg_id));
                }
            }
        }
        None
    }
    fn guard_expr(&self, e: &'a Expr, env: &Env<'a>) -> Option<Guard> {
        let e = resolve(e, env).ok()?;
        if let Some(Payload::Two(a, b)) = op(e, 0x92) {
            if matches!(op(resolve(a, env).ok()?, 0xA3), Some(Payload::Zero)) {
                return Some(Guard::HeightAtLeast {
                    value: self.int(b, env)?,
                });
            }
        }
        if let Some(Payload::Two(a, b)) = op(e, 0x93) {
            for (field, value) in [(a, b), (b, a)] {
                let field = resolve(field, env).ok()?;
                let named = if matches!(op(field, 0xA3), Some(Payload::Zero)) {
                    Some(GuardField::Context {
                        name: "HEIGHT".into(),
                    })
                } else if let Some(Payload::One(b)) = op(field, 0xC1) {
                    if self.box_ref(b, env) == Some(BoxRef::SelfBox) {
                        Some(GuardField::SelfBox {
                            name: "value".into(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                let literal = match self.constant(value, env) {
                    Some((SigmaType::SInt, SigmaValue::Int(v))) => Some(Literal::Int(*v)),
                    Some((SigmaType::SLong, SigmaValue::Long(v))) => Some(Literal::Long(*v)),
                    _ => None,
                };
                if let (Some(field), Some(literal)) = (named, literal) {
                    return Some(Guard::Equals { field, literal });
                }
            }
        }
        for code in [0xED, 0xEC] {
            if let Some(Payload::Two(a, b)) = op(e, code) {
                let guards = vec![self.guard_expr(a, env)?, self.guard_expr(b, env)?];
                return Some(if code == 0xED {
                    Guard::And { guards }
                } else {
                    Guard::Or { guards }
                });
            }
        }
        if let Some(Payload::One(a)) = op(e, 0xEF) {
            return Some(Guard::Not {
                guard: Box::new(self.guard_expr(a, env)?),
            });
        }
        None
    }
    fn guard_value(&self, e: &'a Expr, env: &Env<'a>) -> Option<bool> {
        let condition = self.guard_expr(e, env)?;
        fn contains(g: &Guard, c: &Guard) -> bool {
            g == c || matches!(g,Guard::And{guards} if guards.iter().any(|g|contains(g,c)))
        }
        if contains(self.guard, &condition) {
            Some(true)
        } else if contains(
            self.guard,
            &Guard::Not {
                guard: Box::new(condition),
            },
        ) {
            Some(false)
        } else {
            None
        }
    }
    // Validate captures at the definition site, with lexical parameter and
    // block scopes. A later environment must never repair a forward capture.
    fn check_scope(&mut self, e: &Expr, available: &BTreeSet<u32>) -> Result<(), String> {
        self.visits += 1;
        if self.visits > self.cap {
            return Err("analysis traversal cap".into());
        }
        if let Expr::Op(n) = e {
            match &n.payload {
                Payload::ValUse { id } => {
                    if !available.contains(id) {
                        return Err("forward alias".into());
                    }
                }
                Payload::FuncValue { args, body } => {
                    let mut local = available.clone();
                    for (id, _) in args {
                        if !local.insert(*id) {
                            return Err("ambiguous wire binding".into());
                        }
                    }
                    self.check_scope(body, &local)?;
                }
                Payload::BlockValue { items, result } => {
                    let mut local = available.clone();
                    for item in items {
                        let Some(Payload::ValDef { id, rhs, .. }) = op(item, 0xD6) else {
                            return Err("unsupported block definition".into());
                        };
                        self.check_scope(rhs, &local)?;
                        if !local.insert(*id) {
                            return Err("ambiguous wire binding".into());
                        }
                    }
                    self.check_scope(result, &local)?;
                }
                _ => {
                    for child in children(e) {
                        self.check_scope(child, available)?;
                    }
                }
            }
        }
        Ok(())
    }
    fn facts(&mut self, e: &'a Expr, env: &Env<'a>) -> Result<BTreeSet<Fact>, String> {
        self.visits += 1;
        if self.visits > self.cap {
            return Err("analysis traversal cap".into());
        }
        let e = resolve(e, env)?;
        let empty = BTreeSet::new();
        let Expr::Op(n) = e else { return Ok(empty) };
        match (n.opcode, &n.payload) {
            (0xD8, Payload::BlockValue { items, result }) => {
                let mut local = env.clone();
                for item in items {
                    let Some(Payload::ValDef { id, rhs, .. }) = op(item, 0xD6) else {
                        return Err("unsupported block definition".into());
                    };
                    // Reject shadowing, forward references and alias ambiguity.
                    if local.contains_key(id) {
                        return Err("ambiguous wire binding".into());
                    }
                    self.check_scope(rhs, &local.keys().copied().collect())?;
                    local.insert(*id, Binding::Alias(rhs));
                }
                self.facts(result, &local)
            }
            (0xD1, Payload::One(inner)) => self.facts(inner, env),
            (0xED, Payload::Two(a, b)) => {
                let mut f = self.facts(a, env)?;
                f.extend(self.facts(b, env)?);
                Ok(f)
            }
            (0xEC, Payload::Two(a, b)) => {
                let a = self.facts(a, env)?;
                let b = self.facts(b, env)?;
                Ok(a.intersection(&b).cloned().collect())
            }
            (0x95, Payload::Three(c, a, b)) => match self.guard_value(c, env) {
                Some(true) => self.facts(a, env),
                Some(false) => self.facts(b, env),
                None => {
                    let a = self.facts(a, env)?;
                    let b = self.facts(b, env)?;
                    Ok(a.intersection(&b).cloned().collect())
                }
            },
            (0x94, Payload::Two(a, b)) => {
                let mut f = empty;
                for (candidate, subject) in [(a, b), (b, a)] {
                    if let (Some(Field::Id(b)), Some(Field::Id(BoxRef::SelfBox))) =
                        (self.field(candidate, env), self.field(subject, env))
                    {
                        f.insert(Fact {
                            field: Field::Distinct(b),
                            bytes: vec![],
                        });
                    }
                }
                Ok(f)
            }
            (0x93, Payload::Two(a, b)) => {
                let mut f = empty;
                for (field, value) in [(a, b), (b, a)] {
                    if let (Some(field), Some(bytes)) =
                        (self.field(field, env), self.bytes(value, env))
                    {
                        f.insert(Fact { field, bytes });
                    }
                }
                Ok(f)
            }
            (0xAE, Payload::Two(collection, predicate))
                if matches!(op(resolve(collection, env)?, 0xA4), Some(Payload::Zero)) =>
            {
                let predicate = resolve(predicate, env)?;
                let Some(Payload::FuncValue { args, body }) = op(predicate, 0xD9) else {
                    return Err("unsupported existential predicate".into());
                };
                if args.len() != 1
                    || args[0].1 != Some(SigmaType::SBox)
                    || env.contains_key(&args[0].0)
                {
                    return Err("ambiguous existential scope/type".into());
                }
                let scope = preorder(&self.tree.body)
                    .find(|(_, n)| std::ptr::eq(*n, predicate))
                    .ok_or("unmapped predicate anchor")?
                    .0;
                let mut local = env.clone();
                local.insert(args[0].0, Binding::Box(scope));
                // A fresh scope keeps separate existentials from being merged
                // into a conjunction on a single witness.
                self.facts(body, &local)
            }
            // Unknown, optional/defaulted and negated forms contribute no facts.
            _ => Ok(empty),
        }
    }
}
pub(crate) fn selector_matches(s: &Selector, b: &WireBox) -> Result<bool, String> {
    let c = &b.node().candidate;
    Ok(match s {
        Selector::BoxId { hex } => b.id()? == *hex,
        Selector::PropositionBytes { hex } => hex::encode(c.ergo_tree_bytes()) == *hex,
        Selector::PropositionHash { hex } => {
            hex::encode(ergo_primitives::digest::blake2b256(c.ergo_tree_bytes()).as_bytes()) == *hex
        }
        Selector::TokenAt { index, id, amount } => c
            .tokens
            .get(*index as usize)
            .is_some_and(|t| hex::encode(t.token_id.as_bytes()) == *id && t.amount >= *amount),
        Selector::TokenMember { id, amount } => c
            .tokens
            .iter()
            .any(|t| hex::encode(t.token_id.as_bytes()) == *id && t.amount >= *amount),
        Selector::And { predicates } => {
            !predicates.is_empty()
                && predicates
                    .iter()
                    .map(|s| selector_matches(s, b))
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .all(|x| *x)
        }
    })
}
fn supports(f: &BTreeSet<Fact>, b: &BoxRef, s: &Selector) -> bool {
    match s {
        Selector::BoxId { hex } => hex::decode(hex).ok().is_some_and(|bytes| {
            bytes.len() == 32
                && f.contains(&Fact {
                    field: Field::Id(b.clone()),
                    bytes,
                })
        }),
        Selector::PropositionBytes { hex } => hex::decode(hex).ok().is_some_and(|bytes| {
            !bytes.is_empty()
                && f.contains(&Fact {
                    field: Field::Script(b.clone()),
                    bytes,
                })
        }),
        Selector::PropositionHash { hex } => hex::decode(hex).ok().is_some_and(|bytes| {
            bytes.len() == 32
                && f.contains(&Fact {
                    field: Field::Hash(b.clone()),
                    bytes,
                })
        }),
        Selector::TokenAt { index, id, amount } if *amount == 1 => {
            hex::decode(id).ok().is_some_and(|bytes| {
                bytes.len() == 32
                    && f.contains(&Fact {
                        field: Field::Token(b.clone(), *index),
                        bytes,
                    })
            })
        }
        Selector::And { predicates } => {
            !predicates.is_empty() && predicates.iter().all(|s| supports(f, b, s))
        }
        _ => false,
    }
}
fn field_box(f: &Field) -> &BoxRef {
    match f {
        Field::Id(b)
        | Field::Script(b)
        | Field::Hash(b)
        | Field::Token(b, _)
        | Field::Register(b, _)
        | Field::Distinct(b) => b,
    }
}
/// Fresh import checking uses exact code and P, never serialized status. The
/// complete anchored tree is an untrusted rule application candidate; mandatory
/// facts are reconstructed below with fail-closed rules, not an evaluator.
pub fn check(claim: &RelationProposal, derivation: &Derivation) -> Result<Established, String> {
    if derivation.version != "necessity-derivation:v1"
        || derivation.claim_digest != claim.claim_digest()
    {
        return Err("version or claim binding mismatch".into());
    }
    if !derivation.dependencies.is_empty() || !claim.dependency_ids.is_empty() {
        return Err("dependencies/cycles cannot establish direct premises".into());
    }
    let p = &claim.premises;
    if p.node_revision.value().map(String::as_str) != Some(engine_revision())
        || p.compiler_revision.value().map(String::as_str) != Some(engine_revision())
    {
        return Err("unpinned revision".into());
    }
    if p.network.value().map(String::as_str) != Some("hypothetical-mainnet-activation-3")
        || p.activation_rules.value()
            != Some(
                &json!({"activatedScriptVersion":3,"rootEvaluation":"required-before-storage-rent"}),
            )
    {
        return Err("unsupported activation/network domain".into());
    }
    if p.provenance.value().is_none() {
        return Err("missing provenance".into());
    }
    let domain: StateDomain = serde_json::from_value(
        p.state_constraints
            .value()
            .ok_or("missing state domain")?
            .clone(),
    )
    .map_err(|e| format!("unsupported state constraints: {e}"))?;
    domain.network_rules.node()?;
    domain.block_context.node()?;
    let subject = WireBox::from_record(
        p.self_box
            .value()
            .ok_or("unresolved SELF reference")?
            .clone(),
    )?;
    let bytes = hex::decode(p.root_bytes.value().ok_or("missing exact root")?)
        .map_err(|e| e.to_string())?;
    if subject.node().candidate.ergo_tree_bytes() != bytes {
        return Err("root/SELF mismatch".into());
    }
    match &claim.subject {
        Subject::BoxId { hex } if *hex == subject.id()? => {}
        Subject::Script { hex } if *hex == hex::encode(&bytes) => {}
        _ => return Err("Matches(P,A,a) subject binding mismatch".into()),
    }
    let height = domain.block_context.height;
    let creation = subject.node().candidate.creation_height;
    if !domain.positive_input_tokens
        || domain.block_context.activated_script_version != 3
        || height < creation
        || height - creation >= domain.parameters.storage_period
    {
        return Err("root success not guaranteed: storage-rent or activation domain".into());
    }
    if !valid_guard(&p.guard, 0) {
        return Err("unsupported normalized guard".into());
    }
    let cap = usize::try_from(*p.analysis_caps.get("nodes").ok_or("missing node cap")?)
        .map_err(|e| e.to_string())?;
    if p.analysis_caps.len() != 1 || cap == 0 || cap > 10000 || bytes.len() > 65536 {
        return Err("unsupported analysis cap".into());
    }
    let tree = crate::inspect::parse_tree(&bytes).map_err(|e| e.to_string())?;
    if tree.version > 3 {
        return Err("unsupported tree version".into());
    }
    let anchors: Vec<_> = preorder(&tree.body)
        .map(|(node, e)| Anchor {
            node,
            opcode: node_opcode(e),
            rule: checked_rule(e).into(),
        })
        .collect();
    if anchors.len() > cap || derivation.anchors != anchors {
        return Err("missing, altered, reordered or unsupported exact anchor".into());
    }
    let mut walk = Walk {
        tree: &tree,
        guard: &p.guard,
        visits: 0,
        cap,
    };
    let mut facts = walk.facts(&tree.body, &Env::new())?;
    // One fixed canonical register, authenticated by a mandatory exact data
    // input ID equality. Register contents are state, never script invariants.
    let mut fixed = BTreeMap::new();
    for root in &p.authentication_roots {
        let b = WireBox::from_record(root.clone())?;
        if fixed.insert(b.id()?, b).is_some() {
            return Err("duplicate authentication root".into());
        }
    }
    // Revisit mandatory equalities using the same lexical traversal, replacing
    // only authenticated fixed-register constants through a dedicated pass.
    let literal_facts = facts.clone();
    resolve_register_equalities(
        &mut walk,
        &tree.body,
        &Env::new(),
        &fixed,
        &literal_facts,
        &mut facts,
    )?;
    let rule = match &claim.target {
        Relation::Spend { selector } => {
            if !canonical_selector(selector, 0) {
                return Err(
                    "selector requires bounded canonical lowercase hex and positive amounts".into(),
                );
            }
            let self_matches = selector_matches(selector, &subject)?;
            let candidates: BTreeSet<_> =
                facts.iter().map(|f| field_box(&f.field).clone()).collect();
            if !candidates.iter().any(|b| {
                matches!(b, BoxRef::Input(_) | BoxRef::Bound(_))
                    && supports(&facts, b, selector)
                    && (!self_matches
                        || facts.contains(&Fact {
                            field: Field::Distinct(b.clone()),
                            bytes: vec![],
                        }))
            }) {
                return Err("no mandatory distinct spending identity for target".into());
            }
            "mandatory-positive-identity; explicit-distinctness-or-fixed-SELF-exclusion"
        }
        Relation::Authenticates {
            field,
            expected: Literal::Bytes(expected),
        } => {
            let bytes = hex::decode(expected).map_err(|e| e.to_string())?;
            if !facts
                .iter()
                .filter(|f| !matches!(f.field, Field::Distinct(_)))
                .any(|f| format_field(&f.field) == *field && f.bytes == bytes)
            {
                return Err("field is not mandatorily authenticated to exact bytes".into());
            }
            "mandatory-positive-equality"
        }
        _ => return Err("unsupported relation; extension execution belongs to M05".into()),
    };
    Ok(Established {
        claim: claim.clone(),
        derivation: derivation.clone(),
        rule: rule.into(),
    })
}
fn format_field(f: &Field) -> String {
    let b = match field_box(f) {
        BoxRef::Input(i) => format!("inputs/{i}"),
        BoxRef::Data(i) => format!("data-inputs/{i}"),
        BoxRef::SelfBox => "self".into(),
        BoxRef::Bound(i) => format!("exists/{i}"),
    };
    let name = match f {
        Field::Id(_) => "id".into(),
        Field::Script(_) => "proposition-bytes".into(),
        Field::Hash(_) => "proposition-hash".into(),
        Field::Token(_, i) => format!("tokens/{i}/id"),
        Field::Register(_, i) => format!("R{i}"),
        Field::Distinct(_) => "distinct-from-self".into(),
    };
    format!("{b}/{name}")
}
fn resolve_register_equalities<'a>(
    walk: &mut Walk<'a>,
    e: &'a Expr,
    env: &Env<'a>,
    fixed: &BTreeMap<String, WireBox>,
    literal_facts: &BTreeSet<Fact>,
    facts: &mut BTreeSet<Fact>,
) -> Result<(), String> {
    walk.visits += 1;
    if walk.visits > walk.cap {
        return Err("analysis traversal cap".into());
    }
    let e = resolve(e, env)?;
    if let Some(Payload::BlockValue { items, result }) = op(e, 0xD8) {
        let mut local = env.clone();
        for item in items {
            if let Some(Payload::ValDef { id, rhs, .. }) = op(item, 0xD6) {
                local.insert(*id, Binding::Alias(rhs));
            } else {
                return Err("unsupported binding".into());
            }
        }
        return resolve_register_equalities(walk, result, &local, fixed, literal_facts, facts);
    }
    if let Some(Payload::One(inner)) = op(e, 0xD1) {
        return resolve_register_equalities(walk, inner, env, fixed, literal_facts, facts);
    }
    if let Some(Payload::Two(a, b)) = op(e, 0xED) {
        resolve_register_equalities(walk, a, env, fixed, literal_facts, facts)?;
        return resolve_register_equalities(walk, b, env, fixed, literal_facts, facts);
    }
    if let Some(Payload::Two(a, b)) = op(e, 0x93) {
        for (target, value) in [(a, b), (b, a)] {
            if let (Some(field), Some(Field::Register(b @ BoxRef::Data(_), reg))) =
                (walk.field(target, env), walk.field(value, env))
            {
                let id = literal_facts
                    .iter()
                    .find(|f| f.field == Field::Id(b.clone()))
                    .map(|f| hex::encode(&f.bytes));
                if let Some(config) = id.and_then(|id| fixed.get(&id)) {
                    let register = config
                        .node()
                        .candidate
                        .additional_registers
                        .registers
                        .get((reg - 4) as usize)
                        .map(|r| (&r.tpe, &r.value));
                    if let Some((SigmaType::SColl(t), SigmaValue::Coll(CollValue::Bytes(bytes)))) =
                        register
                    {
                        if **t == SigmaType::SByte {
                            facts.insert(Fact {
                                field,
                                bytes: bytes.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn valid_guard(g: &Guard, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    match g {
        Guard::True | Guard::HeightAtLeast { value: 0.. } => true,
        Guard::Equals {
            field: GuardField::Context { name },
            literal: Literal::Int(_),
        } => name == "HEIGHT",
        Guard::Equals {
            field: GuardField::SelfBox { name },
            literal: Literal::Long(_),
        } => name == "value",
        Guard::And { guards } | Guard::Or { guards } => {
            !guards.is_empty()
                && guards.len() <= 32
                && guards.iter().all(|g| valid_guard(g, depth + 1))
        }
        Guard::Not { guard } => valid_guard(guard, depth + 1),
        _ => false,
    }
}

fn checked_rule(e: &Expr) -> &'static str {
    match e {
        Expr::Op(n) => match (n.opcode, &n.payload) {
            (0xD8, Payload::BlockValue { .. }) => "scoped-bindings",
            (0xD1, Payload::One(_)) => "required-boolean-root",
            (0xED, Payload::Two(..)) => "conjunction",
            (0xEC, Payload::Two(..)) => "common-alternatives",
            (0x95, Payload::Three(..)) => "guarded-alternatives",
            (0x93, Payload::Two(..)) => "positive-equality",
            (0x94, Payload::Two(..)) => "distinct-identity",
            (0xAE, Payload::Two(..)) => "input-existential",
            _ => "operand-or-no-inference",
        },
        _ => "operand-or-no-inference",
    }
}

pub(crate) fn canonical_selector(s: &Selector, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    let canonical = |text: &str, size: Option<usize>| {
        hex::decode(text).ok().is_some_and(|bytes| {
            !bytes.is_empty() && size.is_none_or(|n| bytes.len() == n) && hex::encode(bytes) == text
        })
    };
    match s {
        Selector::BoxId { hex } | Selector::PropositionHash { hex } => canonical(hex, Some(32)),
        Selector::PropositionBytes { hex } => canonical(hex, None),
        Selector::TokenAt { id, amount, .. } | Selector::TokenMember { id, amount } => {
            *amount > 0 && canonical(id, Some(32))
        }
        Selector::And { predicates } => {
            !predicates.is_empty()
                && predicates.len() <= 32
                && predicates.iter().all(|s| canonical_selector(s, depth + 1))
        }
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;
    use ergo_ser::opcode::IrNode;

    fn node(opcode: u8, payload: Payload) -> Expr {
        Expr::Op(IrNode { opcode, payload })
    }
    fn used(id: u32) -> Expr {
        node(0x72, Payload::ValUse { id })
    }
    fn definition(id: u32, rhs: Expr) -> Expr {
        node(
            0xD6,
            Payload::ValDef {
                id,
                tpe: None,
                rhs: Box::new(rhs),
            },
        )
    }
    fn lambda(body: Expr) -> Expr {
        node(
            0xD9,
            Payload::FuncValue {
                args: vec![(2, Some(SigmaType::SBox))],
                body: Box::new(body),
            },
        )
    }
    fn block(items: Vec<Expr>, result: Expr) -> Expr {
        node(
            0xD8,
            Payload::BlockValue {
                items,
                result: Box::new(result),
            },
        )
    }
    fn facts(body: Expr) -> Result<BTreeSet<Fact>, String> {
        let tree = ErgoTree {
            version: 3,
            has_size: true,
            constant_segregation: false,
            constants: vec![],
            body,
        };
        let mut walk = Walk {
            tree: &tree,
            guard: &Guard::True,
            visits: 0,
            cap: 10000,
        };
        walk.facts(&tree.body, &Env::new())
    }

    #[test]
    fn existential_forward_capture_cannot_establish_mandatory_facts() {
        let bytes = Expr::Const {
            tpe: SigmaType::SColl(Box::new(SigmaType::SByte)),
            val: SigmaValue::Coll(CollValue::Bytes(vec![7; 32])),
        };
        let equality = node(
            0x93,
            Payload::Two(
                Box::new(node(0xC5, Payload::One(Box::new(used(2))))),
                Box::new(used(1)),
            ),
        );
        let predicate = definition(3, lambda(equality));
        let capture = definition(1, bytes);
        let exists = node(
            0xAE,
            Payload::Two(Box::new(node(0xA4, Payload::Zero)), Box::new(used(3))),
        );
        let valid = facts(block(
            vec![capture.clone(), predicate.clone()],
            exists.clone(),
        ))
        .unwrap();
        assert_eq!(
            valid.len(),
            1,
            "earlier capture and parameter must still establish a fact"
        );
        assert_eq!(
            facts(block(vec![predicate, capture], exists)).unwrap_err(),
            "forward alias"
        );
    }

    #[test]
    fn nested_lambda_bindings_do_not_leak_or_repair_forward_captures() {
        let local = lambda(block(vec![definition(4, used(2))], used(4)));
        assert!(facts(block(
            vec![definition(3, local.clone())],
            Expr::Const {
                tpe: SigmaType::SBoolean,
                val: SigmaValue::Boolean(true)
            }
        ))
        .is_ok());
        let forward = lambda(block(
            vec![definition(4, used(5)), definition(5, used(2))],
            used(4),
        ));
        for body in [
            forward,
            lambda(block(vec![definition(6, local)], used(4))),
            lambda(lambda(used(2))),
        ] {
            assert!(facts(block(vec![definition(3, body)], used(3))).is_err());
        }
    }
}
