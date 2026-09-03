//! The composer: a contract as a list of **spending paths**, each "who may
//! spend" (a key, any/all/k-of-n keys, or anyone) plus "under what
//! conditions". The paths are OR-ed; a path's conditions are AND-ed.
//!
//! The conditions cover what a script can actually see: the height and the
//! block timestamp, how long the box has sat, how many inputs and outputs
//! the transaction has, any box in it (this one, an output, an input, a
//! data input — one, any, or all) with its script, value, tokens and
//! registers, values the spender attaches, a hash preimage, a token the
//! spender must hold, the miner, and totals across outputs.
//!
//! Assembly is source-to-source: the output is readable ErgoScript with
//! `$name` parameters and `// $name: Type` hints, compiled through the
//! same path as anything typed by hand, decompiled back, hunted, tested.
//!
//! With parameter values, the composer also emits a test suite. Its
//! expected verdicts come from the composer's OWN model of the rules (which
//! paths a scenario satisfies), not from the evaluator — so running the
//! suite checks that the assembled ErgoScript means what the spec says.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::compile::ParamNeed;
use crate::scenario::TypedValue;
use crate::testsuite::Suite;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Spec {
    pub paths: Vec<Path>,
    /// Values the spender will attach (context variables), keyed by var id,
    /// used only for the generated checks — e.g. the secret behind a
    /// `hashPreimage` condition. Never part of the contract.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub witness: BTreeMap<String, TypedValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Path {
    pub name: String,
    pub who: Who,
    #[serde(default)]
    pub conditions: Vec<Condition>,
}

/// Who may take this path. Keys are parameter names.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Who {
    AnyOne {
        #[serde(rename = "anyOne")]
        any_one: bool,
    },
    AnyOf {
        #[serde(rename = "anyOf")]
        any_of: Vec<String>,
    },
    AllOf {
        #[serde(rename = "allOf")]
        all_of: Vec<String>,
    },
    KOf {
        #[serde(rename = "kOf")]
        k_of: usize,
        keys: Vec<String>,
    },
}

/// A condition on the spending transaction. Names are parameter names.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Condition {
    /// `HEIGHT >= $name`
    After(String),
    /// `HEIGHT < $name`
    Before(String),
    /// `CONTEXT.preHeader.timestamp >= $name` (Unix milliseconds).
    AfterTime(String),
    /// `CONTEXT.preHeader.timestamp < $name` (Unix milliseconds).
    BeforeTime(String),
    /// `HEIGHT - SELF.creationInfo._1 >= $name`: the box has sat here for
    /// at least this many blocks.
    BoxAge(String),
    /// `INPUTS.size == $name`
    InputCount(String),
    /// `OUTPUTS.size == $name`
    OutputCount(String),
    /// An output paying at least `$amount` to `$key`.
    PayTo { key: String, amount: String },
    /// An output keeping at least `$at_least` under this same contract.
    KeepHere {
        #[serde(rename = "atLeast")]
        at_least: String,
    },
    /// Data input 0 carries `$nft` and reports R4 >= `$floor`.
    OracleAbove { nft: String, floor: String },
    /// A rule on one box of the transaction: this one, an output, an input
    /// or a data input — by index, any, or all.
    Box(BoxRule),
    /// The spender attaches variable `index` equal to `$value`.
    VarEquals {
        index: u8,
        #[serde(rename = "type")]
        r#type: String,
        value: String,
    },
    /// The spender attaches variable `var` whose hash is `$hash`.
    HashPreimage {
        var: u8,
        hash: String,
        #[serde(default = "default_algo")]
        algo: String,
    },
    /// Some input other than this box carries the token `$token_id` — a
    /// membership token the spender must bring.
    TokenGated {
        #[serde(rename = "tokenId")]
        token_id: String,
    },
    /// `CONTEXT.minerPubKey == $name`
    MinerIs(String),
    /// The outputs to `$key` add up to at least `$at_least`.
    SumPaidTo {
        key: String,
        #[serde(rename = "atLeast")]
        at_least: String,
    },
}

fn default_algo() -> String {
    "blake2b256".into()
}

/// Which box a [`BoxRule`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Which {
    #[serde(rename = "self")]
    SelfBox,
    Output,
    Input,
    DataInput,
}

/// Index within the box list: a number, `"any"` (some box), `"all"`
/// (every box). Omitted on an output means the next free slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Index {
    At(usize),
    Word(Word),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Word {
    Any,
    All,
}

/// The script a box must carry: `"self"` (this same contract) or a key.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ScriptRef {
    SelfScript(SelfWord),
    Key { key: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelfWord {
    #[serde(rename = "self")]
    SelfScript,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenReq {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_least: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    /// `value >= SELF.value * $percent / 100`
    pub percent: String,
}

/// How a register must compare: to a parameter (`eq`, `ne`, `gte`, `lte`),
/// to the current height (`eqHeight`, Int), or to SELF's same register
/// (`eqSelf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RegOp {
    Eq,
    Ne,
    Gte,
    Lte,
    EqHeight,
    EqSelf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegRule {
    /// `R4`..`R9`
    pub reg: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub op: RegOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxRule {
    pub which: Which,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Index>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<ScriptRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_at_least: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_at_least_share: Option<Share>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<TokenReq>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_tokens: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keeps_self_tokens: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registers: Vec<RegRule>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composed {
    pub source: String,
    pub params: Vec<ParamNeed>,
    /// Generated when values were given: one case per path satisfied, one
    /// per condition violated, and a baseline.
    pub suite: Option<Suite>,
}

#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    #[error("a contract needs at least one spending path")]
    NoPaths,
    #[error("path `{0}`: anyone may spend with no conditions — that is a burn of the opposite kind; add a condition or a key")]
    AnyoneUnconditional(String),
    #[error("path `{0}`: no keys named")]
    NoKeys(String),
    #[error("path `{path}`: k-of-n needs 1 <= k <= n, got k={k} for {n} keys")]
    BadThreshold { path: String, k: usize, n: usize },
    #[error("path `{0}`: a box rule with nothing required")]
    EmptyBoxRule(String),
    #[error("path `{0}`: {1}")]
    BadRule(String, String),
    #[error(
        "path `{0}`: its conditions contradict each other ({1}); no transaction can take this path"
    )]
    Unsatisfiable(String, String),
    #[error("parameter `{0}`: {1}")]
    Value(String, String),
}

// ── source ─────────────────────────────────────────────────────────────────

fn who_source(w: &Who) -> Option<String> {
    match w {
        Who::AnyOne { .. } => None,
        Who::AnyOf { any_of } => Some(join(any_of, " || ")),
        Who::AllOf { all_of } => Some(join(all_of, " && ")),
        Who::KOf { k_of, keys } => Some(format!(
            "atLeast({k_of}, Coll({}))",
            keys.iter()
                .map(|k| format!("${k}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn join(keys: &[String], op: &str) -> String {
    let parts: Vec<String> = keys.iter().map(|k| format!("${k}")).collect();
    if parts.len() == 1 {
        parts[0].clone()
    } else {
        format!("({})", parts.join(op))
    }
}

/// The lowered form of every condition: a box rule or a scalar.
fn lower(c: &Condition) -> Condition {
    match c {
        Condition::PayTo { key, amount } => Condition::Box(BoxRule {
            which: Which::Output,
            index: None,
            script: Some(ScriptRef::Key { key: key.clone() }),
            value_at_least: Some(amount.clone()),
            value_at_least_share: None,
            token: None,
            no_tokens: false,
            keeps_self_tokens: false,
            registers: vec![],
        }),
        Condition::KeepHere { at_least } => Condition::Box(BoxRule {
            which: Which::Output,
            index: None,
            script: Some(ScriptRef::SelfScript(SelfWord::SelfScript)),
            value_at_least: Some(at_least.clone()),
            value_at_least_share: None,
            token: None,
            no_tokens: false,
            keeps_self_tokens: false,
            registers: vec![],
        }),
        Condition::OracleAbove { nft, floor } => Condition::Box(BoxRule {
            which: Which::DataInput,
            index: Some(Index::At(0)),
            script: None,
            value_at_least: None,
            value_at_least_share: None,
            token: Some(TokenReq {
                id: nft.clone(),
                at_least: None,
            }),
            no_tokens: false,
            keeps_self_tokens: false,
            registers: vec![RegRule {
                reg: "R4".into(),
                r#type: "Long".into(),
                op: RegOp::Gte,
                value: Some(floor.clone()),
            }],
        }),
        other => other.clone(),
    }
}

/// A box rule with its slot resolved (outputs without an index take the
/// next free slot, in clause order).
fn resolve_slots(conds: &[Condition]) -> Vec<Condition> {
    let mut slot = 0usize;
    conds
        .iter()
        .map(lower)
        .map(|c| match c {
            Condition::Box(mut r) => {
                match (r.which, r.index) {
                    (Which::SelfBox, _) => r.index = None,
                    (Which::Output, None) => {
                        r.index = Some(Index::At(slot));
                        slot += 1;
                    }
                    (Which::Output, Some(Index::At(i))) => slot = slot.max(i + 1),
                    (Which::Input, None) => r.index = Some(Index::Word(Word::Any)),
                    (Which::DataInput, None) => r.index = Some(Index::At(0)),
                    _ => {}
                }
                Condition::Box(r)
            }
            Condition::SumPaidTo { .. } => c,
            other => other,
        })
        .collect()
}

fn list_expr(which: Which) -> &'static str {
    match which {
        Which::Output => "OUTPUTS",
        Which::Input => "INPUTS",
        Which::DataInput => "CONTEXT.dataInputs",
        Which::SelfBox => "SELF",
    }
}

/// The predicate on a box expression `b`.
fn box_predicate(r: &BoxRule, b: &str) -> Vec<String> {
    let mut parts = Vec::new();
    match &r.script {
        Some(ScriptRef::SelfScript(_)) => {
            parts.push(format!("{b}.propositionBytes == SELF.propositionBytes"))
        }
        Some(ScriptRef::Key { key }) => {
            parts.push(format!("{b}.propositionBytes == ${key}.propBytes"))
        }
        None => {}
    }
    if let Some(v) = &r.value_at_least {
        parts.push(format!("{b}.value >= ${v}"));
    }
    if let Some(s) = &r.value_at_least_share {
        parts.push(format!("{b}.value >= SELF.value * ${} / 100L", s.percent));
    }
    if let Some(t) = &r.token {
        let amt = t
            .at_least
            .as_ref()
            .map(|a| format!(" && t._2 >= ${a}"))
            .unwrap_or_default();
        parts.push(format!(
            "{b}.tokens.exists {{ (t: (Coll[Byte], Long)) => t._1 == ${}{amt} }}",
            t.id
        ));
    }
    if r.no_tokens {
        parts.push(format!("{b}.tokens.size == 0"));
    }
    if r.keeps_self_tokens {
        parts.push(format!("{b}.tokens == SELF.tokens"));
    }
    for rr in &r.registers {
        let reg = format!("{b}.{}[{}]", rr.reg, rr.r#type);
        let rhs = match rr.op {
            RegOp::EqHeight => "HEIGHT".to_string(),
            RegOp::EqSelf => format!("SELF.{}[{}].get", rr.reg, rr.r#type),
            _ => format!("${}", rr.value.as_deref().unwrap_or("?")),
        };
        let op = match rr.op {
            RegOp::Ne => "!=",
            RegOp::Gte => ">=",
            RegOp::Lte => "<=",
            _ => "==",
        };
        if rr.op == RegOp::EqSelf {
            parts.push(format!(
                "SELF.{}[{}].isDefined && {reg}.isDefined && {reg}.get {op} {rhs}",
                rr.reg, rr.r#type
            ));
        } else {
            parts.push(format!("{reg}.isDefined && {reg}.get {op} {rhs}"));
        }
    }
    parts
}

fn box_rule_source(r: &BoxRule) -> String {
    let list = list_expr(r.which);
    match (r.which, r.index) {
        (Which::SelfBox, _) => format!("({})", box_predicate(r, "SELF").join(" && ")),
        (_, Some(Index::At(i))) => {
            let b = format!("{list}({i})");
            format!(
                "({list}.size > {i} && {})",
                box_predicate(r, &b).join(" && ")
            )
        }
        (_, Some(Index::Word(Word::Any))) => format!(
            "{list}.exists {{ (bx: Box) => {} }}",
            box_predicate(r, "bx").join(" && ")
        ),
        (_, Some(Index::Word(Word::All))) => format!(
            "{list}.forall {{ (bx: Box) => {} }}",
            box_predicate(r, "bx").join(" && ")
        ),
        (_, None) => unreachable!("slots resolved"),
    }
}

/// Conditions as a boolean, with output slots allocated in clause order.
fn conditions_source(conds: &[Condition]) -> Option<String> {
    let parts: Vec<String> = resolve_slots(conds)
        .iter()
        .map(|c| match c {
            Condition::After(h) => format!("HEIGHT >= ${h}"),
            Condition::Before(h) => format!("HEIGHT < ${h}"),
            Condition::AfterTime(t) => format!("CONTEXT.preHeader.timestamp >= ${t}"),
            Condition::BeforeTime(t) => format!("CONTEXT.preHeader.timestamp < ${t}"),
            Condition::BoxAge(n) => format!("HEIGHT - SELF.creationInfo._1 >= ${n}"),
            Condition::InputCount(n) => format!("INPUTS.size == ${n}"),
            Condition::OutputCount(n) => format!("OUTPUTS.size == ${n}"),
            Condition::Box(r) => box_rule_source(r),
            Condition::VarEquals { index, r#type, value } => format!(
                "(getVar[{t}]({index}).isDefined && getVar[{t}]({index}).get == ${value})",
                t = r#type
            ),
            Condition::HashPreimage { var, hash, algo } => format!(
                "(getVar[Coll[Byte]]({var}).isDefined && {algo}(getVar[Coll[Byte]]({var}).get) == ${hash})"
            ),
            Condition::TokenGated { token_id } => format!(
                "INPUTS.exists {{ (bx: Box) => bx.id != SELF.id && bx.tokens.exists {{ (t: (Coll[Byte], Long)) => t._1 == ${token_id} }} }}"
            ),
            Condition::MinerIs(m) => format!("CONTEXT.minerPubKey == ${m}"),
            Condition::SumPaidTo { key, at_least } => format!(
                "OUTPUTS.fold(0L, {{ (acc: Long, bx: Box) => if (bx.propositionBytes == ${key}.propBytes) acc + bx.value else acc }}) >= ${at_least}"
            ),
            Condition::PayTo { .. } | Condition::KeepHere { .. } | Condition::OracleAbove { .. } => {
                unreachable!("lowered")
            }
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" &&\n              "))
    }
}

// ── parameters ─────────────────────────────────────────────────────────────

const NANO: &str = "in nanoERG — 1 ERG = 1,000,000,000 nanoERG.";

/// Parameters the spec needs, in first-use order, with types.
fn params_of(spec: &Spec) -> Vec<ParamNeed> {
    let mut out: Vec<ParamNeed> = Vec::new();
    let mut push = |name: &str, tpe: &str, desc: String| {
        if !out.iter().any(|p| p.name == name) {
            out.push(ParamNeed {
                name: name.to_string(),
                type_hint: Some(tpe.to_string()),
                default: None,
                description: Some(desc),
            });
        }
    };
    for p in &spec.paths {
        match &p.who {
            Who::AnyOne { .. } => {}
            Who::AnyOf { any_of: ks } | Who::AllOf { all_of: ks } | Who::KOf { keys: ks, .. } => {
                for k in ks {
                    push(
                        k,
                        "SigmaProp",
                        format!("Address for \"{k}\" — An Ergo address from a wallet."),
                    );
                }
            }
        }
        for c in &p.conditions {
            match c {
                Condition::After(h) => push(h, "Int", format!("From when? (\"{h}\") — Spending on this path is possible from this date.")),
                Condition::Before(h) => push(h, "Int", format!("Until when? (\"{h}\") — Spending on this path stops at this date.")),
                Condition::AfterTime(t) => push(t, "Long", format!("From what time? (\"{t}\") — Unix time in milliseconds, from the block being mined.")),
                Condition::BeforeTime(t) => push(t, "Long", format!("Until what time? (\"{t}\") — Unix time in milliseconds, from the block being mined.")),
                Condition::BoxAge(n) => push(n, "Int", format!("How long must the funds have sat here (\"{n}\")? — In blocks; about 720 a day.")),
                Condition::InputCount(n) => push(n, "Int", format!("How many inputs must the transaction have (\"{n}\")? — Counting this box.")),
                Condition::OutputCount(n) => push(n, "Int", format!("How many outputs must the transaction have (\"{n}\")?")),
                Condition::PayTo { key, amount } => {
                    push(key, "SigmaProp", format!("Who is paid (\"{key}\")? — Their Ergo address."));
                    push(amount, "Long", format!("How much must be paid (\"{amount}\"), {NANO}"));
                }
                Condition::KeepHere { at_least } => push(at_least, "Long", format!("How much must stay in the contract (\"{at_least}\"), {NANO}")),
                Condition::OracleAbove { nft, floor } => {
                    push(nft, "Coll[Byte]", format!("Which oracle (\"{nft}\")? — The token id that identifies the oracle box."));
                    push(floor, "Long", format!("Minimum price (\"{floor}\") — In the oracle's own units."));
                }
                Condition::Box(r) => {
                    if let Some(ScriptRef::Key { key }) = &r.script {
                        push(key, "SigmaProp", format!("Whose box (\"{key}\")? — Their Ergo address."));
                    }
                    if let Some(v) = &r.value_at_least {
                        push(v, "Long", format!("Minimum value (\"{v}\"), {NANO}"));
                    }
                    if let Some(s) = &r.value_at_least_share {
                        push(&s.percent, "Long", format!("Minimum share of this box's value (\"{}\") — A percentage, 0 to 100.", s.percent));
                    }
                    if let Some(t) = &r.token {
                        push(&t.id, "Coll[Byte]", format!("Which token (\"{}\")? — Its token id.", t.id));
                        if let Some(a) = &t.at_least {
                            push(a, "Long", format!("How many of the token (\"{a}\")? — In the token's smallest unit."));
                        }
                    }
                    for rr in &r.registers {
                        if let Some(v) = &rr.value {
                            push(v, &rr.r#type, format!("Register {} value (\"{v}\") — A {}.", rr.reg, rr.r#type));
                        }
                    }
                }
                Condition::VarEquals { r#type, value, index } => push(value, r#type, format!("Value the spender must attach as variable {index} (\"{value}\") — A {type}.", type = r#type)),
                Condition::HashPreimage { hash, algo, .. } => push(hash, "Coll[Byte]", format!("The {algo} hash of the secret (\"{hash}\") — 32 bytes, hex; the secret itself never goes on chain until it is spent.")),
                Condition::TokenGated { token_id } => push(token_id, "Coll[Byte]", format!("Which token must the spender hold (\"{token_id}\")? — Its token id.")),
                Condition::MinerIs(m) => push(m, "Coll[Byte]", format!("Which miner (\"{m}\")? — The miner's public key, 33 bytes hex.")),
                Condition::SumPaidTo { key, at_least } => {
                    push(key, "SigmaProp", format!("Who is paid (\"{key}\")? — Their Ergo address."));
                    push(at_least, "Long", format!("How much in total across outputs (\"{at_least}\"), {NANO}"));
                }
            }
        }
    }
    out
}

fn numeric(t: &str) -> bool {
    matches!(t, "Byte" | "Short" | "Int" | "Long" | "BigInt")
}

fn validate(spec: &Spec) -> Result<(), ComposeError> {
    if spec.paths.is_empty() {
        return Err(ComposeError::NoPaths);
    }
    for p in &spec.paths {
        match &p.who {
            Who::AnyOne { .. } if p.conditions.is_empty() => {
                return Err(ComposeError::AnyoneUnconditional(p.name.clone()))
            }
            Who::AnyOf { any_of: ks } | Who::AllOf { all_of: ks } if ks.is_empty() => {
                return Err(ComposeError::NoKeys(p.name.clone()))
            }
            Who::KOf { k_of, keys } if *k_of == 0 || *k_of > keys.len() || keys.is_empty() => {
                return Err(ComposeError::BadThreshold {
                    path: p.name.clone(),
                    k: *k_of,
                    n: keys.len(),
                })
            }
            _ => {}
        }
        let bad = |m: String| ComposeError::BadRule(p.name.clone(), m);
        for c in &p.conditions {
            match c {
                Condition::Box(r) => {
                    if r.script.is_none()
                        && r.value_at_least.is_none()
                        && r.value_at_least_share.is_none()
                        && r.token.is_none()
                        && !r.no_tokens
                        && !r.keeps_self_tokens
                        && r.registers.is_empty()
                    {
                        return Err(ComposeError::EmptyBoxRule(p.name.clone()));
                    }
                    if r.which == Which::SelfBox && r.script.is_some() {
                        return Err(bad("this box's own script is fixed; a script rule on `self` is meaningless".into()));
                    }
                    if r.which == Which::SelfBox && r.keeps_self_tokens {
                        return Err(bad(
                            "`keepsSelfTokens` on this box itself is always true".into()
                        ));
                    }
                    if r.no_tokens && (r.token.is_some() || r.keeps_self_tokens) {
                        return Err(bad(
                            "`noTokens` contradicts a token requirement on the same box".into(),
                        ));
                    }
                    for rr in &r.registers {
                        if !matches!(rr.reg.as_str(), "R4" | "R5" | "R6" | "R7" | "R8" | "R9") {
                            return Err(bad(format!("register `{}` — use R4..R9", rr.reg)));
                        }
                        match rr.op {
                            RegOp::EqHeight if rr.r#type != "Int" => {
                                return Err(bad(format!(
                                    "{}: HEIGHT is an Int, so an `eqHeight` register must be Int",
                                    rr.reg
                                )))
                            }
                            RegOp::EqSelf if r.which == Which::SelfBox => {
                                return Err(bad(format!(
                                    "{}: `eqSelf` on this box itself is always true",
                                    rr.reg
                                )))
                            }
                            RegOp::Gte | RegOp::Lte if !numeric(&rr.r#type) => {
                                return Err(bad(format!(
                                    "{}: only numbers compare with >= or <=",
                                    rr.reg
                                )))
                            }
                            RegOp::Eq | RegOp::Ne | RegOp::Gte | RegOp::Lte
                                if rr.value.is_none() =>
                            {
                                return Err(bad(format!(
                                    "{}: this comparison needs a value name",
                                    rr.reg
                                )))
                            }
                            _ => {}
                        }
                    }
                }
                Condition::HashPreimage { algo, .. }
                    if !matches!(algo.as_str(), "blake2b256" | "sha256") =>
                {
                    return Err(bad(format!("hash `{algo}` — use blake2b256 or sha256")))
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Assemble the source (and, with values, the suite).
pub fn compose(
    spec: &Spec,
    values: &BTreeMap<String, TypedValue>,
) -> Result<Composed, ComposeError> {
    validate(spec)?;
    let params = params_of(spec);
    let mut src = String::new();
    src.push_str(&format!(
        "// Composed with ergo-forge: {} spending path{}. Paths are OR-ed; a path's\n// conditions are AND-ed.\n",
        spec.paths.len(),
        if spec.paths.len() == 1 { "" } else { "s" }
    ));
    for p in &params {
        src.push_str(&format!(
            "// ${}: {}\n",
            p.name,
            p.type_hint.as_deref().unwrap_or("Long")
        ));
    }
    src.push_str("{\n");
    let mut names = Vec::new();
    for (i, p) in spec.paths.iter().enumerate() {
        let var = format!("path{}", i + 1);
        let who = who_source(&p.who);
        let conds = conditions_source(&p.conditions);
        let expr = match (who, conds) {
            (Some(w), Some(c)) => format!("{w} &&\n    sigmaProp({c})"),
            (Some(w), None) => w,
            (None, Some(c)) => format!("sigmaProp({c})"),
            (None, None) => unreachable!("validated"),
        };
        src.push_str(&format!("  // {}: {}\n  val {var} = {expr}\n", var, p.name));
        names.push(var);
    }
    src.push_str(&format!("  {}\n}}\n", names.join(" || ")));

    let suite = if values.is_empty() {
        None
    } else {
        Some(generate_suite(spec, &src, values)?)
    };
    Ok(Composed {
        source: src,
        params,
        suite,
    })
}

/// Hex digest of `data` under `algo` (`blake2b256` or `sha256`).
pub fn hash_hex(algo: &str, data: &[u8]) -> Option<String> {
    use sha2::Digest as _;
    match algo {
        "blake2b256" => Some(hex::encode(
            ergo_primitives::digest::blake2b256(data).as_bytes(),
        )),
        "sha256" => Some(hex::encode(sha2::Sha256::digest(data))),
        _ => None,
    }
}

// ── the model, and the generated suite ─────────────────────────────────────

/// A register's value in the model: a literal, or something resolved
/// against the world when the scenario is written out.
#[derive(Clone, Debug, PartialEq)]
enum RegVal {
    Lit(serde_json::Value),
    Height,
}

#[derive(Clone, Debug)]
struct MBox {
    /// Tree hex, or "$self".
    tree: String,
    value: i64,
    tokens: Vec<(String, u64)>,
    /// reg → (type, value)
    registers: BTreeMap<String, (String, RegVal)>,
    creation_height: i64,
}

impl Default for MBox {
    fn default() -> Self {
        MBox {
            tree: ANY_TREE.into(),
            value: 1,
            tokens: vec![],
            registers: BTreeMap::new(),
            creation_height: 0,
        }
    }
}

/// Any valid tree does for a box whose script is not the point.
const ANY_TREE: &str = "10010101d17300";
/// The token this box carries in the model when a rule refers to its tokens.
const SELF_TOKEN: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

/// A concrete scenario the model reasons about.
#[derive(Clone, Debug)]
struct World {
    height: i64,
    timestamp: Option<u64>,
    self_box: MBox,
    /// Inputs after SELF (INPUTS = SELF ++ extra).
    extra_inputs: Vec<MBox>,
    outputs: Vec<MBox>,
    data_inputs: Vec<MBox>,
    vars: BTreeMap<u8, TypedValue>,
    miner: Option<String>,
}

impl World {
    fn baseline(spec: &Spec) -> World {
        let uses_self_tokens = spec
            .paths
            .iter()
            .flat_map(|p| &p.conditions)
            .any(|c| matches!(c, Condition::Box(r) if r.keeps_self_tokens));
        World {
            height: 1,
            timestamp: None,
            self_box: MBox {
                tree: "$self".into(),
                value: 1_000_000_000,
                tokens: if uses_self_tokens {
                    vec![(SELF_TOKEN.into(), 7)]
                } else {
                    vec![]
                },
                registers: BTreeMap::new(),
                creation_height: 0,
            },
            extra_inputs: vec![],
            outputs: vec![],
            data_inputs: vec![],
            vars: BTreeMap::new(),
            miner: None,
        }
    }

    fn list(&self, which: Which) -> Vec<&MBox> {
        match which {
            Which::SelfBox => vec![&self.self_box],
            Which::Output => self.outputs.iter().collect(),
            Which::DataInput => self.data_inputs.iter().collect(),
            Which::Input => std::iter::once(&self.self_box)
                .chain(self.extra_inputs.iter())
                .collect(),
        }
    }

    /// The box a rule at (which, index) targets, created if absent.
    fn slot(&mut self, which: Which, i: usize) -> &mut MBox {
        let list = match which {
            Which::SelfBox => return &mut self.self_box,
            Which::Output => &mut self.outputs,
            Which::DataInput => &mut self.data_inputs,
            Which::Input => {
                if i == 0 {
                    return &mut self.self_box;
                }
                let j = i - 1;
                while self.extra_inputs.len() <= j {
                    self.extra_inputs.push(MBox::default());
                }
                return &mut self.extra_inputs[j];
            }
        };
        while list.len() <= i {
            list.push(MBox::default());
        }
        &mut list[i]
    }
}

fn key_tree(values: &BTreeMap<String, TypedValue>, name: &str) -> Result<String, ComposeError> {
    let tv = values
        .get(name)
        .ok_or_else(|| ComposeError::Value(name.into(), "no value given".into()))?;
    let (_, v) = crate::scenario::parse_typed_value(&tv.r#type, &tv.value)
        .map_err(|e| ComposeError::Value(name.into(), e.to_string()))?;
    match v {
        ergo_ser::sigma_value::SigmaValue::SigmaProp(
            ergo_ser::sigma_value::SigmaBoolean::ProveDlog(pk),
        ) => Ok(format!("0008cd{}", hex::encode(pk.as_bytes()))),
        _ => Err(ComposeError::Value(
            name.into(),
            "must be a key (address or pubkey)".into(),
        )),
    }
}

fn key_hex(values: &BTreeMap<String, TypedValue>, name: &str) -> Result<String, ComposeError> {
    Ok(key_tree(values, name)?[6..].to_string())
}

fn int_of(values: &BTreeMap<String, TypedValue>, name: &str) -> Result<i64, ComposeError> {
    let tv = values
        .get(name)
        .ok_or_else(|| ComposeError::Value(name.into(), "no value given".into()))?;
    tv.value
        .as_i64()
        .or_else(|| tv.value.as_str().and_then(|s| s.parse().ok()))
        .ok_or_else(|| ComposeError::Value(name.into(), "must be a whole number".into()))
}

fn str_of(values: &BTreeMap<String, TypedValue>, name: &str) -> Result<String, ComposeError> {
    values
        .get(name)
        .and_then(|tv| tv.value.as_str().map(|s| s.to_lowercase()))
        .ok_or_else(|| ComposeError::Value(name.into(), "no value given".into()))
}

fn val_of(
    values: &BTreeMap<String, TypedValue>,
    name: &str,
) -> Result<serde_json::Value, ComposeError> {
    values
        .get(name)
        .map(|tv| tv.value.clone())
        .ok_or_else(|| ComposeError::Value(name.into(), "no value given".into()))
}

/// Literal comparison the way the script would see it.
fn json_num(v: &serde_json::Value) -> Option<i128> {
    v.as_i64()
        .map(i128::from)
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}
fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (json_num(a), json_num(b)) {
        (Some(x), Some(y)) => x == y,
        _ => match (a.as_str(), b.as_str()) {
            (Some(x), Some(y)) => x.eq_ignore_ascii_case(y),
            _ => a == b,
        },
    }
}

fn resolve(rv: &RegVal, w: &World) -> serde_json::Value {
    match rv {
        RegVal::Lit(v) => v.clone(),
        RegVal::Height => serde_json::json!(w.height),
    }
}

/// Does box `b` satisfy rule `r` in world `w`?
fn box_ok(
    r: &BoxRule,
    b: &MBox,
    w: &World,
    values: &BTreeMap<String, TypedValue>,
) -> Result<bool, ComposeError> {
    match &r.script {
        Some(ScriptRef::SelfScript(_)) if b.tree != "$self" => return Ok(false),
        Some(ScriptRef::Key { key }) if b.tree != key_tree(values, key)? => return Ok(false),
        _ => {}
    }
    if let Some(v) = &r.value_at_least {
        if b.value < int_of(values, v)? {
            return Ok(false);
        }
    }
    if let Some(s) = &r.value_at_least_share {
        if i128::from(b.value)
            < i128::from(w.self_box.value) * i128::from(int_of(values, &s.percent)?) / 100
        {
            return Ok(false);
        }
    }
    if let Some(t) = &r.token {
        let id = str_of(values, &t.id)?;
        let min = match &t.at_least {
            Some(a) => int_of(values, a)?,
            None => i64::MIN,
        };
        if !b
            .tokens
            .iter()
            .any(|(i, n)| *i == id && i64::try_from(*n).unwrap_or(i64::MAX) >= min)
        {
            return Ok(false);
        }
    }
    if r.no_tokens && !b.tokens.is_empty() {
        return Ok(false);
    }
    if r.keeps_self_tokens && b.tokens != w.self_box.tokens {
        return Ok(false);
    }
    for rr in &r.registers {
        let Some((t, rv)) = b.registers.get(&rr.reg) else {
            return Ok(false);
        };
        if *t != rr.r#type {
            return Ok(false);
        }
        let have = resolve(rv, w);
        let ok = match rr.op {
            RegOp::EqHeight => json_num(&have) == Some(i128::from(w.height)),
            RegOp::EqSelf => match w.self_box.registers.get(&rr.reg) {
                Some((st, sv)) if *st == rr.r#type => json_eq(&have, &resolve(sv, w)),
                _ => false,
            },
            RegOp::Eq => json_eq(&have, &val_of(values, rr.value.as_deref().unwrap_or(""))?),
            RegOp::Ne => !json_eq(&have, &val_of(values, rr.value.as_deref().unwrap_or(""))?),
            RegOp::Gte | RegOp::Lte => {
                let want = json_num(&val_of(values, rr.value.as_deref().unwrap_or(""))?);
                match (json_num(&have), want) {
                    (Some(h), Some(x)) => {
                        if rr.op == RegOp::Gte {
                            h >= x
                        } else {
                            h <= x
                        }
                    }
                    _ => false,
                }
            }
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rule_ok(
    r: &BoxRule,
    w: &World,
    values: &BTreeMap<String, TypedValue>,
) -> Result<bool, ComposeError> {
    let list = w.list(r.which);
    match (r.which, r.index) {
        (Which::SelfBox, _) => box_ok(r, &w.self_box, w, values),
        (_, Some(Index::At(i))) => match list.get(i) {
            Some(b) => box_ok(r, b, w, values),
            None => Ok(false),
        },
        (_, Some(Index::Word(Word::Any))) => {
            for b in list {
                if box_ok(r, b, w, values)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        (_, Some(Index::Word(Word::All))) => {
            for b in list {
                if !box_ok(r, b, w, values)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (_, None) => unreachable!("slots resolved"),
    }
}

fn var_bytes(w: &World, i: u8) -> Option<Vec<u8>> {
    let tv = w.vars.get(&i)?;
    if tv.r#type != "Coll[Byte]" {
        return None;
    }
    hex::decode(tv.value.as_str()?).ok()
}

/// Does this world satisfy the path's conditions (ignoring keys)?
fn satisfied(
    p: &Path,
    w: &World,
    values: &BTreeMap<String, TypedValue>,
) -> Result<bool, ComposeError> {
    for c in &resolve_slots(&p.conditions) {
        let ok = match c {
            Condition::After(h) => w.height >= int_of(values, h)?,
            Condition::Before(h) => w.height < int_of(values, h)?,
            Condition::AfterTime(t) => {
                w.timestamp.unwrap_or(0) as i128 >= int_of(values, t)? as i128
            }
            Condition::BeforeTime(t) => {
                (w.timestamp.unwrap_or(0) as i128) < int_of(values, t)? as i128
            }
            Condition::BoxAge(n) => w.height - w.self_box.creation_height >= int_of(values, n)?,
            Condition::InputCount(n) => (1 + w.extra_inputs.len()) as i64 == int_of(values, n)?,
            Condition::OutputCount(n) => w.outputs.len() as i64 == int_of(values, n)?,
            Condition::Box(r) => rule_ok(r, w, values)?,
            Condition::VarEquals {
                index,
                r#type,
                value,
            } => w
                .vars
                .get(index)
                .map(|tv| {
                    tv.r#type == *r#type
                        && json_eq(&tv.value, &val_of(values, value).unwrap_or_default())
                })
                .unwrap_or(false),
            Condition::HashPreimage { var, hash, algo } => match var_bytes(w, *var) {
                Some(bytes) => {
                    hash_hex(algo, &bytes).as_deref() == Some(str_of(values, hash)?.as_str())
                }
                None => false,
            },
            Condition::TokenGated { token_id } => {
                let id = str_of(values, token_id)?;
                w.extra_inputs
                    .iter()
                    .any(|b| b.tokens.iter().any(|(i, _)| *i == id))
            }
            Condition::MinerIs(m) => w.miner.as_deref() == Some(str_of(values, m)?.as_str()),
            Condition::SumPaidTo { key, at_least } => {
                let tree = key_tree(values, key)?;
                let sum: i128 = w
                    .outputs
                    .iter()
                    .filter(|b| b.tree == tree)
                    .map(|b| i128::from(b.value))
                    .sum();
                sum >= i128::from(int_of(values, at_least)?)
            }
            Condition::PayTo { .. }
            | Condition::KeepHere { .. }
            | Condition::OracleAbove { .. } => unreachable!("lowered"),
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Make box `b` satisfy rule `r` (requirements accumulate; later rules win
/// on conflicts, which the satisfiability check then reports).
fn apply_rule(
    r: &BoxRule,
    b: &mut MBox,
    self_value: i64,
    self_tokens: &[(String, u64)],
    values: &BTreeMap<String, TypedValue>,
) -> Result<(), ComposeError> {
    match &r.script {
        Some(ScriptRef::SelfScript(_)) => b.tree = "$self".into(),
        Some(ScriptRef::Key { key }) => b.tree = key_tree(values, key)?,
        None => {}
    }
    if let Some(v) = &r.value_at_least {
        b.value = b.value.max(int_of(values, v)?);
    }
    if let Some(s) = &r.value_at_least_share {
        let need = (i128::from(self_value) * i128::from(int_of(values, &s.percent)?) + 99) / 100;
        b.value = b.value.max(i64::try_from(need).unwrap_or(i64::MAX));
    }
    if let Some(t) = &r.token {
        let id = str_of(values, &t.id)?;
        let n = match &t.at_least {
            Some(a) => u64::try_from(int_of(values, a)?).unwrap_or(1).max(1),
            None => 1,
        };
        match b.tokens.iter_mut().find(|(i, _)| *i == id) {
            Some(slot) => slot.1 = slot.1.max(n),
            None => b.tokens.push((id, n)),
        }
    }
    if r.no_tokens {
        b.tokens.clear();
    }
    if r.keeps_self_tokens {
        b.tokens = self_tokens.to_vec();
    }
    for rr in &r.registers {
        let rv = match rr.op {
            RegOp::EqHeight => RegVal::Height,
            RegOp::EqSelf => continue, // copied from SELF after all rules ran
            RegOp::Eq | RegOp::Gte | RegOp::Lte => {
                RegVal::Lit(val_of(values, rr.value.as_deref().unwrap_or(""))?)
            }
            RegOp::Ne => {
                let v = val_of(values, rr.value.as_deref().unwrap_or(""))?;
                RegVal::Lit(bump(&rr.r#type, &v))
            }
        };
        b.registers.insert(rr.reg.clone(), (rr.r#type.clone(), rv));
    }
    Ok(())
}

/// A different value of the same type.
fn bump(tpe: &str, v: &serde_json::Value) -> serde_json::Value {
    if let Some(n) = json_num(v) {
        if numeric(tpe) {
            return serde_json::json!((n + 1).to_string());
        }
    }
    if let Some(b) = v.as_bool() {
        return serde_json::json!(!b);
    }
    if let Some(s) = v.as_str() {
        // flip the first hex digit
        let mut cs: Vec<char> = s.chars().collect();
        if let Some(c) = cs.first_mut() {
            *c = if *c == '0' { '1' } else { '0' };
        }
        return serde_json::json!(cs.into_iter().collect::<String>());
    }
    v.clone()
}

/// A world that satisfies every condition of `p`: with this box carrying
/// a token when a rule refers to its tokens, else (a tokenless box is a
/// valid state too) without.
fn satisfying_world(
    spec: &Spec,
    p: &Path,
    values: &BTreeMap<String, TypedValue>,
) -> Result<World, ComposeError> {
    let with = World::baseline(spec);
    if with.self_box.tokens.is_empty() {
        return satisfying_world_from(spec, p, values, with);
    }
    let mut without = with.clone();
    without.self_box.tokens.clear();
    match satisfying_world_from(spec, p, values, with) {
        Err(ComposeError::Unsatisfiable(..)) => satisfying_world_from(spec, p, values, without),
        r => r,
    }
}

fn satisfying_world_from(
    spec: &Spec,
    p: &Path,
    values: &BTreeMap<String, TypedValue>,
    mut w: World,
) -> Result<World, ComposeError> {
    let mut lo: Option<i64> = None;
    let mut hi: Option<i64> = None;
    let mut tlo: Option<i64> = None;
    let mut thi: Option<i64> = None;
    let mut all_rules: Vec<BoxRule> = Vec::new();
    let mut out_count: Option<i64> = None;
    let conds = resolve_slots(&p.conditions);
    for c in &conds {
        match c {
            Condition::After(h) => lo = Some(lo.unwrap_or(i64::MIN).max(int_of(values, h)?)),
            Condition::Before(h) => hi = Some(hi.unwrap_or(i64::MAX).min(int_of(values, h)?)),
            Condition::AfterTime(t) => tlo = Some(tlo.unwrap_or(i64::MIN).max(int_of(values, t)?)),
            Condition::BeforeTime(t) => thi = Some(thi.unwrap_or(i64::MAX).min(int_of(values, t)?)),
            Condition::BoxAge(n) => lo = Some(lo.unwrap_or(i64::MIN).max(int_of(values, n)?)),
            Condition::InputCount(n) => {
                let n = int_of(values, n)?;
                if n < 1 {
                    return Err(ComposeError::Unsatisfiable(
                        p.name.clone(),
                        "a transaction has at least one input, this box".into(),
                    ));
                }
                while (1 + w.extra_inputs.len()) < n as usize {
                    w.extra_inputs.push(MBox::default());
                }
            }
            Condition::OutputCount(n) => out_count = Some(int_of(values, n)?),
            Condition::Box(r) => {
                let self_value = w.self_box.value;
                let self_tokens = w.self_box.tokens.clone();
                match (r.which, r.index) {
                    (Which::SelfBox, _) => {
                        apply_rule(r, &mut w.self_box, self_value, &self_tokens, values)?
                    }
                    (which, Some(Index::At(i))) => {
                        apply_rule(r, w.slot(which, i), self_value, &self_tokens, values)?
                    }
                    (which, Some(Index::Word(Word::Any))) => {
                        let i = w
                            .list(which)
                            .len()
                            .max(if which == Which::Input { 1 } else { 0 });
                        apply_rule(r, w.slot(which, i), self_value, &self_tokens, values)?;
                    }
                    (_, Some(Index::Word(Word::All))) => all_rules.push(r.clone()),
                    (_, None) => unreachable!("slots resolved"),
                }
            }
            Condition::VarEquals {
                index,
                r#type,
                value,
            } => {
                w.vars.insert(
                    *index,
                    TypedValue {
                        r#type: r#type.clone(),
                        value: val_of(values, value)?,
                    },
                );
            }
            Condition::HashPreimage { var, .. } => {
                if let Some(tv) = spec.witness.get(&var.to_string()) {
                    w.vars.insert(*var, tv.clone());
                }
            }
            Condition::TokenGated { token_id } => {
                // The token may sit on any input the spender brings: reuse
                // one when there is one, so a fixed input count still holds.
                let id = str_of(values, token_id)?;
                match w.extra_inputs.first_mut() {
                    Some(b) => b.tokens.push((id, 1)),
                    None => w.extra_inputs.push(MBox {
                        tokens: vec![(id, 1)],
                        ..MBox::default()
                    }),
                }
            }
            Condition::MinerIs(m) => w.miner = Some(str_of(values, m)?),
            Condition::SumPaidTo { key, at_least } => {
                w.outputs.push(MBox {
                    tree: key_tree(values, key)?,
                    value: int_of(values, at_least)?.max(1),
                    ..MBox::default()
                });
            }
            Condition::PayTo { .. }
            | Condition::KeepHere { .. }
            | Condition::OracleAbove { .. } => unreachable!("lowered"),
        }
    }
    if let Some(n) = out_count {
        if (w.outputs.len() as i64) > n {
            return Err(ComposeError::Unsatisfiable(
                p.name.clone(),
                format!(
                    "{} outputs are required but the count is fixed at {n}",
                    w.outputs.len()
                ),
            ));
        }
        while (w.outputs.len() as i64) < n {
            w.outputs.push(MBox::default());
        }
    }
    // "all" rules apply to every box of their list (at least one).
    for r in &all_rules {
        let self_value = w.self_box.value;
        let self_tokens = w.self_box.tokens.clone();
        let n = w.list(r.which).len().max(1);
        for i in 0..n {
            apply_rule(r, w.slot(r.which, i), self_value, &self_tokens, values)?;
        }
    }
    // `eqSelf` registers: copy SELF's register (given one if it has none).
    for c in &conds {
        if let Condition::Box(r) = c {
            for rr in r.registers.iter().filter(|rr| rr.op == RegOp::EqSelf) {
                let sv = w
                    .self_box
                    .registers
                    .entry(rr.reg.clone())
                    .or_insert_with(|| (rr.r#type.clone(), RegVal::Lit(sample(&rr.r#type))))
                    .clone();
                let n = w.list(r.which).len();
                let idxs: Vec<usize> = match r.index {
                    Some(Index::At(i)) => vec![i],
                    Some(Index::Word(Word::Any)) => vec![n.saturating_sub(1)],
                    Some(Index::Word(Word::All)) => (0..n).collect(),
                    None => vec![],
                };
                for i in idxs {
                    w.slot(r.which, i)
                        .registers
                        .insert(rr.reg.clone(), sv.clone());
                }
            }
        }
    }
    w.height = match (lo, hi) {
        (Some(l), Some(h)) => l.max(1).min(h - 1),
        (Some(l), None) => l.max(1),
        (None, Some(h)) => (h - 1).max(1),
        (None, None) => 1,
    };
    w.timestamp = match (tlo, thi) {
        (None, None) => None,
        (l, h) => Some(
            l.unwrap_or(0)
                .max(0)
                .min(h.map(|h| h - 1).unwrap_or(i64::MAX))
                .max(0) as u64,
        ),
    };
    let secret_missing = conds.iter().any(|c| matches!(c, Condition::HashPreimage { var, .. } if !spec.witness.contains_key(&var.to_string())));
    if !secret_missing && !satisfied(p, &w, values)? {
        return Err(ComposeError::Unsatisfiable(
            p.name.clone(),
            "no single transaction meets every condition at once".into(),
        ));
    }
    Ok(w)
}

/// A plausible value of a register type, for a register nothing else fixes.
fn sample(tpe: &str) -> serde_json::Value {
    match tpe {
        "Boolean" => serde_json::json!(true),
        "Coll[Byte]" => serde_json::json!("0102030405"),
        "GroupElement" => {
            serde_json::json!("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
        }
        _ => serde_json::json!(7),
    }
}

/// What a violation of condition `c` looks like, in words.
fn violation_name(c: &Condition) -> String {
    match c {
        Condition::After(_) => "one block before its start date".into(),
        Condition::Before(_) => "at its end date".into(),
        Condition::AfterTime(_) => "one millisecond before its start time".into(),
        Condition::BeforeTime(_) => "at its end time".into(),
        Condition::BoxAge(_) => "one block too young".into(),
        Condition::InputCount(_) => "with one input too many".into(),
        Condition::OutputCount(_) => "with one output too many".into(),
        Condition::PayTo { .. } => "paying one nanoERG too little".into(),
        Condition::KeepHere { .. } => "keeping one nanoERG too little here".into(),
        Condition::OracleAbove { .. } => "oracle price one below the floor".into(),
        Condition::Box(r) => {
            // Most specific requirement first, so the case names differ
            // between rules that share a script.
            let what = if let Some(rr) = r.registers.first() {
                match rr.op {
                    RegOp::EqHeight => "not recording the current height",
                    RegOp::EqSelf => "not carrying the register over",
                    _ => "with the wrong register value",
                }
            } else if r.token.is_some() {
                "without the token"
            } else if r.no_tokens {
                "carrying a token"
            } else if r.keeps_self_tokens {
                "dropping this box's tokens"
            } else if r.value_at_least.is_some() || r.value_at_least_share.is_some() {
                "one nanoERG short"
            } else {
                "to the wrong script"
            };
            let which = match (r.which, r.index) {
                (Which::SelfBox, _) => "this box".to_string(),
                (w, Some(Index::At(i))) => format!("{} {i}", which_name(w)),
                (w, Some(Index::Word(Word::Any))) => {
                    format!("the {} meant to match", which_name(w))
                }
                (w, Some(Index::Word(Word::All))) => format!("one {}", which_name(w)),
                (w, None) => which_name(w).to_string(),
            };
            format!("{which} {what}")
        }
        Condition::VarEquals { index, .. } => format!("without variable {index}"),
        Condition::HashPreimage { var, .. } => format!("with the wrong secret in variable {var}"),
        Condition::TokenGated { .. } => "without the membership token".into(),
        Condition::MinerIs(_) => "mined by someone else".into(),
        Condition::SumPaidTo { .. } => "paying one nanoERG too little in total".into(),
    }
}

fn which_name(w: Which) -> &'static str {
    match w {
        Which::SelfBox => "this box",
        Which::Output => "output",
        Which::Input => "input",
        Which::DataInput => "data input",
    }
}

/// Break box `b` for rule `r`: its most specific requirement (the order
/// [`violation_name`] describes).
fn break_box(
    r: &BoxRule,
    b: &mut MBox,
    self_tokens: &[(String, u64)],
    values: &BTreeMap<String, TypedValue>,
) -> Result<(), ComposeError> {
    if let Some(rr) = r.registers.first() {
        match rr.op {
            RegOp::Eq | RegOp::EqSelf => {
                if let Some((t, RegVal::Lit(v))) = b.registers.get(&rr.reg).cloned() {
                    b.registers
                        .insert(rr.reg.clone(), (t, RegVal::Lit(bump(&rr.r#type, &v))));
                } else {
                    b.registers.remove(&rr.reg);
                }
            }
            RegOp::EqHeight | RegOp::Ne => {
                let v = match rr.op {
                    RegOp::Ne => val_of(values, rr.value.as_deref().unwrap_or(""))?,
                    _ => serde_json::json!(0),
                };
                b.registers
                    .insert(rr.reg.clone(), (rr.r#type.clone(), RegVal::Lit(v)));
            }
            RegOp::Gte | RegOp::Lte => {
                let v = val_of(values, rr.value.as_deref().unwrap_or(""))?;
                let n = json_num(&v).unwrap_or(0);
                let n = if rr.op == RegOp::Gte { n - 1 } else { n + 1 };
                b.registers.insert(
                    rr.reg.clone(),
                    (
                        rr.r#type.clone(),
                        RegVal::Lit(serde_json::json!(n.to_string())),
                    ),
                );
            }
        }
    } else if let Some(t) = &r.token {
        let id = str_of(values, &t.id)?;
        b.tokens.retain(|(i, _)| *i != id);
    } else if r.no_tokens {
        b.tokens.push((SELF_TOKEN.into(), 1));
    } else if r.keeps_self_tokens {
        if self_tokens.is_empty() {
            b.tokens.push((SELF_TOKEN.into(), 1));
        } else {
            b.tokens.clear();
        }
    } else if r.value_at_least.is_some() || r.value_at_least_share.is_some() {
        b.value -= 1;
    } else if r.script.is_some() {
        b.tree = ANY_TREE.into();
    }
    Ok(())
}

/// The world with condition `k` of path `p` violated (others as satisfied).
fn violating_world(
    spec: &Spec,
    p: &Path,
    k: usize,
    values: &BTreeMap<String, TypedValue>,
) -> Result<World, ComposeError> {
    let mut w = satisfying_world(spec, p, values)?;
    let conds = resolve_slots(&p.conditions);
    // Where did each "any" rule / SumPaidTo / TokenGated land? Replay the
    // allocation order of satisfying_world.
    let mut any_slots: BTreeMap<usize, (Which, usize)> = BTreeMap::new();
    {
        let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
        for (i, c) in conds.iter().enumerate() {
            match c {
                Condition::InputCount(n) => {
                    let n = int_of(values, n)?.max(1) as usize;
                    let cur = counts.entry(Which::Input as u8).or_insert(1);
                    *cur = (*cur).max(n);
                }
                Condition::Box(r) if r.index == Some(Index::Word(Word::Any)) => {
                    let key = r.which as u8;
                    let base = if r.which == Which::Input { 1 } else { 0 };
                    let cur = counts.entry(key).or_insert(base);
                    let at = *cur;
                    *cur += 1;
                    any_slots.insert(i, (r.which, at));
                }
                Condition::Box(r) => {
                    if let Some(Index::At(j)) = r.index {
                        let base = if r.which == Which::Input { 1 } else { 0 };
                        let cur = counts.entry(r.which as u8).or_insert(base);
                        *cur = (*cur).max(j + 1);
                    }
                }
                Condition::TokenGated { .. } => {
                    let cur = counts.entry(Which::Input as u8).or_insert(1);
                    *cur = (*cur).max(2);
                }
                Condition::SumPaidTo { .. } => {
                    let cur = counts.entry(Which::Output as u8).or_insert(0);
                    any_slots.insert(i, (Which::Output, *cur));
                    *cur += 1;
                }
                _ => {}
            }
        }
    }
    let c = &conds[k];
    match c {
        Condition::After(h) => w.height = (int_of(values, h)? - 1).max(1),
        Condition::Before(h) => w.height = int_of(values, h)?,
        Condition::AfterTime(t) => w.timestamp = Some((int_of(values, t)? - 1).max(0) as u64),
        Condition::BeforeTime(t) => w.timestamp = Some(int_of(values, t)?.max(0) as u64),
        Condition::BoxAge(n) => w.self_box.creation_height = w.height - int_of(values, n)? + 1,
        Condition::InputCount(_) => w.extra_inputs.push(MBox::default()),
        Condition::OutputCount(_) => w.outputs.push(MBox::default()),
        Condition::Box(r) => {
            let st = w.self_box.tokens.clone();
            match (r.which, r.index) {
                (Which::SelfBox, _) => break_box(r, &mut w.self_box, &st, values)?,
                (which, Some(Index::At(i))) => break_box(r, w.slot(which, i), &st, values)?,
                (which, Some(Index::Word(Word::Any))) => {
                    let (_, at) = any_slots.get(&k).copied().unwrap_or((which, 0));
                    break_box(r, w.slot(which, at), &st, values)?;
                }
                (which, Some(Index::Word(Word::All))) => {
                    let last = w.list(which).len().saturating_sub(1);
                    break_box(r, w.slot(which, last), &st, values)?;
                }
                (_, None) => unreachable!("slots resolved"),
            }
        }
        Condition::VarEquals { index, .. } => {
            w.vars.remove(index);
        }
        Condition::HashPreimage { var, .. } => {
            w.vars.insert(
                *var,
                TypedValue {
                    r#type: "Coll[Byte]".into(),
                    value: serde_json::json!("00"),
                },
            );
        }
        Condition::TokenGated { token_id } => {
            let id = str_of(values, token_id)?;
            for b in w.extra_inputs.iter_mut() {
                b.tokens.retain(|(i, _)| *i != id);
            }
        }
        Condition::MinerIs(_) => w.miner = None,
        Condition::SumPaidTo { .. } => {
            let (_, at) = any_slots.get(&k).copied().unwrap_or((Which::Output, 0));
            w.slot(Which::Output, at).value -= 1;
        }
        Condition::PayTo { .. } | Condition::KeepHere { .. } | Condition::OracleAbove { .. } => {
            unreachable!("lowered")
        }
    }
    Ok(w)
}

/// Expected verdict for a world: which paths are satisfied, and whether any
/// of them needs no key.
fn expectation(
    spec: &Spec,
    w: &World,
    values: &BTreeMap<String, TypedValue>,
) -> Result<(&'static str, Option<String>, Option<String>), ComposeError> {
    let mut sat: Vec<&Path> = Vec::new();
    for p in &spec.paths {
        if satisfied(p, w, values)? {
            sat.push(p);
        }
    }
    if sat.iter().any(|p| matches!(p.who, Who::AnyOne { .. })) {
        return Ok(("pass", None, None));
    }
    if sat.is_empty() {
        return Ok(("fail", None, None));
    }
    // A key of a satisfied path must appear; a key used ONLY by unsatisfied
    // paths must not (when the keys are distinct).
    let first_key = |p: &Path| -> Option<String> {
        match &p.who {
            Who::AnyOf { any_of: ks } | Who::AllOf { all_of: ks } | Who::KOf { keys: ks, .. } => {
                ks.first().cloned()
            }
            Who::AnyOne { .. } => None,
        }
    };
    let sat_hexes: Vec<String> = sat
        .iter()
        .filter_map(|p| first_key(p))
        .filter_map(|k| key_hex(values, &k).ok())
        .collect();
    let want = sat_hexes.first().map(|h| h[..8].to_string());
    let unsat_keys: Vec<String> = spec
        .paths
        .iter()
        .filter(|p| !sat.iter().any(|s| std::ptr::eq(*s, *p)))
        .filter_map(first_key)
        .filter_map(|k| key_hex(values, &k).ok())
        .filter(|h| !sat_hexes.contains(h))
        .collect();
    let excl = unsat_keys.first().map(|h| h[..8].to_string());
    Ok(("needsProof", want, excl))
}

fn box_json(b: &MBox, w: &World) -> serde_json::Value {
    let mut j = serde_json::json!({ "value": b.value, "ergoTree": b.tree, "creationHeight": b.creation_height.max(0) });
    if !b.tokens.is_empty() {
        j["tokens"] = b
            .tokens
            .iter()
            .map(|(id, n)| serde_json::json!({ "id": id, "amount": n }))
            .collect();
    }
    if !b.registers.is_empty() {
        // A real box's registers are dense from R4: fill the gaps.
        let top = b
            .registers
            .keys()
            .filter_map(|k| k[1..].parse::<u8>().ok())
            .max()
            .unwrap_or(4);
        let regs: serde_json::Map<String, serde_json::Value> = (4..=top)
            .map(|n| {
                let k = format!("R{n}");
                let v = match b.registers.get(&k) {
                    Some((t, v)) => serde_json::json!({ "type": t, "value": resolve(v, w) }),
                    None => serde_json::json!({ "type": "Int", "value": 0 }),
                };
                (k, v)
            })
            .collect();
        j["registers"] = serde_json::Value::Object(regs);
    }
    j
}

fn scenario_json(w: &World) -> serde_json::Value {
    let list = |bs: &[MBox]| -> serde_json::Value { bs.iter().map(|b| box_json(b, w)).collect() };
    let mut sc = serde_json::json!({ "height": w.height, "selfBox": box_json(&w.self_box, w), "outputs": list(&w.outputs) });
    // Synthetic boxes share an all-zero id unless told otherwise; a token
    // gate compares ids, so every input gets its own.
    sc["selfBox"]["boxId"] = serde_json::json!("11".repeat(32));
    if !w.extra_inputs.is_empty() {
        let mut inputs = list(&w.extra_inputs);
        for (i, b) in inputs.as_array_mut().unwrap().iter_mut().enumerate() {
            b["boxId"] = serde_json::json!(format!("{:02x}", 0x20 + i).repeat(32));
        }
        sc["inputs"] = inputs;
    }
    if !w.data_inputs.is_empty() {
        sc["dataInputs"] = list(&w.data_inputs);
    }
    if !w.vars.is_empty() {
        let vars: serde_json::Map<String, serde_json::Value> = w
            .vars
            .iter()
            .map(|(k, tv)| (k.to_string(), serde_json::to_value(tv).unwrap_or_default()))
            .collect();
        sc["contextVars"] = serde_json::Value::Object(vars);
    }
    if let Some(t) = w.timestamp {
        sc["preHeader"] = serde_json::json!({ "timestamp": t });
    }
    if let Some(m) = &w.miner {
        sc["minerPubkey"] = serde_json::json!(m);
    }
    sc
}

fn generate_suite(
    spec: &Spec,
    source: &str,
    values: &BTreeMap<String, TypedValue>,
) -> Result<Suite, ComposeError> {
    let mut scenarios = Vec::new();
    let mut add = |name: String, w: &World| -> Result<(), ComposeError> {
        let (expect, want, excl) = expectation(spec, w, values)?;
        let mut case = scenario_json(w);
        case["name"] = serde_json::Value::String(name);
        case["expect"] = serde_json::Value::String(expect.into());
        if let Some(x) = want {
            case["expectResidual"] = serde_json::Value::String(x);
        }
        if let Some(x) = excl {
            case["expectResidualExcludes"] = serde_json::Value::String(x);
        }
        scenarios.push(case);
        Ok(())
    };
    for p in &spec.paths {
        let w = satisfying_world(spec, p, values)?;
        let needs_secret = p.conditions.iter().any(|c| matches!(c, Condition::HashPreimage { var, .. } if !spec.witness.contains_key(&var.to_string())));
        let name = if needs_secret {
            format!("{}: every condition met (no secret supplied, so the reveal cannot be shown passing)", p.name)
        } else {
            format!("{}: every condition met", p.name)
        };
        add(name, &w)?;
        for (k, c) in p.conditions.iter().enumerate() {
            let w = violating_world(spec, p, k, values)?;
            add(format!("{}: {}", p.name, violation_name(c)), &w)?;
        }
    }
    if spec.paths.iter().any(|p| !p.conditions.is_empty()) {
        add(
            "baseline: no conditions met, no outputs".into(),
            &World::baseline(spec),
        )?;
    }
    let doc = serde_json::json!({ "source": source, "params": values, "scenarios": scenarios });
    serde_json::from_value(doc).map_err(|e| ComposeError::Value("suite".into(), e.to_string()))
}
