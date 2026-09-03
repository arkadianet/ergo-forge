//! The composer: a contract as a list of **spending paths**, each "who may
//! spend" (a key, any/all/k-of-n keys, or anyone) plus "under what
//! conditions" (after/before a height, pay someone, keep funds here, an
//! oracle price). The paths are OR-ed; a path's conditions are AND-ed.
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
    /// An output paying at least `$amount` to `$key`.
    PayTo { key: String, amount: String },
    /// An output keeping at least `$at_least` under this same contract.
    KeepHere {
        #[serde(rename = "atLeast")]
        at_least: String,
    },
    /// Data input 0 carries `$nft` and reports R4 >= `$floor`.
    OracleAbove { nft: String, floor: String },
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
    #[error("parameter `{0}`: {1}")]
    Value(String, String),
}

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

/// Conditions as a boolean, with output slots allocated in clause order.
fn conditions_source(conds: &[Condition]) -> Option<String> {
    let mut slot = 0usize;
    let parts: Vec<String> = conds
        .iter()
        .map(|c| match c {
            Condition::After(h) => format!("HEIGHT >= ${h}"),
            Condition::Before(h) => format!("HEIGHT < ${h}"),
            Condition::PayTo { key, amount } => {
                let i = slot;
                slot += 1;
                format!("(OUTPUTS.size > {i} && OUTPUTS({i}).propositionBytes == ${key}.propBytes && OUTPUTS({i}).value >= ${amount})")
            }
            Condition::KeepHere { at_least } => {
                let i = slot;
                slot += 1;
                format!("(OUTPUTS.size > {i} && OUTPUTS({i}).propositionBytes == SELF.propositionBytes && OUTPUTS({i}).value >= ${at_least})")
            }
            Condition::OracleAbove { nft, floor } => format!(
                "(CONTEXT.dataInputs.size > 0 && CONTEXT.dataInputs(0).tokens.size > 0 && CONTEXT.dataInputs(0).tokens(0)._1 == ${nft} && CONTEXT.dataInputs(0).R4[Long].get >= ${floor})"
            ),
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" &&\n              "))
    }
}

/// Parameters the spec needs, in first-use order, with types.
fn params_of(spec: &Spec) -> Vec<ParamNeed> {
    let mut out: Vec<ParamNeed> = Vec::new();
    let mut push = |name: &str, tpe: &str, desc: &str| {
        if !out.iter().any(|p| p.name == name) {
            out.push(ParamNeed {
                name: name.to_string(),
                type_hint: Some(tpe.to_string()),
                default: None,
                description: Some(desc.to_string()),
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
                        &format!("Address for \"{k}\" — An Ergo address from a wallet."),
                    );
                }
            }
        }
        for c in &p.conditions {
            match c {
                Condition::After(h) => push(h, "Int", &format!("From when? (\"{h}\") — Spending on this path is possible from this date.")),
                Condition::Before(h) => push(h, "Int", &format!("Until when? (\"{h}\") — Spending on this path stops at this date.")),
                Condition::PayTo { key, amount } => {
                    push(key, "SigmaProp", &format!("Who is paid (\"{key}\")? — Their Ergo address."));
                    push(amount, "Long", &format!("How much must be paid (\"{amount}\"), in nanoERG — 1 ERG = 1,000,000,000 nanoERG."));
                }
                Condition::KeepHere { at_least } => push(at_least, "Long", &format!("How much must stay in the contract (\"{at_least}\"), in nanoERG — 1 ERG = 1,000,000,000 nanoERG.")),
                Condition::OracleAbove { nft, floor } => {
                    push(nft, "Coll[Byte]", &format!("Which oracle (\"{nft}\")? — The token id that identifies the oracle box."));
                    push(floor, "Long", &format!("Minimum price (\"{floor}\") — In the oracle's own units."));
                }
            }
        }
    }
    out
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

// ── the model, and the generated suite ─────────────────────────────────────

/// A concrete scenario the model reasons about.
#[derive(Clone, Default)]
struct World {
    height: i64,
    /// Output i → (tree hex or "$self", value)
    outputs: Vec<(String, i64)>,
    /// Data input 0 → (token id, R4 price)
    oracle: Option<(String, i64)>,
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

/// Does this world satisfy the path's conditions (ignoring keys)?
fn satisfied(
    p: &Path,
    w: &World,
    values: &BTreeMap<String, TypedValue>,
) -> Result<bool, ComposeError> {
    let mut slot = 0usize;
    for c in &p.conditions {
        let ok = match c {
            Condition::After(h) => w.height >= int_of(values, h)?,
            Condition::Before(h) => w.height < int_of(values, h)?,
            Condition::PayTo { key, amount } => {
                let i = slot;
                slot += 1;
                let tree = key_tree(values, key)?;
                w.outputs
                    .get(i)
                    .map(|(t, v)| *t == tree && *v >= int_of(values, amount).unwrap_or(i64::MAX))
                    .unwrap_or(false)
            }
            Condition::KeepHere { at_least } => {
                let i = slot;
                slot += 1;
                w.outputs
                    .get(i)
                    .map(|(t, v)| {
                        t == "$self" && *v >= int_of(values, at_least).unwrap_or(i64::MAX)
                    })
                    .unwrap_or(false)
            }
            Condition::OracleAbove { nft, floor } => w
                .oracle
                .as_ref()
                .map(|(id, price)| {
                    *id == str_of(values, nft).unwrap_or_default()
                        && *price >= int_of(values, floor).unwrap_or(i64::MAX)
                })
                .unwrap_or(false),
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A world that satisfies every condition of `p`.
fn satisfying_world(
    p: &Path,
    values: &BTreeMap<String, TypedValue>,
) -> Result<World, ComposeError> {
    let mut w = World::default();
    let mut lo: Option<i64> = None;
    let mut hi: Option<i64> = None;
    for c in &p.conditions {
        match c {
            Condition::After(h) => {
                lo = Some(lo.map_or(int_of(values, h)?, |l| {
                    l.max(int_of(values, h).unwrap_or(l))
                }))
            }
            Condition::Before(h) => {
                hi = Some(hi.map_or(int_of(values, h)?, |x| {
                    x.min(int_of(values, h).unwrap_or(x))
                }))
            }
            Condition::PayTo { key, amount } => w
                .outputs
                .push((key_tree(values, key)?, int_of(values, amount)?)),
            Condition::KeepHere { at_least } => {
                w.outputs.push(("$self".into(), int_of(values, at_least)?))
            }
            Condition::OracleAbove { nft, floor } => {
                w.oracle = Some((str_of(values, nft)?, int_of(values, floor)?))
            }
        }
    }
    w.height = match (lo, hi) {
        (Some(l), Some(h)) => l.max(1).min(h - 1),
        (Some(l), None) => l.max(1),
        (None, Some(h)) => (h - 1).max(1),
        (None, None) => 1,
    };
    Ok(w)
}

/// The world with condition `k` of path `p` violated (others as satisfied).
fn violating_world(
    p: &Path,
    k: usize,
    values: &BTreeMap<String, TypedValue>,
) -> Result<World, ComposeError> {
    let mut w = satisfying_world(p, values)?;
    let mut slot = 0usize;
    for (i, c) in p.conditions.iter().enumerate() {
        let my_slot = slot;
        if matches!(c, Condition::PayTo { .. } | Condition::KeepHere { .. }) {
            slot += 1;
        }
        if i != k {
            continue;
        }
        match c {
            Condition::After(h) => w.height = (int_of(values, h)? - 1).max(1),
            Condition::Before(h) => w.height = int_of(values, h)?,
            Condition::PayTo { .. } | Condition::KeepHere { .. } => {
                if let Some(o) = w.outputs.get_mut(my_slot) {
                    o.1 -= 1;
                }
            }
            Condition::OracleAbove { floor, .. } => {
                if let Some(o) = w.oracle.as_mut() {
                    o.1 = int_of(values, floor)? - 1;
                }
            }
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

fn scenario_json(
    w: &World,
    values: &BTreeMap<String, TypedValue>,
    oracle_tree: &str,
) -> serde_json::Value {
    let _ = values;
    let outputs: Vec<serde_json::Value> = w
        .outputs
        .iter()
        .map(|(t, v)| serde_json::json!({ "value": v, "ergoTree": t }))
        .collect();
    let mut sc = serde_json::json!({ "height": w.height, "selfBox": { "value": 1_000_000_000i64 }, "outputs": outputs });
    if let Some((id, price)) = &w.oracle {
        sc["dataInputs"] = serde_json::json!([{ "value": 1, "ergoTree": oracle_tree, "tokens": [{ "id": id, "amount": 1 }],
                                                 "registers": { "R4": { "type": "Long", "value": price } } }]);
    }
    sc
}

fn generate_suite(
    spec: &Spec,
    source: &str,
    values: &BTreeMap<String, TypedValue>,
) -> Result<Suite, ComposeError> {
    // Any valid tree does for the oracle box's own script.
    let oracle_tree = "10010101d17300";
    let mut scenarios = Vec::new();
    let mut add = |name: String, w: &World| -> Result<(), ComposeError> {
        let (expect, want, excl) = expectation(spec, w, values)?;
        let mut case = scenario_json(w, values, oracle_tree);
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
        let w = satisfying_world(p, values)?;
        add(format!("{}: every condition met", p.name), &w)?;
        for (k, c) in p.conditions.iter().enumerate() {
            let w = violating_world(p, k, values)?;
            let what = match c {
                Condition::After(_) => "one block before its start date".to_string(),
                Condition::Before(_) => "at its end date".to_string(),
                Condition::PayTo { .. } => "paying one nanoERG too little".to_string(),
                Condition::KeepHere { .. } => "keeping one nanoERG too little here".to_string(),
                Condition::OracleAbove { .. } => "oracle price one below the floor".to_string(),
            };
            add(format!("{}: {what}", p.name), &w)?;
        }
    }
    if spec.paths.iter().any(|p| !p.conditions.is_empty()) {
        add(
            "baseline: no conditions met, no outputs".into(),
            &World {
                height: 1,
                ..Default::default()
            },
        )?;
    }
    let doc = serde_json::json!({ "source": source, "params": values, "scenarios": scenarios });
    serde_json::from_value(doc).map_err(|e| ComposeError::Value("suite".into(), e.to_string()))
}
