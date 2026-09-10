//! Fresh execution refutations of author assertions, never universal proof or defect attribution.
use super::{
    schema::*,
    trace::{self, Step, SuppliedTrace},
};
use crate::evidence::wire::WireBox;
use ergo_ser::sigma_value::SigmaValue;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Inspection only: no public constructor and no deserialization authority.
/// ```compile_fail
/// use ergo_sandbox::properties::evaluate::Evaluation;
/// let _: Evaluation = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use ergo_sandbox::properties::evaluate::Evaluation;
/// let forged = Evaluation {};
/// ```
#[derive(Debug)]
pub struct Evaluation {
    report: Value,
}
impl Evaluation {
    pub fn report(&self) -> &Value {
        &self.report
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Operand {
    Bool(bool),
    Int(i128),
    Bytes(String),
}
impl Operand {
    fn int(self) -> Result<i128, String> {
        if let Self::Int(n) = self {
            Ok(n)
        } else {
            Err("integer type mismatch".into())
        }
    }
    fn boolean(self) -> Result<bool, String> {
        if let Self::Bool(b) = self {
            Ok(b)
        } else {
            Err("boolean type mismatch".into())
        }
    }
    fn report(&self) -> Value {
        match self {
            Self::Int(n) => json!({"integer":n.to_string()}),
            Self::Bool(b) => json!({"boolean":b}),
            Self::Bytes(b) => json!({"bytes":b}),
        }
    }
}
struct Observation<'a> {
    d: &'a Declaration,
    s: &'a Step,
    bindings: BTreeMap<String, Vec<usize>>,
    operands: Vec<Value>,
}
impl Observation<'_> {
    fn boxes(&self, role: &str) -> Vec<&WireBox> {
        let all = match self.d.input().roles[role].collection {
            Collection::Inputs => &self.s.inputs,
            Collection::Outputs => &self.s.outputs,
            Collection::DataInputs => &self.s.data,
        };
        self.bindings[role].iter().map(|i| &all[*i]).collect()
    }
    fn eval(&mut self, e: &Expr) -> Result<Operand, String> {
        use Expr::*;
        let value = match e {
            Boolean { value } => Operand::Bool(*value),
            And { args } | Or { args } => {
                let values = args
                    .iter()
                    .map(|a| self.eval(a)?.boolean())
                    .collect::<Result<Vec<_>, _>>()?;
                Operand::Bool(if matches!(e, And { .. }) {
                    values.iter().all(|b| *b)
                } else {
                    values.iter().any(|b| *b)
                })
            }
            Not { arg } => Operand::Bool(!self.eval(arg)?.boolean()?),
            Bytes { hex } => Operand::Bytes(hex.clone()),
            ReadBytes { role, field } => {
                let b = self.boxes(role)[0];
                Operand::Bytes(match field {
                    ByteField::Script => hex::encode(b.node().candidate.ergo_tree_bytes()),
                    ByteField::BoxId => b.id()?,
                })
            }
            BytesEq { left, right } => {
                let a = self.eval(left)?;
                let b = self.eval(right)?;
                Operand::Bool(a == b)
            }
            Integer { value, .. } => Operand::Int(value.parse().map_err(|_| "invalid integer")?),
            Count { role } => Operand::Int(self.boxes(role).len() as i128),
            SumErg { role } => Operand::Int(self.boxes(role).iter().try_fold(0i128, |n, b| {
                n.checked_add(b.node().candidate.value as i128)
                    .ok_or("sum overflow")
            })?),
            SumToken { role, token } => Operand::Int(
                self.boxes(role)
                    .iter()
                    .flat_map(|b| &b.node().candidate.tokens)
                    .filter(|t| hex::encode(t.token_id.as_bytes()) == *token)
                    .try_fold(0i128, |n, t| {
                        n.checked_add(t.amount as i128).ok_or("sum overflow")
                    })?,
            ),
            Height {} => Operand::Int(
                self.s
                    .accepted
                    .request()
                    .block_context
                    .value()
                    .ok_or("missing height")?
                    .height as i128,
            ),
            Register { name } => {
                let r = &self.d.input().registers[name];
                let b = self.boxes(&r.role)[0];
                let v = &b
                    .node()
                    .candidate
                    .additional_registers
                    .registers
                    .get((r.index - 4) as usize)
                    .ok_or("missing register")?
                    .value;
                let n = match (&r.value_type, v) {
                    (IntegerType::Byte, SigmaValue::Byte(n)) => *n as i128,
                    (IntegerType::Short, SigmaValue::Short(n)) => *n as i128,
                    (IntegerType::Int, SigmaValue::Int(n)) => *n as i128,
                    (IntegerType::Long, SigmaValue::Long(n)) => *n as i128,
                    (IntegerType::BigInt, SigmaValue::BigInt(n)) => {
                        n.to_string().parse().map_err(|_| "register outside i128")?
                    }
                    _ => return Err("register type mismatch".into()),
                };
                Operand::Int(n)
            }
            Scale { arg, factor } => Operand::Int(
                self.eval(arg)?
                    .int()?
                    .checked_mul(factor.parse().map_err(|_| "factor")?)
                    .ok_or("arithmetic overflow")?,
            ),
            Eq { left, right }
            | Lt { left, right }
            | Le { left, right }
            | Gt { left, right }
            | Ge { left, right }
            | Add { left, right }
            | Sub { left, right } => {
                let a = self.eval(left)?.int()?;
                let b = self.eval(right)?.int()?;
                match e {
                    Eq { .. } => Operand::Bool(a == b),
                    Lt { .. } => Operand::Bool(a < b),
                    Le { .. } => Operand::Bool(a <= b),
                    Gt { .. } => Operand::Bool(a > b),
                    Ge { .. } => Operand::Bool(a >= b),
                    Add { .. } => Operand::Int(a.checked_add(b).ok_or("arithmetic overflow")?),
                    _ => Operand::Int(a.checked_sub(b).ok_or("arithmetic overflow")?),
                }
            }
        };
        self.operands
            .push(json!({"expression":e,"value":value.report()}));
        Ok(value)
    }
}
/// Step zero is the nominated trigger; K successor observations belong to this
/// single supplied trace. Early termination remains unresolved even after a goal.
/// Every invocation revalidates all supplied steps. Execution/linkage rejection
/// is separate from the five property dispositions; it cannot return a violation.
pub fn evaluate(d: &Declaration, t: &SuppliedTrace) -> Result<Evaluation, String> {
    let steps = trace::check(t)?;
    let mut report = json!({"propertyDigest":d.digest(),"rootDigest":trace::digest(&t.root),"scheduleDigest":trace::digest(&t.schedule),"triggerIndex":0,"evaluatorRevision":"author-property-evaluator:v1","nodeRevision":crate::evidence::validate::node_revision(),"premiseDigest":trace::digest(&(d.digest(),&t.root,&t.schedule,t.steps.iter().map(|r|r.fingerprint()).collect::<Vec<_>>())),"reachability":"assumed-root","modelReview":"not-adjudicated","contractDefect":"not-adjudicated","observations":[],"status":"unresolved","firstFailingIndex":null});
    if let Scope::BoundedResponse { schedule, .. } = &d.input().scope {
        if schedule.sha256 != trace::digest(&t.schedule) {
            return Err("unresolved: declaration schedule digest mismatch".into());
        }
    }
    let horizon = match d.input().scope {
        Scope::Transition {} => 0,
        Scope::BoundedResponse { horizon, .. } => horizon as usize,
    };
    if horizon == 0 && steps.len() != 1 {
        return Err("transition requires exactly one step".into());
    }
    let mut guard = None;
    let mut goals = vec![];
    let mut unknown = false;
    for (index, s) in steps.iter().enumerate() {
        let in_scope = s.inputs.iter().chain(&s.outputs).chain(&s.data).any(|b| {
            d.input()
                .contracts
                .values()
                .any(|c| c.script == hex::encode(b.node().candidate.ergo_tree_bytes()))
        });
        let bindings = if in_scope {
            d.bind_roles(&s.inputs, &s.outputs, &s.data)
        } else {
            Err(SchemaError::Unresolved(
                "no declared contract in step observations".into(),
            ))
        };
        let mut row = json!({"index":index,"prefixDigest":s.prefix,"requestFingerprint":s.accepted.request().fingerprint(),"execution":s.accepted.report()});
        match bindings {
            Err(e) => {
                row["unresolved"] = json!(e.to_string());
                unknown = true
            }
            Ok(bindings) => {
                let mut o = Observation {
                    d,
                    s,
                    bindings,
                    operands: vec![],
                };
                let value = if index == 0 {
                    o.eval(&d.input().guard).and_then(Operand::boolean)
                } else {
                    Ok(true)
                };
                if index == 0 {
                    guard = value.as_ref().ok().copied();
                }
                let assertion =
                    if value == Ok(true) && (horizon == 0 || (index > 0 && index <= horizon)) {
                        Some(o.eval(&d.input().assertion).and_then(Operand::boolean))
                    } else {
                        None
                    };
                if value.is_err() {
                    unknown = true;
                    row["unresolved"] = json!(value.err());
                }
                if let Some(a) = assertion {
                    match a {
                        Ok(b) => goals.push((index, b)),
                        Err(e) => {
                            unknown = true;
                            row["unresolved"] = json!(e)
                        }
                    }
                }
                row["bindingBoxIds"] = json!(o
                    .bindings
                    .keys()
                    .map(|role| {
                        Ok((
                            role.clone(),
                            o.boxes(role)
                                .iter()
                                .map(|b| b.id())
                                .collect::<Result<Vec<_>, String>>()?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>, String>>()?);
                row["bindings"] = json!(o.bindings);
                row["operands"] = json!(o.operands);
            }
        }
        report["observations"].as_array_mut().unwrap().push(row);
    }
    let status = if guard == Some(false) {
        "not-applicable"
    } else if guard.is_none() || unknown || (horizon > 0 && steps.len() <= horizon) {
        "unresolved"
    } else if goals.iter().any(|(_, b)| *b) {
        "holds-on-execution"
    } else {
        "violated"
    };
    report["guard"] = json!(guard);
    report["goals"] = json!(goals);
    report["status"] = json!(status);
    if status == "violated" {
        report["firstFailingIndex"] = json!(horizon);
        report["firstFailingAssertion"] = json!(d.input().assertion);
    }
    Ok(Evaluation { report })
}
