//! Conservative program identity under substitution of constant leaves.
//!
//! Compares parsed wire IR, before the decompiler's lossy source-like rewrites.
//! No source text, rendered expressions, or byte substrings are compared.
use std::collections::{BTreeMap, BTreeSet};

use ergo_primitives::reader::VlqReader;
use ergo_ser::{
    ergo_tree::{read_ergo_tree, ErgoTree},
    opcode::{Expr, Payload},
    sigma_type::SigmaType,
    sigma_value::{CollValue, SigmaValue},
};
use serde::Serialize;

use crate::SandboxError;

pub const LIMITATION: &str = "Structural identity under constant substitution does not prove behavioural equivalence: a constant can change which branch is reachable, authorization, or asset identity. This is not proof of deployment, safety, or compiler/source provenance. Binding IDs and non-constant types must match; compiler rewrites can cause false negatives. Unused constant-table entries and serialization flags are ignored; tree versions must match. For different programs, constant differences are positional observations, not a substitution map.";
pub const SIMILARITY_DEFINITION: &str = "Equal node labels at the same ordered child path, divided by the union of node paths, including one tree-version label. Constant leaves are untyped holes. This is a structural overlap score, not a probability or a behavioural-equivalence score; only 1.0 establishes this structural match.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchVerdict {
    SameProgram,
    SameProgramWithDifferingConstants,
    DifferentProgram,
}

impl std::fmt::Display for MatchVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SameProgram => "same program",
            Self::SameProgramWithDifferingConstants => "same program with differing constants",
            Self::DifferentProgram => "different program",
        })
    }
}

/// A resolved constant occurrence. `table_index` is absent for inline literals.
/// Values use the complete (untruncated) typed IR debug representation.
#[derive(Debug, Clone, Serialize)]
pub struct ConstantValue {
    pub table_index: Option<u32>,
    pub r#type: String,
    pub value: String,
}

/// Positions are ordered child paths: [] is root, [0] its first child, etc.
/// A missing side means that path is absent or contains an operation, not a hole.
/// For different programs these are positional observations, not substitutions.
#[derive(Debug, Clone, Serialize)]
pub struct ConstantDifference {
    pub path: Vec<usize>,
    pub left: Option<ConstantValue>,
    pub right: Option<ConstantValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchReport {
    pub verdict: MatchVerdict,
    pub byte_identical: bool,
    pub left_version: u8,
    pub right_version: u8,
    pub matching_labels: usize,
    pub total_positions: usize,
    pub similarity: f64,
    pub constant_differences: Vec<ConstantDifference>,
    pub similarity_definition: &'static str,
    pub limitation: &'static str,
}

#[derive(PartialEq)]
enum Label {
    Hole,
    // Compare opcode and Payload separately: IrNode::eq normalizes some ops.
    Op(u8, Payload),
}
struct Entry {
    label: Label,
    constant: Option<(Option<u32>, SigmaType, SigmaValue)>,
}
type Structure = BTreeMap<Vec<usize>, Entry>;

/// Match two serialized ErgoTrees. Rejects trailing data, unresolved constants,
/// and unparsed/unsupported bodies rather than issuing an identity claim.
/// Call through `decompile::with_large_stack` on small-stack hosts, as for other
/// parser consumers. No compilation or evaluation is performed here.
pub fn match_trees(left: &[u8], right: &[u8]) -> Result<MatchReport, SandboxError> {
    let a = parse(left)?;
    let b = parse(right)?;
    let mut sa = Structure::new();
    let mut sb = Structure::new();
    flatten(a.body, &a.constants, &mut Vec::new(), &mut sa)?;
    flatten(b.body, &b.constants, &mut Vec::new(), &mut sb)?;
    let paths: BTreeSet<_> = sa.keys().chain(sb.keys()).collect();
    let total_positions = paths.len() + 1;
    let mut matching_labels = usize::from(a.version == b.version);
    let mut constant_differences = Vec::new();
    for path in paths {
        let x = sa.get(path);
        let y = sb.get(path);
        matching_labels += usize::from(matches!((x, y), (Some(x), Some(y)) if x.label == y.label));
        let cx = x.and_then(|e| e.constant.as_ref());
        let cy = y.and_then(|e| e.constant.as_ref());
        if cx.map(|(_, t, v)| (t, v)) != cy.map(|(_, t, v)| (t, v)) {
            constant_differences.push(ConstantDifference {
                path: path.clone(),
                left: cx.map(describe),
                right: cy.map(describe),
            });
        }
    }
    Ok(MatchReport {
        verdict: if matching_labels != total_positions {
            MatchVerdict::DifferentProgram
        } else if constant_differences.is_empty() {
            MatchVerdict::SameProgram
        } else {
            MatchVerdict::SameProgramWithDifferingConstants
        },
        byte_identical: left == right,
        left_version: a.version,
        right_version: b.version,
        matching_labels,
        total_positions,
        similarity: matching_labels as f64 / total_positions as f64,
        constant_differences,
        similarity_definition: SIMILARITY_DEFINITION,
        limitation: LIMITATION,
    })
}

fn describe((index, tpe, val): &(Option<u32>, SigmaType, SigmaValue)) -> ConstantValue {
    ConstantValue {
        table_index: *index,
        r#type: crate::inspect::type_str(tpe),
        value: format!("{val:?}"),
    }
}
fn parse(bytes: &[u8]) -> Result<ErgoTree, SandboxError> {
    let mut reader = VlqReader::new(bytes);
    let tree = read_ergo_tree(&mut reader).map_err(|e| SandboxError::Tree(e.to_string()))?;
    if reader.remaining() != 0 {
        return Err(SandboxError::Tree("trailing bytes after ErgoTree".into()));
    }
    Ok(tree)
}

fn flatten(
    expr: Expr,
    constants: &[(SigmaType, SigmaValue)],
    path: &mut Vec<usize>,
    out: &mut Structure,
) -> Result<(), SandboxError> {
    let constant = match &expr {
        Expr::Unparsed(_) => {
            return Err(SandboxError::Tree(
                "cannot match an unparsed ErgoTree".into(),
            ))
        }
        Expr::Const { tpe, val } => Some((None, tpe.clone(), val.clone())),
        Expr::Op(n) => match &n.payload {
            Payload::ConstPlaceholder { index } => {
                let (t, v) = constants.get(*index as usize).ok_or_else(|| {
                    SandboxError::Tree(format!("unresolved constant index {index}"))
                })?;
                Some((Some(*index), t.clone(), v.clone()))
            }
            Payload::Zero if matches!(n.opcode, 0x7f | 0x80) => Some((
                None,
                SigmaType::SBoolean,
                SigmaValue::Boolean(n.opcode == 0x7f),
            )),
            Payload::BoolCollection { bits } => Some((
                None,
                SigmaType::SColl(Box::new(SigmaType::SBoolean)),
                SigmaValue::Coll(CollValue::BoolBits(bits.clone())),
            )),
            _ => None,
        },
    };
    let (label, children) = if constant.is_some() {
        (Label::Hole, Vec::new())
    } else if let Expr::Op(mut n) = expr {
        let children = take_children(&mut n.payload);
        (Label::Op(n.opcode, n.payload), children)
    } else {
        unreachable!("all non-operations handled above")
    };
    out.insert(path.clone(), Entry { label, constant });
    for (i, child) in children.into_iter().enumerate() {
        path.push(i);
        flatten(child, constants, path, out)?;
        path.pop();
    }
    Ok(())
}

// Exhaustive over the parser's payload variants. Retain all metadata, arities,
// and optional-child presence; replace each child with the same sentinel.
// The sentinel is internal only and is never serialized or evaluated.
fn take_children(p: &mut Payload) -> Vec<Expr> {
    fn take(e: &mut Expr) -> Expr {
        std::mem::replace(
            e,
            Expr::Const {
                tpe: SigmaType::SBoolean,
                val: SigmaValue::Boolean(false),
            },
        )
    }
    match p {
        Payload::One(a) => vec![take(a)],
        Payload::Two(a, b) => vec![take(a), take(b)],
        Payload::Three(a, b, c) => vec![take(a), take(b), take(c)],
        Payload::Four(a, b, c, d) => vec![take(a), take(b), take(c), take(d)],
        Payload::ValDef { rhs, .. } | Payload::FunDef { rhs, .. } => vec![take(rhs)],
        Payload::BlockValue { items, result } => items
            .iter_mut()
            .map(take)
            .chain(std::iter::once(take(result)))
            .collect(),
        Payload::FuncValue { body, .. } => vec![take(body)],
        Payload::MethodCall { obj, args, .. } => std::iter::once(take(obj))
            .chain(args.iter_mut().map(take))
            .collect(),
        Payload::ConcreteCollection { items, .. }
        | Payload::Tuple { items }
        | Payload::SigmaCollection { items } => items.iter_mut().map(take).collect(),
        Payload::SelectField { input, .. }
        | Payload::ExtractRegisterAs { input, .. }
        | Payload::NumericCast { input, .. } => vec![take(input)],
        Payload::DeserializeRegister { default, .. } => {
            default.as_deref_mut().map(take).into_iter().collect()
        }
        Payload::ByIndex {
            input,
            index,
            default,
        } => vec![take(input), take(index)]
            .into_iter()
            .chain(default.as_deref_mut().map(take))
            .collect(),
        Payload::FuncApply { func, args } => std::iter::once(take(func))
            .chain(args.iter_mut().map(take))
            .collect(),
        Payload::Zero
        | Payload::ValUse { .. }
        | Payload::ConstPlaceholder { .. }
        | Payload::TaggedVar { .. }
        | Payload::BoolCollection { .. }
        | Payload::GetVar { .. }
        | Payload::DeserializeContext { .. }
        | Payload::NoneValue { .. } => vec![],
    }
}
