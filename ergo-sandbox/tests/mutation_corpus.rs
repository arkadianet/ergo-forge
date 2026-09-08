//! The mutation corpus harness (roadmap phase 2.5,
//! `docs/superpowers/specs/2026-09-08-mutation-corpus-design.md`).
//!
//! Runs every mutant in `examples/mutants/mutants.json` through the drain
//! hunt in two configurations — synthesis off, synthesis fully on — after
//! validating every proven mutant's witness against `txcheck::check` (a
//! proven mutant whose witness does not validate is a harness bug). The
//! detection rate is reported over the **proven** denominator only; the
//! recorded answer key (`answer-key.json`) is asserted as **no regression**
//! — recorded verdicts must not degrade — never against an absolute floor.

use std::collections::BTreeMap;
use std::time::Instant;

use ergo_sandbox::drain::{drain_hunt, DrainRequest, DrainVerdict};
use ergo_sandbox::TypedValue;
use serde_json::{json, Value};

const CORPUS: &str = include_str!("../../examples/mutants/mutants.json");
const ANSWER_KEY: &str = include_str!("../../examples/mutants/answer-key.json");

/// The corpus's seller/receiver key tree (`028333f9…` P2PK).
const P2PK: &str = "0008cd028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197";

fn label_placeholder() -> String {
    "<extra tree>".to_string()
}

fn compile_tree(source: &str, params: &Value) -> String {
    compile_tree_labeled(source, params, "<unknown>")
}

fn compile_tree_labeled(source: &str, params: &Value, label: &str) -> String {
    let p: BTreeMap<String, TypedValue> = match params {
        Value::Null => BTreeMap::new(),
        v => v
            .as_object()
            .expect("params object")
            .iter()
            .map(|(k, tv)| {
                (
                    k.clone(),
                    TypedValue {
                        r#type: tv["type"].as_str().expect("type").to_string(),
                        value: tv["value"].clone(),
                    },
                )
            })
            .collect(),
    };
    hex::encode(
        ergo_sandbox::compile::compile_with_params(
            source,
            &p,
            3,
            ergo_ser::address::NetworkPrefix::Mainnet,
        )
        .unwrap_or_else(|e| panic!("{label}: mutant source does not compile: {e:?}"))
        .tree_bytes,
    )
}

/// Resolve a template tree reference: `$mutant` → the mutated tree,
/// `$<name>` → a named extra tree (from `extraTrees`), `p2pk-*` → the key
/// tree, otherwise a literal hex tree.
fn resolve_tree(
    reference: &str,
    mutant_tree: &str,
    extra_trees: &serde_json::Map<String, Value>,
) -> String {
    match reference {
        "$mutant" => mutant_tree.to_string(),
        "p2pk-seller" | "p2pk-receiver" => P2PK.to_string(),
        other => {
            if let Some(name) = other.strip_prefix('$') {
                if let Some(source) = extra_trees.get(name) {
                    let params = extra_trees
                        .get(&format!("{name}-params"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    return compile_tree_labeled(
                        source.as_str().expect("extra tree source"),
                        &params,
                        &label_placeholder(),
                    );
                }
            }
            other.to_string()
        }
    }
}

/// A template box in node shape, placeholders resolved.
fn box_json(tpl: &Value, mutant_tree: &str, extra_trees: &serde_json::Map<String, Value>) -> Value {
    let tree_ref = tpl["ergoTree"].as_str().expect("template box names a tree");
    // Node shape for txcheck (`assets[].tokenId`), plus the scenario shape
    // (`tokens[].id`) carried alongside — DrainRequest and txcheck read
    // different field names, and the fixtures name tokens once.
    let assets: Vec<Value> = tpl
        .get("tokens")
        .and_then(|t| t.as_array())
        .map(|a| {
            a.iter()
                .map(|t| {
                    json!({
                        "tokenId": t["id"],
                        "id": t["id"],
                        "amount": t["amount"],
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mut o = json!({
        "value": tpl["value"],
        "ergoTree": resolve_tree(tree_ref, mutant_tree, extra_trees),
        "creationHeight": tpl.get("creationHeight").cloned().unwrap_or(json!(1)),
        "assets": assets,
        "tokens": tpl.get("tokens").cloned().unwrap_or(json!([])),
    });
    if let Some(regs) = tpl
        .get("registers")
        .or_else(|| tpl.get("additionalRegisters"))
    {
        o["additionalRegisters"] = regs.clone();
    }
    if let Some(role) = tpl.get("roleRef").and_then(|r| r.as_str()) {
        // Data inputs (oracles) take no role; every hunted input does.
        o["role"] = json!(role);
    }
    if let Some(payee) = tpl.get("roleRef").and_then(|r| r.as_str()) {
        if payee.starts_with("free") {
            o["payee"] = json!("free");
        }
    }
    o
}

/// Build the hunt request from a mutant record. The template is the
/// protocol's HONEST shape; `protocolNfts` is derived from the protected
/// inputs' tokens(0) — the caller's claim, and nothing more.
fn build_request(m: &Value, synthesis_on: bool) -> DrainRequest {
    let extra: serde_json::Map<String, Value> = m
        .get("extraTrees")
        .and_then(|e| e.as_object())
        .cloned()
        .unwrap_or_default();
    let mutant_tree = compile_tree(m["mutatedSource"].as_str().unwrap(), &m["params"]);

    let inputs: Vec<Value> = m["template"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| box_json(b, &mutant_tree, &extra))
        .collect();
    let data_inputs: Vec<Value> = m["template"]
        .get("dataInputs")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|b| box_json(b, &mutant_tree, &extra))
                .collect()
        })
        .unwrap_or_default();
    let outputs: Vec<Value> = m["template"]["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| box_json(b, &mutant_tree, &extra))
        .collect();

    let mut protocol_nfts: Vec<Value> = Vec::new();
    for i in m["template"]["inputs"].as_array().unwrap() {
        if i["roleRef"].as_str() == Some("protected") {
            if let Some(first) = i["tokens"].as_array().and_then(|a| a.first()) {
                let id = first["id"].as_str().unwrap().to_string();
                if !protocol_nfts.iter().any(|v| v == &id) {
                    protocol_nfts.push(Value::String(id));
                }
            }
        }
    }

    let caps = &m["caps"];
    let synthesis = if synthesis_on {
        json!({
            "maxNewOutputs": 2, "companionRecreations": true, "successorStates": true,
            "splits": true, "mints": true, "permuteOutputs": true,
            "maxOutputPermutations": 24
        })
    } else {
        json!({})
    };
    let text = json!({
        "inputs": inputs,
        "dataInputs": data_inputs,
        "outputs": outputs,
        "protocolNfts": protocol_nfts,
        "height": m["template"]["height"],
        "network": "mainnet",
        "maxPermutations": caps["maxPermutations"],
        "maxProbes": caps["maxProbes"],
        "synthesis": synthesis,
    });
    serde_json::from_value(text).expect("request builds")
}

/// A proven mutant's witness must validate before the hunt runs — the
/// denominator is only honest if every entry is a real keyless drain.
/// Witness boxes name the same tree references as templates (`$mutant`,
/// `$<extra>`, `p2pk-*`, literal hex), resolved identically.
fn witnesses_validate(corpus: &Value) {
    for m in corpus["mutants"].as_array().unwrap() {
        if m["proven"].as_bool() != Some(true) {
            continue;
        }
        let Some(w) = m.get("witnessTx") else {
            panic!("{}: proven mutant carries no witnessTx", m["id"]);
        };
        let extra: serde_json::Map<String, Value> = m
            .get("extraTrees")
            .and_then(|e| e.as_object())
            .cloned()
            .unwrap_or_default();
        let mutant_tree = compile_tree_labeled(
            m["mutatedSource"].as_str().unwrap(),
            &m["params"],
            m["id"].as_str().unwrap(),
        );
        let wit_box = |b: &Value| -> Value { box_json(b, &mutant_tree, &extra) };
        let data_inputs_vec: Vec<Value> = w
            .get("dataInputs")
            .and_then(|d| d.as_array())
            .map(|dis| {
                dis.iter()
                    .enumerate()
                    .map(|(i, _b)| {
                        // Distinct, hex, and disjoint from the input ids.
                        json!({"boxId": format!("f{:0>63x}", i + 1), "extension": {}})
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut boxes_vec: Vec<Value> = w["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let mut bj = wit_box(b);
                bj["boxId"] = json!(format!("{:0>64}", i + 1));
                bj
            })
            .collect();
        if let Some(dis) = w.get("dataInputs").and_then(|d: &Value| d.as_array()) {
            for (i, b) in dis.iter().enumerate() {
                let mut bj = wit_box(b);
                bj["boxId"] = json!(format!("f{:0>63x}", i + 1));
                boxes_vec.push(bj);
            }
        }
        let req: ergo_sandbox::txcheck::TxRequest = serde_json::from_value(json!({
            "tx": {
                "inputs": w["inputs"].as_array().unwrap().iter().enumerate()
                    .map(|(i, _b)| json!({"boxId": format!("{:0>64}", i + 1), "extension": {}}))
                    .collect::<Vec<_>>(),
                "dataInputs": data_inputs_vec,
                "outputs": w["outputs"].as_array().unwrap().iter()
                    .map(wit_box).collect::<Vec<_>>(),
            },
            "boxes": boxes_vec,
            "height": w["height"],
        }))
        .expect("witness request builds");
        let c = ergo_sandbox::txcheck::check(&req).expect("witness checks");
        if !c.valid {
            panic!(
                "{} witness must validate: {:?} — input verdicts: {:?}",
                m["id"],
                c.problems,
                c.inputs
                    .iter()
                    .map(|i| (i.index, i.verdict, i.error.clone()))
                    .collect::<Vec<_>>()
            );
        }
    }
}

fn verdict_name(v: &DrainVerdict) -> &'static str {
    match v {
        DrainVerdict::Drainable => "drainable",
        DrainVerdict::NotUnderProbes => "notunderprobes",
        DrainVerdict::InvalidShape => "invalidshape",
    }
}

fn run_one(m: &Value, synthesis_on: bool) -> Value {
    let req = build_request(m, synthesis_on);
    let t = Instant::now();
    let report =
        ergo_sandbox::decompile::with_large_stack(move || drain_hunt(&req)).expect("hunt runs");
    if matches!(report.verdict, DrainVerdict::InvalidShape) {
        eprintln!("{}: shape errors: {:?}", m["id"], report.notes);
    }
    json!({
        "verdict": verdict_name(&report.verdict),
        "probesRun": report.probes_run,
        "capped": report.capped,
        "hits": report.hits,
        "wallMs": t.elapsed().as_millis() as u64,
    })
}

#[test]
fn the_corpus_is_measured_and_does_not_regress() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
    witnesses_validate(&corpus);

    let mut records = Vec::new();
    let mut found = 0usize;
    let mut proven = 0usize;
    for m in corpus["mutants"].as_array().unwrap() {
        let off = run_one(m, false);
        let on = run_one(m, true);
        if m["proven"].as_bool() == Some(true) {
            proven += 1;
            // A proven mutant is "found" when the hunt reports drainable.
            let hit = off["verdict"] == "drainable" || on["verdict"] == "drainable";
            if hit {
                found += 1;
            }
        }
        records.push(json!({
            "id": m["id"],
            "proven": m["proven"],
            "synthesisOff": off,
            "synthesisOn": on,
        }));
        eprintln!(
            "{}: off[{} probes={} capped={}] on[{} probes={} capped={}]",
            m["id"],
            off["verdict"],
            off["probesRun"],
            off["capped"],
            on["verdict"],
            on["probesRun"],
            on["capped"],
        );
    }
    let rate = if proven == 0 {
        0.0
    } else {
        found as f64 / proven as f64
    };
    println!("DETECTION RATE: {found}/{proven} = {rate:.2} (proven mutants only)");

    // ── the answer key: no regression against the recorded run ──
    let key: Value = serde_json::from_str(ANSWER_KEY).expect("answer key parses");
    let key_rate = key["detectionRate"].as_f64().expect("recorded rate");
    assert!(
        rate + 1e-9 >= key_rate,
        "the detection rate regressed: {rate} < recorded {key_rate}"
    );
    for kr in key["perMutant"].as_array().expect("per-mutant records") {
        let id = kr["id"].as_str().unwrap();
        let m = corpus["mutants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == id)
            .unwrap_or_else(|| panic!("{id} missing from corpus"));
        let off = run_one(m, false);
        let on = run_one(m, true);
        let recorded_off = kr["synthesisOff"]["verdict"].as_str().unwrap();
        let recorded_on = kr["synthesisOn"]["verdict"].as_str().unwrap();
        assert_eq!(
            off["verdict"].as_str().unwrap(),
            recorded_off,
            "{id}: synthesis-off verdict regressed"
        );
        assert_eq!(
            on["verdict"].as_str().unwrap(),
            recorded_on,
            "{id}: synthesis-on verdict regressed"
        );
    }
}
