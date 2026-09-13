use ergo_sandbox::{
    evidence::{
        validate::{validate, ValidationRequest},
        Premise,
    },
    properties::{
        evaluate::evaluate,
        schema::Declaration,
        trace::{digest, schedule_step, SuppliedTrace},
    },
};
use serde_json::{json, Value};
fn fixture(id: &str) -> SuppliedTrace {
    let c: Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/tests/fixtures/properties/cases/{id}.fixture",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    let steps: Vec<ValidationRequest> = c["references"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| serde_json::from_value(r["request"].clone()).unwrap())
        .collect();
    SuppliedTrace {
        root: steps[0].case.premises().boxes.value().unwrap().clone(),
        schedule: steps.iter().map(schedule_step).collect(),
        steps,
    }
}
fn role(collection: &str, index: usize) -> Value {
    json!({"collection":collection,"selector":{"kind":"position","index":index},"cardinality":"exactly-one"})
}
fn int(n: &str, unit: &str) -> Value {
    json!({"op":"integer","value":n,"unit":{"kind":unit}})
}
fn binary(op: &str, a: Value, b: Value) -> Value {
    json!({"op":op,"left":a,"right":b})
}
fn reg(name: &str) -> Value {
    json!({"op":"register","name":name})
}
fn base(t: &SuppliedTrace) -> Value {
    let script = t.root[0].document()["ergoTree"].clone();
    json!({"schemaVersion":"author-property:v1","propertyId":"d02-independent-test","revision":"1","contracts":{"state":{"script":script,"compilerRevision":ergo_sandbox::evidence::validate::node_revision()}},"scope":{"kind":"transition"},"roles":{"before":role("inputs",0),"after":role("outputs",0)},"registers":{"before":{"role":"before","index":4,"valueType":"long","unit":{"kind":"nano-erg"}},"after":{"role":"after","index":4,"valueType":"long","unit":{"kind":"nano-erg"}}},"guard":{"op":"boolean","value":true},"assertion":{"op":"boolean","value":false},"authorizationPremises":[],"sources":[{"origin":"hypothetical","reference":"D02 supplied fixture","sha256":digest(&t.root)}]})
}
fn run(d: &Value, t: &SuppliedTrace) -> Value {
    evaluate(&Declaration::parse(&d.to_string()).unwrap(), t)
        .unwrap()
        .report()
        .clone()
}
fn response(t: &SuppliedTrace, k: u8) -> Value {
    let mut d = base(t);
    d["scope"] = json!({"kind":"bounded-response","horizon":k,"schedule":{"origin":"hypothetical","reference":"D02 explicit action/environment schedule","sha256":digest(&t.schedule)}});
    d["guard"] = binary("eq", reg("after"), int("1", "nano-erg"));
    d["assertion"] = binary("eq", reg("after"), int("0", "nano-erg"));
    d
}
fn reject(d: &Value, t: &SuppliedTrace) {
    assert!(evaluate(&Declaration::parse(&d.to_string()).unwrap(), t).is_err());
}
#[test]
fn four_property_families_match_independent_operands() {
    for family in ["reserve", "issuance", "continuation", "bounded_response"] {
        for disposition in ["violation", "control"] {
            let t = fixture(&format!("{family}_{disposition}_1"));
            let mut d = base(&t);
            match family {
                "reserve" => {
                    d["assertion"] = binary(
                        "ge",
                        binary(
                            "sub",
                            json!({"op":"sum-erg","role":"after"}),
                            json!({"op":"sum-erg","role":"before"}),
                        ),
                        binary("sub", reg("after"), reg("before")),
                    )
                }
                "issuance" => {
                    let id = t.root[0].document()["boxId"].as_str().unwrap();
                    d["assertion"] = binary(
                        "le",
                        json!({"op":"sum-token","role":"after","token":id}),
                        json!({"op":"integer","value":"10","unit":{"kind":"token","id":id}}),
                    );
                }
                "continuation" => {
                    d["roles"]["successors"] = json!({"collection":"outputs","selector":{"kind":"script","hex":t.root[0].document()["ergoTree"]},"cardinality":"all-matches"});
                    d["assertion"] = json!({"op":"and","args":[binary("eq",json!({"op":"count","role":"successors"}),int("1","count")),binary("eq",reg("before"),int("7","nano-erg"))]});
                }
                _ => d = response(&t, 2),
            }
            let r = run(&d, &t);
            assert_eq!(
                r["status"],
                if disposition == "violation" {
                    "violated"
                } else {
                    "holds-on-execution"
                },
                "{family}: {r}"
            );
            assert_eq!(r["contractDefect"], "not-adjudicated");
            assert_eq!(r["reachability"], "assumed-root");
            let observations = r["observations"].as_array().unwrap();
            assert_eq!(observations.len(), t.steps.len());
            // Independent arithmetic operands, not evaluator-produced expected answers.
            if family == "reserve" {
                let operands = observations[0]["operands"].as_array().unwrap();
                assert!(operands.iter().any(|o| o["value"]["integer"] == "10000000"));
                assert!(operands.iter().any(|o| o["value"]["integer"] == "2000000"));
            }
            if family == "issuance" || family == "continuation" {
                let expected = match (family, disposition) {
                    ("issuance", "violation") => "11",
                    ("issuance", _) => "10",
                    (_, "violation") => "0",
                    _ => "1",
                };
                let op = if family == "issuance" {
                    "sum-token"
                } else {
                    "count"
                };
                assert!(observations[0]["operands"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|o| o["expression"]["op"] == op && o["value"]["integer"] == expected));
            }
            if family == "bounded_response" {
                for row in &observations[1..] {
                    assert!(row["operands"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|o| o["expression"]["op"] == "register"
                            && o["value"]["integer"]
                                == if disposition == "violation" { "1" } else { "0" }));
                }
            }
            println!("{family} {disposition}: {}", r["status"]);
        }
    }
}
#[test]
fn guards_missing_fields_and_overflow_preserve_unknowns() {
    let t = fixture("reserve_violation_1");
    let d = base(&t);
    let mut x = d.clone();
    x["guard"] = json!({"op":"boolean","value":false});
    assert_eq!(run(&x, &t)["status"], "not-applicable");
    let mut x = d.clone();
    x["guard"] = binary("eq", reg("after"), int("0", "nano-erg"));
    x["registers"]["after"]["index"] = json!(9);
    assert_eq!(run(&x, &t)["status"], "unresolved");
    for kind in ["byte", "short", "int", "big-int"] {
        let mut x = d.clone();
        x["registers"]["after"]["valueType"] = json!(kind);
        x["assertion"] = binary("eq", reg("after"), int("0", "nano-erg"));
        assert_eq!(run(&x, &t)["status"], "unresolved");
    }
    for op in ["add", "sub"] {
        let mut x = d.clone();
        let (a, b) = if op == "add" {
            (i128::MAX.to_string(), "1")
        } else {
            (i128::MIN.to_string(), "1")
        };
        x["assertion"] = binary(
            "eq",
            binary(op, int(&a, "scalar"), int(b, "scalar")),
            int("0", "scalar"),
        );
        assert_eq!(run(&x, &t)["status"], "unresolved");
    }
    let mut x = d.clone();
    x["assertion"] = binary(
        "ge",
        json!({"op":"scale","arg":{"op":"sum-erg","role":"after"},"factor":i128::MAX.to_string()}),
        int("0", "nano-erg"),
    );
    assert_eq!(run(&x, &t)["status"], "unresolved");
    let mut x = d.clone();
    x["roles"]["empty"] = json!({"collection":"outputs","selector":{"kind":"script","hex":"00"},"cardinality":"all-matches"});
    x["assertion"] = binary(
        "eq",
        json!({"op":"sum-erg","role":"empty"}),
        int("0", "nano-erg"),
    );
    assert_eq!(run(&x, &t)["status"], "holds-on-execution");
    x["roles"]["empty"]["cardinality"] = json!("exactly-one");
    assert_eq!(run(&x, &t)["status"], "unresolved");
    let mut x = d;
    x["roles"]["after"]["selector"] = json!({"kind":"position","index":15});
    assert_eq!(run(&x, &t)["status"], "unresolved");
    println!("false guard, missing guard/register/role, type mismatches, empty sets and checked overflow preserve dispositions");
}
fn refresh(t: &mut SuppliedTrace) {
    t.schedule = t.steps.iter().map(schedule_step).collect();
}
#[test]
fn bounded_response_requires_a_complete_accepted_linked_trace() {
    let t = fixture("bounded_response_violation_1");
    assert_eq!(run(&response(&t, 2), &t)["status"], "violated");
    let mut short = t.clone();
    short.steps.pop();
    refresh(&mut short);
    assert_eq!(run(&response(&short, 2), &short)["status"], "unresolved");
    let mut bad = t.clone();
    bad.steps.swap(0, 1);
    refresh(&mut bad);
    reject(&response(&bad, 2), &bad);
    let mut bad = t.clone();
    bad.steps[1] = bad.steps[0].clone();
    refresh(&mut bad);
    reject(&response(&bad, 2), &bad);
    let mut bad = t.clone();
    bad.root.pop();
    reject(&response(&t, 2), &bad);
    let mut bad = t.clone();
    bad.schedule[0]["transactionBytes"] = json!("00");
    reject(&response(&bad, 2), &bad);
    let mut bad = t.clone();
    bad.steps = vec![t.steps[0].clone(); 9];
    refresh(&mut bad);
    reject(&response(&bad, 2), &bad);
    // Same-block replay uses actual accepted accumulated cost, never reset-to-zero.
    let mut same = t.clone();
    for i in 0..same.steps.len() {
        same.steps[i].block_context = t.steps.last().unwrap().block_context.clone();
        let mut premises = same.steps[i].case.premises().clone();
        premises.context = Premise::missing("typed block context supplied separately");
        same.steps[i].case = ergo_sandbox::evidence::EvidenceCase::new(premises).unwrap();
        if i > 0 {
            let cost = validate(&same.steps[i - 1]).unwrap().report()["totalBlockCost"]
                .as_u64()
                .unwrap();
            same.steps[i].prior_block_cost = Premise::supplied(cost);
        }
    }
    refresh(&mut same);
    assert_eq!(run(&response(&same, 2), &same)["status"], "violated");
    same.steps[1].prior_block_cost = Premise::supplied(0);
    refresh(&mut same);
    reject(&response(&same, 2), &same);
    // Exercise every allowed K with canonical test-only supplied successors.
    let mut extended = t.clone();
    while extended.steps.len() < 5 {
        let previous = extended.steps.last().unwrap();
        let accepted = validate(previous).unwrap();
        let wire = ergo_sandbox::evidence::wire::WireTransaction::from_bytes(
            &hex::decode(&previous.transaction_bytes).unwrap(),
            &hex::encode(accepted.checked().tx_id()),
        )
        .unwrap();
        let outputs = wire.output_boxes().unwrap();
        let mut node = wire.node().clone();
        node.inputs[0].box_id = outputs[0].node().box_id().unwrap();
        let wire = ergo_sandbox::evidence::wire::WireTransaction::from_node(node).unwrap();
        let mut next = previous.clone();
        next.transaction_bytes = hex::encode(wire.bytes());
        next.prior_block_cost =
            Premise::supplied(accepted.report()["totalBlockCost"].as_u64().unwrap());
        let mut p = next.case.premises().clone();
        p.boxes = Premise::supplied(outputs.iter().map(|b| b.record().clone()).collect());
        p.assumptions.clear();
        next.case = ergo_sandbox::evidence::EvidenceCase::new(p).unwrap();
        extended.steps.push(next);
    }
    for k in 1..=4 {
        let mut prefix = extended.clone();
        prefix.steps.truncate(k + 1);
        refresh(&mut prefix);
        let r = run(&response(&prefix, k as u8), &prefix);
        assert_eq!(r["status"], "violated");
        assert_eq!(r["firstFailingIndex"], k);
    }
    // A reused spend can pass single-request validation under a later context,
    // yet must fail the linked UTXO check independently of schedule/cost checks.
    let mut reused = t.clone();
    reused.steps[1] = t.steps[0].clone();
    reused.steps[1].block_context = t.steps[1].block_context.clone();
    let mut p = reused.steps[1].case.premises().clone();
    p.context = Premise::missing("typed schedule");
    reused.steps[1].case = ergo_sandbox::evidence::EvidenceCase::new(p).unwrap();
    validate(&reused.steps[1]).unwrap();
    refresh(&mut reused);
    let error = evaluate(
        &Declaration::parse(&response(&reused, 2).to_string()).unwrap(),
        &reused,
    )
    .unwrap_err();
    assert!(error.contains("unavailable"), "{error}");

    // Canonical data-input references must be both in the node snapshot and
    // available from the registered root. Build only a test request, no action constructor.
    let mut data = fixture("reserve_violation_1");
    let external = fixture("bounded_response_violation_1").root[0].clone();
    let accepted = validate(&data.steps[0]).unwrap();
    let wire = ergo_sandbox::evidence::wire::WireTransaction::from_bytes(
        &hex::decode(&data.steps[0].transaction_bytes).unwrap(),
        &hex::encode(accepted.checked().tx_id()),
    )
    .unwrap();
    let mut node = wire.node().clone();
    node.data_inputs.push(ergo_ser::input::DataInput {
        box_id: ergo_primitives::digest::Digest32::from_bytes(
            hex::decode(external.document()["boxId"].as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap(),
        ),
    });
    let wire = ergo_sandbox::evidence::wire::WireTransaction::from_node(node).unwrap();
    data.steps[0].transaction_bytes = hex::encode(wire.bytes());
    let mut p = data.steps[0].case.premises().clone();
    let mut boxes = p.boxes.value().unwrap().clone();
    boxes.push(external.clone());
    p.boxes = Premise::supplied(boxes);
    data.steps[0].case = ergo_sandbox::evidence::EvidenceCase::new(p).unwrap();
    data.root.push(external);
    refresh(&mut data);
    assert_eq!(run(&base(&data), &data)["status"], "violated");
    let mut missing = data.clone();
    missing.root.pop();
    reject(&base(&data), &missing);
    let mut missing = data.clone();
    let mut p = missing.steps[0].case.premises().clone();
    let mut boxes = p.boxes.value().unwrap().clone();
    boxes.pop();
    p.boxes = Premise::supplied(boxes);
    missing.steps[0].case = ergo_sandbox::evidence::EvidenceCase::new(p).unwrap();
    assert!(validate(&missing.steps[0]).is_err());
    reject(&base(&data), &missing);
    println!("complete linked response accepted; short horizon unresolved; reordered/reused spends, absent root, schedule tamper, trace cap and same-block cost reset rejected");
}
#[test]
fn property_evaluation_requires_fresh_node_acceptance() {
    let t = fixture("bounded_response_violation_1");
    for i in 0..t.steps.len() {
        let mut bad = t.clone();
        bad.steps[i].transaction_bytes = "00".into();
        refresh(&mut bad);
        reject(&response(&bad, 2), &bad);
    }
    let mut bad = t.clone();
    bad.steps[0].parameters = Premise::missing("no parameters");
    refresh(&mut bad);
    reject(&response(&bad, 2), &bad);
    let mut rejected = t.clone();
    let mut parameters = rejected.steps[0].parameters.value().unwrap().clone();
    parameters.min_value_per_byte = u64::MAX / 4096;
    rejected.steps[0].parameters = Premise::supplied(parameters);
    let failure = validate(&rejected.steps[0]).unwrap_err();
    assert_eq!(failure.status, "node-rejected");
    assert!(failure.pipeline_invoked);
    refresh(&mut rejected);
    reject(&response(&rejected, 2), &rejected);
    let r = run(&response(&t, 2), &t);
    let mut forged = r.clone();
    forged["status"] = json!("holds-on-execution");
    assert_ne!(forged, run(&response(&t, 2), &t));
    println!("trigger and every successor require fresh full node acceptance; exported JSON grants no authority");
}
