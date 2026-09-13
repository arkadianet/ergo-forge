//! Mechanical adaptation of frozen draft syntax; never reads the answer key.
use ergo_sandbox::{
    evidence::validate::{node_revision, ValidationRequest},
    properties::{
        replay::ReplayEnvelope,
        trace::{digest, schedule_step},
    },
};
use serde_json::{json, Value};
pub fn envelope(c: &Value) -> ReplayEnvelope {
    let p = &c["authoring"]["property"];
    let steps: Vec<ValidationRequest> = c["references"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| serde_json::from_value(r["request"].clone()).unwrap())
        .collect();
    let root = steps[0].case.premises().boxes.value().unwrap().clone();
    let schedule: Vec<Value> = steps.iter().map(schedule_step).collect();
    let token = root[0].document()["boxId"].as_str().unwrap();
    let script = root[0].document()["ergoTree"].clone();
    let source = json!({"origin":"hypothetical","reference":format!("D00 frozen draft {}",c["id"]),"sha256":digest(p)});
    let mut registers = json!({});
    let roles: serde_json::Map<String, Value> = p["roles"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, r)| {
            let mut r = r.clone();
            let selector = &r["selector"];
            r["selector"] = if let Some(i) = selector.get("position") {
                json!({"kind":"position","index":i})
            } else {
                json!({"kind":"script","hex":script})
            };
            (name.clone(), r)
        })
        .collect();
    fn unit(u: &str, token: &str) -> Value {
        match u {
            "nanoERG" => json!({"kind":"nano-erg"}),
            "token-units" => json!({"kind":"token","id":token}),
            "count" | "scalar" => json!({"kind":u}),
            _ => json!({"kind":"named-amount","name":u}),
        }
    }
    fn expr(e: &Value, token: &str, registers: &mut Value) -> Value {
        let op = e["op"].as_str().unwrap();
        match op {
            "read" => {
                let role = e["role"].as_str().unwrap();
                match e["field"].as_str().unwrap() {
                    "value" => json!({"op":"sum-erg","role":role}),
                    "token:first-input-id" => json!({"op":"sum-token","role":role,"token":token}),
                    "R4:Long" => {
                        let name = format!("{role}_r4");
                        registers[&name] = json!({"role":role,"index":4,"valueType":"long","unit":unit(e["unit"].as_str().unwrap(),token)});
                        json!({"op":"register","name":name})
                    }
                    _ => e.clone(),
                }
            }
            "literal" => {
                json!({"op":"integer","value":e["value"],"unit":unit(e["unit"].as_str().unwrap(),token)})
            }
            "count" => json!({"op":"count","role":e["role"]}),
            "mul-literal" => {
                json!({"op":"scale","arg":expr(&e["args"][0],token,registers),"factor":e["args"][1]["value"]})
            }
            "and" | "or" => {
                json!({"op":op,"args":e["args"].as_array().unwrap().iter().map(|e|expr(e,token,registers)).collect::<Vec<_>>()})
            }
            "eq" | "le" | "ge" | "sub" | "add" => {
                json!({"op":op,"left":expr(&e["args"][0],token,registers),"right":expr(&e["args"][1],token,registers)})
            }
            _ => e.clone(),
        }
    }
    let guard = expr(
        p.get("trigger").unwrap_or(&p["guard"]),
        token,
        &mut registers,
    );
    let assertion = expr(&p["assertion"], token, &mut registers);
    let scope = if p["scope"] == "bounded-response" {
        json!({"kind":"bounded-response","horizon":p["horizon"],"schedule":{"origin":"hypothetical","reference":p["schedule"].to_string(),"sha256":digest(&schedule)}})
    } else {
        json!({"kind":p["scope"]})
    };
    let declaration = json!({"schemaVersion":"author-property:v1","propertyId":p["id"],"revision":p["revision"].to_string(),"contracts":{"state":{"script":script,"compilerRevision":node_revision()}},"scope":scope,"roles":roles,"registers":registers,"guard":guard,"assertion":assertion,"authorizationPremises":p["authorizationPremises"].as_array().unwrap().iter().enumerate().map(|(i,s)|json!({"id":format!("premise-{i}"),"statement":s,"source":source})).collect::<Vec<_>>(),"sources":[source]});
    ReplayEnvelope {
        schema_version: "property-replay:v1".into(),
        declaration,
        root,
        steps,
        schedule,
        node_revision: node_revision().into(),
        evaluator_revision: "author-property-evaluator:v1".into(),
    }
}
