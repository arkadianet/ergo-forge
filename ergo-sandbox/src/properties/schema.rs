//! Strict `author-property:v1` input vocabulary and normalized declaration identity.
//!
//! Wire inputs can round-trip; only `Declaration::parse` constructs the immutable
//! checked form. Identity binds author premises, not execution acceptance. Future
//! replay must separately bind canonical transactions, contexts, revisions and
//! trace rules and recheck every observation against these fixed ceilings.
use crate::evidence::wire::WireBox;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const MAX_EXPRESSION_NODES: usize = 32;
pub const MAX_ROLES: usize = 8;
pub const MAX_BOXES: usize = 16;
pub const MAX_TOKENS: usize = 8;
pub const MAX_TRANSACTIONS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    #[error("invalid declaration: {0}")]
    Invalid(String),
    #[error("unsupported/capped: {0}")]
    Capped(String),
    #[error("unresolved binding: {0}")]
    Unresolved(String),
}
type Result<T> = std::result::Result<T, SchemaError>;
fn invalid(s: impl Into<String>) -> SchemaError {
    SchemaError::Invalid(s.into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclarationInput {
    pub schema_version: String,
    pub property_id: String,
    pub revision: String,
    pub contracts: BTreeMap<String, Contract>,
    pub scope: Scope,
    pub roles: BTreeMap<String, Role>,
    pub registers: BTreeMap<String, IntegerRegister>,
    pub guard: Expr,
    pub assertion: Expr,
    pub authorization_premises: Vec<Premise>,
    pub sources: Vec<Source>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contract {
    pub script: String,
    pub compiler_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Scope {
    Transition {},
    /// Guard is the trigger; assertion is the goal at each of K successor steps
    /// of ONE supplied trace. This reference does not certify the schedule.
    BoundedResponse {
        horizon: u8,
        schedule: Source,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub origin: Origin,
    pub reference: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    SourceRecorded,
    Hypothetical,
    Mixed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Premise {
    pub id: String,
    pub statement: String,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Role {
    pub collection: Collection,
    pub selector: Selector,
    pub cardinality: Cardinality,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Collection {
    Inputs,
    Outputs,
    DataInputs,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cardinality {
    ExactlyOne,
    AllMatches,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Selector {
    Position { index: usize },
    BoxId { hex: String },
    Script { hex: String },
    TokenId { hex: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Unit {
    NanoErg {},
    Token { id: String },
    Count {},
    Height {},
    Scalar {},
    NamedAmount { name: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegerRegister {
    pub role: String,
    pub index: u8,
    pub value_type: IntegerType,
    pub unit: Unit,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegerType {
    Byte,
    Short,
    Int,
    Long,
    BigInt,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ByteField {
    Script,
    BoxId,
}

/// Finite typed syntax. Decimal strings preserve the entire signed i128 range
/// across JSON implementations. Arithmetic is mathematical and checked by the
/// future evaluator; accepting syntax never certifies absence of overflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Expr {
    Boolean { value: bool },
    And { args: Vec<Expr> },
    Or { args: Vec<Expr> },
    Not { arg: Box<Expr> },
    Bytes { hex: String },
    ReadBytes { role: String, field: ByteField },
    BytesEq { left: Box<Expr>, right: Box<Expr> },
    Integer { value: String, unit: Unit },
    Count { role: String },
    SumErg { role: String },
    SumToken { role: String, token: String },
    Register { name: String },
    Height {},
    Eq { left: Box<Expr>, right: Box<Expr> },
    Lt { left: Box<Expr>, right: Box<Expr> },
    Le { left: Box<Expr>, right: Box<Expr> },
    Gt { left: Box<Expr>, right: Box<Expr> },
    Ge { left: Box<Expr>, right: Box<Expr> },
    Add { left: Box<Expr>, right: Box<Expr> },
    Sub { left: Box<Expr>, right: Box<Expr> },
    Scale { arg: Box<Expr>, factor: String },
}
#[derive(Debug, PartialEq, Eq)]
enum Type {
    Boolean,
    Bytes,
    Integer(Unit),
}

/// Private immutable normalized input; deliberately neither Deserialize nor a
/// result/claim type. Getters expose shared references only.
#[derive(Debug, Clone)]
pub struct Declaration {
    input: DeclarationInput,
    canonical: Vec<u8>,
    digest: String,
}
impl Declaration {
    pub fn parse(json: &str) -> Result<Self> {
        // Deserialize directly (not via Value): duplicate struct fields fail.
        let mut input: DeclarationInput =
            serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
        // Reject duplicate map keys too; serde's standard BTreeMap would overwrite.
        reject_duplicate_keys(json)?;
        input.normalize()?;
        let canonical = serde_json::to_vec(&input).map_err(|e| invalid(e.to_string()))?;
        let digest = hex::encode(Sha256::digest(&canonical));
        Ok(Self {
            input,
            canonical,
            digest,
        })
    }
    pub fn input(&self) -> &DeclarationInput {
        &self.input
    }
    pub fn canonical_json(&self) -> &[u8] {
        &self.canonical
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Resolve explicit selectors over caller-supplied canonical boxes only.
    /// This is input binding, NOT validation or property evaluation. No implicit
    /// successor carry-over, actor identity, or token uniqueness is inferred.
    /// All-matches may be empty; scalar reads require exactly-one declarations.
    pub fn bind_roles(
        &self,
        inputs: &[WireBox],
        outputs: &[WireBox],
        data_inputs: &[WireBox],
    ) -> Result<BTreeMap<String, Vec<usize>>> {
        for boxes in [inputs, outputs, data_inputs] {
            check_collection_limits(
                boxes.len(),
                boxes.iter().map(|b| b.node().candidate.tokens.len()),
            )?;
        }
        self.input
            .roles
            .iter()
            .map(|(name, role)| {
                let boxes = match role.collection {
                    Collection::Inputs => inputs,
                    Collection::Outputs => outputs,
                    Collection::DataInputs => data_inputs,
                };
                let mut matches = vec![];
                for (index, b) in boxes.iter().enumerate() {
                    let yes = match &role.selector {
                        Selector::Position { index: wanted } => index == *wanted,
                        Selector::BoxId { hex: id } => {
                            hex::encode(
                                b.node()
                                    .box_id()
                                    .map_err(|e| SchemaError::Unresolved(e.to_string()))?
                                    .as_bytes(),
                            ) == *id
                        }
                        Selector::Script { hex: script } => {
                            hex::encode(b.node().candidate.ergo_tree_bytes()) == *script
                        }
                        Selector::TokenId { hex: id } => b
                            .node()
                            .candidate
                            .tokens
                            .iter()
                            .any(|t| hex::encode(t.token_id.as_bytes()) == *id),
                    };
                    if yes {
                        matches.push(index);
                    }
                }
                if role.cardinality == Cardinality::ExactlyOne && matches.len() != 1 {
                    return Err(SchemaError::Unresolved(format!(
                        "{name}: expected exactly one, got {}",
                        matches.len()
                    )));
                }
                Ok((name.clone(), matches))
            })
            .collect()
    }
}

/// Shape checks only; future replay must call these on actual parsed material.
pub fn check_collection_limits(
    boxes: usize,
    token_counts: impl IntoIterator<Item = usize>,
) -> Result<()> {
    if boxes > MAX_BOXES {
        return Err(SchemaError::Capped("boxes per collection > 16".into()));
    }
    let mut count = 0;
    for tokens in token_counts {
        count += 1;
        if count > boxes {
            return Err(invalid("token counts do not cover exactly the collection"));
        }
        if tokens > MAX_TOKENS {
            return Err(SchemaError::Capped("token entries per box > 8".into()));
        }
    }
    if count != boxes {
        return Err(invalid("missing per-box token count"));
    }
    Ok(())
}
pub fn check_trace_limit(transactions: usize) -> Result<()> {
    if transactions == 0 {
        return Err(SchemaError::Unresolved("empty trace".into()));
    }
    if transactions > MAX_TRANSACTIONS {
        return Err(SchemaError::Capped("transactions per trace > 8".into()));
    }
    Ok(())
}
fn nonempty(s: &str) -> Result<()> {
    if s.trim().is_empty() {
        Err(invalid("empty identity/reference/name"))
    } else {
        Ok(())
    }
}
fn bytes(s: &mut String, size: Option<usize>) -> Result<()> {
    let b = hex::decode(&*s).map_err(|_| invalid("invalid exact hex bytes"))?;
    if size.is_some_and(|n| n != b.len()) {
        return Err(invalid("wrong identifier byte length"));
    }
    *s = hex::encode(b);
    Ok(())
}
fn script(s: &mut String) -> Result<()> {
    nonempty(s)?;
    bytes(s, None)
}
fn decimal(s: &mut String) -> Result<()> {
    *s = s
        .parse::<i128>()
        .map_err(|_| invalid("integer literal outside signed i128 or malformed"))?
        .to_string();
    Ok(())
}
impl Source {
    fn normalize(&mut self) -> Result<()> {
        nonempty(&self.reference)?;
        bytes(&mut self.sha256, Some(32))
    }
}
impl Unit {
    fn normalize(&mut self) -> Result<()> {
        match self {
            Self::Token { id } => bytes(id, Some(32)),
            Self::NamedAmount { name } => nonempty(name),
            _ => Ok(()),
        }
    }
}
impl DeclarationInput {
    fn normalize(&mut self) -> Result<()> {
        if self.schema_version != "author-property:v1" {
            return Err(invalid("unsupported property version"));
        }
        nonempty(&self.property_id)?;
        nonempty(&self.revision)?;
        if self.contracts.is_empty() || self.sources.is_empty() {
            return Err(invalid("missing contract identities or provenance"));
        }
        for (name, c) in &mut self.contracts {
            nonempty(name)?;
            script(&mut c.script)?;
            nonempty(&c.compiler_revision)?;
        }
        if let Scope::BoundedResponse { horizon, schedule } = &mut self.scope {
            if !(1..=4).contains(horizon) {
                return Err(invalid("response horizon must be 1..=4"));
            }
            schedule.normalize()?;
        }
        if self.roles.len() > MAX_ROLES {
            return Err(SchemaError::Capped("role bindings > 8".into()));
        }
        for (name, role) in &mut self.roles {
            nonempty(name)?;
            match &mut role.selector {
                Selector::Position { index } if *index >= MAX_BOXES => {
                    return Err(SchemaError::Capped(
                        "position outside 16-box collection".into(),
                    ))
                }
                Selector::Position { .. } => (),
                Selector::BoxId { hex } | Selector::TokenId { hex } => bytes(hex, Some(32))?,
                Selector::Script { hex } => script(hex)?,
            }
        }
        for (name, reg) in &mut self.registers {
            nonempty(name)?;
            singleton(&self.roles, &reg.role)?;
            if !(4..=9).contains(&reg.index) {
                return Err(invalid("integer register must be R4..R9"));
            }
            reg.unit.normalize()?;
        }
        // One physical read cannot acquire conflicting declared types or units.
        let mut reads = BTreeMap::new();
        for reg in self.registers.values() {
            let key = (&reg.role, reg.index);
            let signature = serde_json::to_vec(&(&reg.value_type, &reg.unit)).unwrap();
            if reads
                .insert(key, signature.clone())
                .is_some_and(|old| old != signature)
            {
                return Err(invalid("conflicting named register declarations"));
            }
        }
        for source in &mut self.sources {
            source.normalize()?;
        }
        let mut ids = std::collections::BTreeSet::new();
        for premise in &mut self.authorization_premises {
            nonempty(&premise.id)?;
            nonempty(&premise.statement)?;
            premise.source.normalize()?;
            if !ids.insert(&premise.id) {
                return Err(invalid("duplicate authorization premise"));
            }
        }
        let mut nodes = 0;
        for expr in [&mut self.guard, &mut self.assertion] {
            if expr.check(&self.roles, &self.registers, &mut nodes)? != Type::Boolean {
                return Err(invalid("guard/assertion must be boolean"));
            }
        }
        Ok(())
    }
}
fn role<'a>(roles: &'a BTreeMap<String, Role>, name: &str) -> Result<&'a Role> {
    roles
        .get(name)
        .ok_or_else(|| invalid(format!("undefined role {name}")))
}
fn singleton(roles: &BTreeMap<String, Role>, name: &str) -> Result<()> {
    if role(roles, name)?.cardinality != Cardinality::ExactlyOne {
        return Err(invalid("scalar field read requires exactly-one binding"));
    }
    Ok(())
}
impl Expr {
    fn check(
        &mut self,
        roles: &BTreeMap<String, Role>,
        registers: &BTreeMap<String, IntegerRegister>,
        nodes: &mut usize,
    ) -> Result<Type> {
        *nodes += 1;
        if *nodes > MAX_EXPRESSION_NODES {
            return Err(SchemaError::Capped(
                "expression nodes per declaration > 32".into(),
            ));
        }
        let mut child = |e: &mut Expr| e.check(roles, registers, nodes);
        Ok(match self {
            Self::Boolean { .. } => Type::Boolean,
            Self::And { args } | Self::Or { args } => {
                if args.is_empty() {
                    return Err(invalid("empty boolean connective; use explicit constant"));
                }
                for a in args {
                    if child(a)? != Type::Boolean {
                        return Err(invalid("boolean operand required"));
                    }
                }
                Type::Boolean
            }
            Self::Not { arg } => {
                if child(arg)? != Type::Boolean {
                    return Err(invalid("boolean operand required"));
                }
                Type::Boolean
            }
            Self::Bytes { hex } => {
                bytes(hex, None)?;
                Type::Bytes
            }
            Self::ReadBytes { role, .. } => {
                singleton(roles, role)?;
                Type::Bytes
            }
            Self::BytesEq { left, right } => {
                if child(left)? != Type::Bytes || child(right)? != Type::Bytes {
                    return Err(invalid("byte equality requires bytes"));
                }
                Type::Boolean
            }
            Self::Integer { value, unit } => {
                decimal(value)?;
                unit.normalize()?;
                Type::Integer(unit.clone())
            }
            Self::Count { role: name } => {
                role(roles, name)?;
                Type::Integer(Unit::Count {})
            }
            Self::SumErg { role: name } => {
                role(roles, name)?;
                Type::Integer(Unit::NanoErg {})
            }
            Self::SumToken { role: name, token } => {
                role(roles, name)?;
                bytes(token, Some(32))?;
                Type::Integer(Unit::Token { id: token.clone() })
            }
            Self::Register { name } => Type::Integer(
                registers
                    .get(name)
                    .ok_or_else(|| invalid("undefined integer register"))?
                    .unit
                    .clone(),
            ),
            Self::Height {} => Type::Integer(Unit::Height {}),
            Self::Scale { arg, factor } => {
                decimal(factor)?;
                let ty = child(arg)?;
                if !matches!(ty, Type::Integer(_)) {
                    return Err(invalid("scale requires integer"));
                }
                ty
            }
            Self::Eq { left, right }
            | Self::Lt { left, right }
            | Self::Le { left, right }
            | Self::Gt { left, right }
            | Self::Ge { left, right }
            | Self::Add { left, right }
            | Self::Sub { left, right } => {
                let a = child(left)?;
                let b = child(right)?;
                if !matches!(a, Type::Integer(_)) || a != b {
                    return Err(invalid("integer operands require identical declared units"));
                }
                if matches!(self, Self::Add { .. } | Self::Sub { .. }) {
                    a
                } else {
                    Type::Boolean
                }
            }
        })
    }
}

// Visit the original token stream so duplicate role/register/contract keys can
// never disappear through map deserialization. Unknown fields are separately
// refused by the typed input above, at every nesting level.
fn reject_duplicate_keys(json: &str) -> Result<()> {
    struct Unique;
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Unique;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("JSON with unique object keys")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> std::result::Result<Unique, A::Error> {
                    let mut keys = std::collections::BTreeSet::new();
                    while let Some(key) = map.next_key::<String>()? {
                        if !keys.insert(key) {
                            return Err(serde::de::Error::custom("duplicate object key"));
                        }
                        map.next_value::<Unique>()?;
                    }
                    Ok(Unique)
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut seq: A,
                ) -> std::result::Result<Unique, A::Error> {
                    while seq.next_element::<Unique>()?.is_some() {}
                    Ok(Unique)
                }
                fn visit_bool<E: serde::de::Error>(
                    self,
                    _: bool,
                ) -> std::result::Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Unique, E> {
                    Ok(Unique)
                }
            }
            d.deserialize_any(Visitor)
        }
    }
    serde_json::from_str::<Unique>(json)
        .map(|_| ())
        .map_err(|e| invalid(e.to_string()))
}
