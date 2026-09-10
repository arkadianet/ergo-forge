//! Explicit supplied alternatives only. Reports are not proof capabilities or
//! consensus verdicts. No discovery conversion, input search or request builder.
use super::{
    check_relation::{canonical_selector, check, selector_matches, valid_guard, StateDomain},
    necessity::{propose, Derivation},
    relations::*,
};
use crate::evidence::{
    case::{engine_revision, json_digest, RecordedBox},
    validate::{validate, ValidationRequest},
    wire::{CandidateSpec, WireBox},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NamedInput {
    pub name: String,
    pub material: RecordedBox,
}
/// Caller input syntax only; unknown envelope versions and report fields fail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Action {
    pub version: ActionVersion,
    pub id: String,
    pub spending_inputs: Vec<NamedInput>,
    pub data_inputs: Vec<NamedInput>,
    pub outputs: Vec<CandidateSpec>,
    /// Opaque supplied input-local extension material. No Execute authority in M04.
    pub extensions: BTreeMap<String, Value>,
    /// None explicitly means the context guard value is unknown.
    pub height: Option<u32>,
    pub relation_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionVersion {
    #[serde(rename = "action-check:v1")]
    V1,
}
/// Imported material must be freshly checked; serialized status has no authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Requirement {
    pub claim: RelationProposal,
    pub derivation: Option<Derivation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    SatisfiesDeclaredRequirements,
    ViolatesDeclaredRequirement,
    Unresolved,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCheck {
    version: &'static str,
    pub action_id: String,
    pub disposition: Disposition,
    action_digest: String,
    scope: &'static str,
    pub requirements: Vec<Value>,
}
fn guard(g: &Guard, height: Option<u32>, value: u64) -> Option<bool> {
    match g {
        Guard::True => Some(true),
        Guard::HeightAtLeast { value } => Some(i64::from(height?) >= i64::from(*value)),
        Guard::Equals {
            field: GuardField::Context { name },
            literal: Literal::Int(i),
        } if name == "HEIGHT" => Some(i64::from(height?) == i64::from(*i)),
        Guard::Equals {
            field: GuardField::SelfBox { name },
            literal: Literal::Long(i),
        } if name == "value" => Some(i128::from(value) == i128::from(*i)),
        Guard::And { guards } => Some(
            guards
                .iter()
                .map(|g| guard(g, height, value))
                .collect::<Option<Vec<_>>>()?
                .iter()
                .all(|v| *v),
        ),
        Guard::Or { guards } => Some(
            guards
                .iter()
                .map(|g| guard(g, height, value))
                .collect::<Option<Vec<_>>>()?
                .iter()
                .any(|v| *v),
        ),
        Guard::Not { guard: g } => Some(!guard(g, height, value)?),
        _ => None,
    }
}
fn inputs(action: &Action) -> Result<Vec<WireBox>, String> {
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut spending = vec![];
    for (is_spending, list) in [
        (true, &action.spending_inputs),
        (false, &action.data_inputs),
    ] {
        for input in list {
            let b = WireBox::from_record(input.material.clone())?;
            if input.name.is_empty() || !names.insert(&input.name) || !ids.insert(b.id()?) {
                return Err("duplicate/ambiguous input name or canonical identity".into());
            }
            if is_spending {
                spending.push(b);
            }
        }
    }
    if action
        .extensions
        .keys()
        .any(|k| !action.spending_inputs.iter().any(|i| &i.name == k))
    {
        return Err("extension names an absent spending input".into());
    }
    for output in &action.outputs {
        output.build()?;
    }
    Ok(spending)
}
fn bound_self(claim: &RelationProposal, inputs: &[WireBox]) -> Result<WireBox, String> {
    let p = &claim.premises;
    let fixed = WireBox::from_record(p.self_box.value().ok_or("missing canonical SELF")?.clone())?;
    let root = p.root_bytes.value().ok_or("missing exact root")?;
    if *root != hex::encode(fixed.node().candidate.ergo_tree_bytes()) {
        return Err("root/SELF mismatch".into());
    }
    match &claim.subject {
        Subject::BoxId { hex } if *hex == fixed.id()? => {}
        Subject::Script { hex } if hex == root => {}
        _ => return Err("subject/SELF mismatch".into()),
    }
    if !inputs.iter().any(|b| b.node() == fixed.node()) {
        return Err(
            "subject not bound to a concrete spending input agreeing with fixed SELF".into(),
        );
    }
    Ok(fixed)
}
fn omitted(
    claim: &RelationProposal,
    inputs: &[WireBox],
    subject: &WireBox,
) -> Result<bool, String> {
    let Relation::Spend { selector } = &claim.target else {
        return Err("M04 checks Spend only; other relations unresolved".into());
    };
    if !canonical_selector(selector, 0) {
        return Err("unsupported selector".into());
    }
    for b in inputs {
        if b.id()? != subject.id()? && selector_matches(selector, b)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn assess(action: &Action, requirement: &Requirement) -> Result<(bool, bool), String> {
    let claim = &requirement.claim;
    check(
        claim,
        requirement
            .derivation
            .as_ref()
            .ok_or("missing checked derivation")?,
    )?;
    let bs = inputs(action)?;
    let subject = bound_self(claim, &bs)?;
    let domain: StateDomain = serde_json::from_value(
        claim
            .premises
            .state_constraints
            .value()
            .ok_or("missing state")?
            .clone(),
    )
    .map_err(|e| e.to_string())?;
    if action
        .height
        .is_some_and(|h| h != domain.block_context.height)
    {
        return Err("action context differs from fixed premise domain".into());
    }
    if bs
        .iter()
        .any(|b| b.node().candidate.tokens.iter().any(|t| t.amount == 0))
    {
        return Err("input violates positive-token premise".into());
    }
    // Action checks are conditional declarations, not validation of the whole P.
    let active = guard(
        &claim.premises.guard,
        action.height,
        subject.node().candidate.value,
    )
    .ok_or("unknown guard")?;
    let missing = omitted(claim, &bs, &subject)?;
    Ok((active, active && missing))
}
/// Each action is evaluated from its own material; companions are never unioned.
pub fn check_actions(actions: &[Action], requirements: &[Requirement]) -> Vec<ActionCheck> {
    let mut action_ids = BTreeSet::new();
    let duplicate_actions = actions
        .iter()
        .any(|a| a.id.is_empty() || !action_ids.insert(&a.id));
    actions.iter().map(|action| {
        let mut rows = vec![];
        let mut unknown = duplicate_actions || action.relation_ids.is_empty();
        let mut violation = false;
        let mut seen = BTreeSet::new();
        for id in &action.relation_ids {
            let candidates: Vec<_> = requirements.iter().filter(|r| r.claim.claim_digest() == *id).collect();
            let result = if duplicate_actions || !seen.insert(id) || candidates.len() != 1 {
                Err("missing/ambiguous action or relation binding".into())
            } else { assess(action, candidates[0]) };
            match result {
                Ok((active, missing)) => {
                    violation |= missing;
                    rows.push(json!({"claimDigest":id,"premiseDigest":candidates[0].claim.premises.digest(),"guard":active,"status":if missing {"violates-declared-requirement"} else {"satisfies-declared-requirements"}}));
                },
                Err(reason) => { unknown = true; rows.push(json!({"claimDigest":id,"status":"unresolved","reason":reason})); }
            }
        }
        ActionCheck {
            version: "action-check:v1", action_id: action.id.clone(),
            disposition: if violation { Disposition::ViolatesDeclaredRequirement } else if unknown { Disposition::Unresolved } else { Disposition::SatisfiesDeclaredRequirements },
            action_digest: json_digest(&serde_json::to_value(action).expect("action serializes")),
            scope: "conditional on declared premises; not transaction acceptance, proof, exhaustive or minimal actions",
            requirements: rows,
        }
    }).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OmissionWitness {
    pub version: WitnessVersion,
    pub claim_digest: String,
    pub premise_digest: String,
    pub subject_input_id: String,
    pub execution: ValidationRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WitnessVersion {
    #[serde(rename = "omission-witness:v1")]
    V1,
}
/// No Deserialize implementation: imported reports cannot refute a claim.
#[derive(Debug)]
pub struct Refutation {
    report: Value,
}
impl Refutation {
    pub fn report(&self) -> &Value {
        &self.report
    }
    pub fn blocks_release(&self) -> bool {
        self.report["conflictWithEstablished"] == true
    }
}
/// Always runs the existing full validator. A failure preserves its exact stage;
/// rejection is neither an omission refutation nor evidence of universal necessity.
pub fn refute(claim: &RelationProposal, witness: &OmissionWitness) -> Result<Refutation, Value> {
    if witness.claim_digest != claim.claim_digest()
        || witness.premise_digest != claim.premises.digest()
    {
        return Err(json!({"status":"unresolved","reason":"claim/premise digest mismatch"}));
    }
    let accepted = validate(&witness.execution)
        .map_err(|f| serde_json::to_value(f).expect("failure serializes"))?;
    let assessment = (|| -> Result<(), String> {
        let p = &claim.premises;
        let req = accepted.request();
        let domain: StateDomain = serde_json::from_value(
            p.state_constraints
                .value()
                .ok_or("missing state domain")?
                .clone(),
        )
        .map_err(|e| e.to_string())?;
        let actual = StateDomain {
            block_context: req.block_context.value().ok_or("missing context")?.clone(),
            parameters: req.parameters.value().ok_or("missing parameters")?.clone(),
            network_rules: req
                .network_rules
                .value()
                .ok_or("missing network rules")?
                .clone(),
            headers: req.headers.value().ok_or("missing headers")?.clone(),
            prior_block_cost: *req.prior_block_cost.value().ok_or("missing prior cost")?,
            local_policy: req
                .local_policy
                .value()
                .ok_or("missing local policy")?
                .clone(),
            positive_input_tokens: true,
        };
        if serde_json::to_value(&domain).unwrap() != serde_json::to_value(actual).unwrap()
            || p.node_revision.value().map(String::as_str) != Some(engine_revision())
            || p.compiler_revision.value().map(String::as_str) != Some(engine_revision())
            || req.case.premises().engine_revision != engine_revision()
            || p.network.value().map(String::as_str) != Some("hypothetical-mainnet-activation-3")
            || p.activation_rules.value()
                != Some(
                    &json!({"activatedScriptVersion":3,"rootEvaluation":"required-before-storage-rent"}),
                )
            || p.provenance.value().is_none()
        {
            return Err("execution does not match complete supported premise domain".into());
        }
        let records = req
            .case
            .premises()
            .boxes
            .value()
            .ok_or("missing canonical boxes")?;
        let mut bs = vec![];
        for node in accepted.checked().resolved_inputs() {
            let record = records
                .iter()
                .find(|r| WireBox::from_record((*r).clone()).is_ok_and(|b| b.node() == node))
                .ok_or("missing canonical input")?;
            bs.push(WireBox::from_record(record.clone())?);
        }
        let subject = bound_self(claim, &bs)?;
        if subject.id()? != witness.subject_input_id {
            return Err("wrong witness subject input".into());
        }
        let creation = subject.node().candidate.creation_height;
        let height = domain.block_context.height;
        if domain.block_context.activated_script_version != 3
            || height < creation
            || height - creation >= domain.parameters.storage_period
            || bs
                .iter()
                .any(|b| b.node().candidate.tokens.iter().any(|t| t.amount == 0))
        {
            return Err("execution outside root-evaluation/positive-token premises".into());
        }
        for root in &p.authentication_roots {
            let fixed = WireBox::from_record(root.clone())?;
            if !records
                .iter()
                .any(|r| WireBox::from_record(r.clone()).is_ok_and(|b| b.node() == fixed.node()))
            {
                return Err("missing fixed authentication root".into());
            }
        }
        if !valid_guard(&p.guard, 0)
            || guard(&p.guard, Some(height), subject.node().candidate.value) != Some(true)
        {
            return Err("guard unknown or false".into());
        }
        if !omitted(claim, &bs, &subject)? {
            return Err("distinct matching spending companion is present".into());
        }
        Ok(())
    })();
    assessment.map_err(
        |reason| json!({"status":"unresolved","reason":reason,"execution":accepted.report()}),
    )?;
    let established = propose(claim).and_then(|d| check(claim, &d)).ok();
    Ok(Refutation {
        report: json!({"version":"required-relations:v1","status":"refuted","claimDigest":claim.claim_digest(),"premiseDigest":claim.premises.digest(),"claim":claim,"witness":witness,"execution":accepted.report(),"conflictWithEstablished":established.is_some(),"establishedArtifact":established.as_ref().map(|e|e.report()),"releaseDecision":if established.is_some(){"stop-unsound-relation"}else{"no-established-conflict"},"scope":"accepted supplied-state omission; not automatically a vulnerability"}),
    })
}
