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
/// `sigmaProp(true)`.
const PASS_TREE: &str = "10010101d17300";

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

/// The mechanical witness gate, mirroring the hunt's own gate in `drain.rs`:
/// a witness counts as a KEYLESS drain only when every input's verdict is
/// `pass`, or `needsProof` on an input the attacker owns (the attacker signs
/// their own boxes). `needsProof` anywhere else means the residual requires
/// a key — a keyed-insider witness — and cannot enter the denominator. This
/// makes `keyed-insider` a classification the harness DERIVES, not a
/// judgment someone remembered to make.
fn witness_is_keyless(w: &Value, check: &ergo_sandbox::txcheck::TxCheck) -> Result<(), String> {
    let attacker: Vec<usize> = w
        .get("attackerInputs")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_u64().map(|n| n as usize))
                .collect()
        })
        .unwrap_or_default();
    for ic in &check.inputs {
        let ok = match ic.verdict {
            "pass" => true,
            "needsProof" => attacker.contains(&ic.index),
            _ => false,
        };
        if !ok {
            return Err(if ic.verdict == "needsProof" {
                format!(
                    "input {} needsProof on a non-attacker input: the residual requires a key",
                    ic.index
                )
            } else {
                format!(
                    "input {} verdict `{}`: the witness input did not pass",
                    ic.index, ic.verdict
                )
            });
        }
    }
    Ok(())
}

/// A proven mutant's witness must validate AND pass the keyless gate before
/// the hunt runs. Every mutant's witness (when present) is classified
/// mechanically: `keyless` or `keyed-insider`. The corpus's `proven` flag
/// must agree with the mechanical classification — a misproof cannot enter
/// by accident, and the M1-M3 reclassification is pinned by assertion.
fn witnesses_validate(corpus: &Value) {
    for m in corpus["mutants"].as_array().unwrap() {
        let Some(w) = m.get("witnessTx") else {
            if m["proven"].as_bool() == Some(true) {
                panic!("{}: proven mutant carries no witnessTx", m["id"]);
            }
            continue;
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
        let keyless = witness_is_keyless(w, &c);
        let proven = m["proven"].as_bool() == Some(true);
        match &keyless {
            Ok(()) => assert!(
                proven,
                "{}: the witness is keyless but the mutant is not marked proven — reclassify",
                m["id"]
            ),
            Err(reason) => {
                // The mechanical rule reproduces the hand reclassification.
                assert_eq!(
                    m["attackerModel"].as_str(),
                    Some("keyed-insider"),
                    "{}: witness is keyed ({}), so the mutant must be classified keyed-insider",
                    m["id"],
                    reason
                );
                eprintln!(
                    "{}: classified keyed-insider mechanically ({})",
                    m["id"], reason
                );
            }
        }
        if proven {
            assert!(
                keyless.is_ok(),
                "{} is marked proven but the mechanical gate rejects its witness: {:?}",
                m["id"],
                keyless
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

/// The must-find control (harness validity, NOT a corpus mutant): the
/// `delete-nft-check` operator applied to the incident's own FIXED contract
/// reconstructs the deployed vulnerable `useLpSwap` — the contract that
/// drained 284,695 ERG on mainnet at height 1,868,204. Phase 1 rediscovers
/// it today from the honest shape (permutation [2,1,0], drain payout, the
/// incident's extraction). Ground truth is certain, so this is a pass/fail
/// on the apparatus: if it ever stops being found, the harness or the hunt
/// regressed. It never enters the detection-rate denominator — if it did,
/// the rate would move without the hunt changing.
fn known_detectable_control_is_found(corpus: &Value) {
    let control = &corpus["knownDetectableControl"];
    let find = control["diff"]["find"].as_str().unwrap();
    let replace = control["diff"]["replace"].as_str().unwrap();
    let original = control["originalSource"].as_str().unwrap();
    // One replacement: both halves of the NFT binding (input-side and
    // successor-side) go at once.
    let vulnerable = original.replacen(find, replace, 1);
    assert_ne!(vulnerable, original, "the control's diff must apply");
    let params: BTreeMap<String, TypedValue> = control["params"]
        .as_object()
        .expect("control params")
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
        .collect();
    let vulnerable_tree = hex::encode(
        ergo_sandbox::compile::compile_with_params(
            &vulnerable,
            &params,
            3,
            ergo_ser::address::NetworkPrefix::Mainnet,
        )
        .expect("the deployed-vulnerable contract compiles")
        .tree_bytes,
    );
    // The pool box needs a trivially-passing tree that is NOT the attacker's
    // default `sigmaProp(true)` hex: drain mode re-trees the free payee to
    // the attacker tree, and if that equals a protected tree the free payee
    // counts as a script-matched successor and shields the whole drain (leak
    // = 0). Distinct script, same semantics.
    let pool_tree = hex::encode(
        ergo_sandbox::compile::compile_source(
            "sigmaProp(HEIGHT >= 0)",
            3,
            ergo_ser::address::NetworkPrefix::Mainnet,
        )
        .expect("pool tree compiles")
        .tree_bytes,
    );

    // The honest fill shape (the phase-1 test's template): the pool
    // protected, the (vulnerable) swap companion, the attacker's own box.
    let request: DrainRequest = serde_json::from_value(json!({
        "inputs": [
            { "role": "protected", "value": 1000000000i64, "ergoTree": &pool_tree,
              "tokens": [
                { "id": control["params"]["lpNft"]["value"], "amount": 1i64 },
                { "id": "804a66426283b8281240df8f9de783651986f20ad6391a71b26b9e7d6faad099", "amount": 1000000i64 },
                { "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 5000000000i64 }] },
            { "role": "companion", "value": 1000000i64, "ergoTree": &vulnerable_tree,
              "tokens": [{ "id": control["swapNft"]["value"], "amount": 1i64 }] },
            { "role": "attacker", "value": 2000000i64, "ergoTree": PASS_TREE },
        ],
        "outputs": [
            { "payee": "fixed", "value": 1000000000i64, "ergoTree": &pool_tree,
              "tokens": [
                { "id": control["params"]["lpNft"]["value"], "amount": 1i64 },
                { "id": "804a66426283b8281240df8f9de783651986f20ad6391a71b26b9e7d6faad099", "amount": 1000000i64 },
                { "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "amount": 5000000000i64 }] },
            // The swap successor MUST carry the vulnerable swap tree AND
            // the swap NFT — the contract checks swapSucc.propositionBytes
            // == SELF and swapSucc.tokens == SELF.tokens.
            { "payee": "fixed", "value": 2000000i64, "ergoTree": &vulnerable_tree,
              "tokens": [{ "id": control["swapNft"]["value"], "amount": 1i64 }] },
            { "payee": "free", "value": 0, "ergoTree": PASS_TREE, "tokens": [] },
            { "payee": "fixed", "value": 2000000i64, "ergoTree": PASS_TREE, "tokens": [] },
        ],
        "protocolNfts": [control["params"]["lpNft"]["value"]],
        "height": 1868204u32,
        "network": "mainnet",
    }))
    .expect("control request builds");
    let report = ergo_sandbox::decompile::with_large_stack(move || drain_hunt(&request))
        .expect("control hunt runs");
    assert_eq!(
        verdict_name(&report.verdict),
        "drainable",
        "the known-detectable control was NOT found — probes {} notes {:?} shapes {:?}",
        report.probes_run,
        report.notes,
        report
            .synthesis
            .shapes
            .iter()
            .map(|t| (t.shape.clone(), t.run))
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_corpus_is_measured_and_does_not_regress() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
    witnesses_validate(&corpus);
    let key: Value = serde_json::from_str(ANSWER_KEY).expect("answer key parses");
    let key_escalated: Vec<String> = key["escalatedFindings"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| f["mutant"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    // ── one pass over the corpus: mutants and their negative controls ──
    let mut results: BTreeMap<String, Value> = BTreeMap::new();
    let mut controls: BTreeMap<String, Value> = BTreeMap::new();
    // `attributable` is the headline: a hit counts only when the mutant's own
    // negative control is clean in the SAME configuration. A hit that fires on
    // the unmutated original too says nothing about the mutation — the whole
    // reason the control exists — so it is counted separately, never in the
    // rate. Derived here rather than flagged by hand: the last hand-held
    // guarantee in this harness (the keyless proof) was already promoted to a
    // mechanical check for the same reason.
    let mut attributable = 0usize;
    let mut confounded: Vec<String> = Vec::new();
    let mut raw_drainable = 0usize;
    let mut proven = 0usize;
    for m in corpus["mutants"].as_array().unwrap() {
        let id = m["id"].as_str().unwrap().to_string();
        let off = run_one(m, false);
        let on = run_one(m, true);
        // Negative control: the UNMUTATED original, both configurations,
        // asserted and recorded. A drainable original is either a real
        // finding in a contract we ship as an example (escalate — record it
        // in the answer key's escalatedFindings; do not edit the fixture)
        // or a harness bug. Escalations must be pre-recorded: an
        // unrecorded drainable original fails the run.
        let mut o = m.clone();
        o["mutatedSource"] = m["originalSource"].clone();
        let ctrl_off = run_one(&o, false);
        let ctrl_on = run_one(&o, true);
        let escalated = key_escalated.contains(&id);
        for (cfg, r) in [("synthesisOff", &ctrl_off), ("synthesisOn", &ctrl_on)] {
            let v = r["verdict"].as_str().unwrap();
            if v == "drainable" {
                assert!(
                    escalated,
                    "{}: the UNMUTATED original reported drainable ({}) and is NOT recorded \
                     in the answer key's escalatedFindings — escalate it there (real finding \
                     in a shipped example) or fix the harness; do not edit the fixture",
                    id, cfg
                );
            } else {
                assert_eq!(
                    v, "notunderprobes",
                    "{}: unexpected original verdict {} ({})",
                    id, v, cfg
                );
            }
        }
        // ── attribution: a hit counts only against a clean control ──
        let hit_off = off["verdict"] == "drainable";
        let hit_on = on["verdict"] == "drainable";
        let ctrl_hit_off = ctrl_off["verdict"] == "drainable";
        let ctrl_hit_on = ctrl_on["verdict"] == "drainable";
        let is_attributable = (hit_off && !ctrl_hit_off) || (hit_on && !ctrl_hit_on);
        let is_confounded = (hit_off || hit_on) && !is_attributable;
        if m["proven"].as_bool() == Some(true) {
            proven += 1;
            if hit_off || hit_on {
                raw_drainable += 1;
            }
            if is_attributable {
                attributable += 1;
            }
            if is_confounded {
                confounded.push(id.clone());
            }
        }
        controls.insert(
            id.clone(),
            json!({ "synthesisOff": ctrl_off, "synthesisOn": ctrl_on }),
        );

        eprintln!(
            "{}: off[{} probes={} capped={}] on[{} probes={} capped={}]",
            id,
            off["verdict"],
            off["probesRun"],
            off["capped"],
            on["verdict"],
            on["probesRun"],
            on["capped"],
        );
        results.insert(
            id,
            json!({
                "proven": m["proven"],
                "synthesisOff": off,
                "synthesisOn": on,
                "attributable": is_attributable,
                "confounded": is_confounded,
            }),
        );
    }
    let rate = if proven == 0 {
        0.0
    } else {
        attributable as f64 / proven as f64
    };
    println!(
        "DETECTION RATE (attributable): {attributable}/{proven} = {rate:.2}\n\
         \traw drainable verdicts on proven mutants: {raw_drainable}\n\
         \tconfounded (drainable, but the unmutated control is drainable too): {confounded:?}"
    );

    // ── the must-find control: a pass/fail on the apparatus ──
    known_detectable_control_is_found(&corpus);

    // ── the answer key: no regression against the recorded run ──
    let key_rate = key["detectionRate"].as_f64().expect("recorded rate");
    assert!(
        rate + 1e-9 >= key_rate,
        "the attributable detection rate regressed: {rate} < recorded {key_rate}"
    );
    // The confounded set is part of the meaning of the rate, not a footnote:
    // a mutant becoming confounded (or ceasing to be) changes what the number
    // says, so it is pinned exactly rather than bounded.
    let key_confounded: Vec<String> = key["confounded"]
        .as_array()
        .expect("recorded confounded set")
        .iter()
        .map(|v| v.as_str().expect("confounded id").to_string())
        .collect();
    assert_eq!(
        confounded, key_confounded,
        "the confounded set changed: a drainable verdict is attributable to the mutation \
         only when the unmutated control is clean in the same configuration"
    );
    assert_eq!(
        raw_drainable,
        key["rawDrainableOnProven"]
            .as_u64()
            .expect("recorded raw drainable count") as usize,
        "raw drainable count on proven mutants changed"
    );
    for kr in key["perMutant"].as_array().expect("per-mutant records") {
        let id = kr["id"].as_str().unwrap();
        let r = results
            .get(id)
            .unwrap_or_else(|| panic!("{id} missing from the run"));
        assert_eq!(
            r["synthesisOff"]["verdict"].as_str().unwrap(),
            kr["synthesisOff"]["verdict"].as_str().unwrap(),
            "{id}: synthesis-off verdict regressed"
        );
        assert_eq!(
            r["synthesisOn"]["verdict"].as_str().unwrap(),
            kr["synthesisOn"]["verdict"].as_str().unwrap(),
            "{id}: synthesis-on verdict regressed"
        );
        if let Some(recorded) = kr["foundConfounded"].as_bool() {
            assert_eq!(
                r["confounded"].as_bool().unwrap(),
                recorded,
                "{id}: the DERIVED confounding disagrees with the recorded flag"
            );
        }
    }
    // The negative controls are recorded too — their section in the key
    // must exist and every recorded verdict must stay notUnderProbes.
    for kr in key["negativeControls"]
        .as_array()
        .expect("recorded controls")
    {
        let id = kr["id"].as_str().unwrap();
        // The CONTROL's results, not the mutant's — these are different runs,
        // and comparing the mutant against the control's record passed only by
        // coincidence (both sides were `notunderprobes` on six of seven, and
        // both `drainable` on the seventh).
        let c = controls
            .get(id)
            .unwrap_or_else(|| panic!("{id}: no control result"));
        assert_eq!(
            c["synthesisOff"]["verdict"].as_str().unwrap(),
            kr["synthesisOff"]["verdict"].as_str().unwrap(),
            "{id}: negative-control verdict changed (synthesis off)"
        );
        assert_eq!(
            c["synthesisOn"]["verdict"].as_str().unwrap(),
            kr["synthesisOn"]["verdict"].as_str().unwrap(),
            "{id}: negative-control verdict changed (synthesis on)"
        );
    }
}
