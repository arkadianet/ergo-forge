use ergo_sandbox::{
    compile::compile_with_params,
    evidence::wire::{CandidateSpec, CreationReference, WireBox},
    map::{self, fixture::Recorded, source::ChainBox, Fixture, MapOptions, Seed},
    Scenario,
};
use ergo_ser::address::NetworkPrefix;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mapping")
}
pub fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(name)).unwrap()).unwrap()
}
pub fn compile(source: &str) -> Vec<u8> {
    compile_with_params(source, &BTreeMap::new(), 3, NetworkPrefix::Mainnet)
        .unwrap_or_else(|e| panic!("{source}: {e}"))
        .tree_bytes
}

pub fn legacy_case(case: &Value) -> Value {
    let mut fx = Fixture::new("closed-synthetic", None, 100);
    let boxes: Vec<ChainBox> = case["universe"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            let spec = wire_spec(&b["spec"]);
            ChainBox {
                box_id: b["boxId"].as_str().unwrap().into(),
                ergo_tree: spec.ergo_tree,
                value: spec.value,
                tokens: spec
                    .tokens
                    .into_iter()
                    .map(|t| map::source::ChainToken {
                        id: t.id,
                        amount: t.amount,
                    })
                    .collect(),
                creation_height: 1,
                inclusion_height: 1,
                registers: b["scenarioBox"]["registers"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, tv)| {
                        let (t, v) = ergo_sandbox::scenario::parse_typed_value(
                            tv["type"].as_str().unwrap(),
                            &tv["value"],
                        )
                        .unwrap();
                        let mut w = ergo_primitives::writer::VlqWriter::new();
                        ergo_ser::sigma_value::write_constant(&mut w, &t, &v).unwrap();
                        (k.clone(), hex::encode(w.result()))
                    })
                    .collect(),
            }
        })
        .collect();
    let a = boxes
        .iter()
        .find(|b| b.box_id == case["subjectId"].as_str().unwrap())
        .unwrap()
        .clone();
    fx.boxes_by_address.insert(
        "registered-seed".into(),
        Recorded {
            items: vec![a],
            total: Some(1),
        },
    );
    for b in &boxes {
        fx.boxes.insert(b.box_id.clone(), b.clone());
        for t in &b.tokens {
            fx.tokens.insert(
                t.id.clone(),
                Some(map::source::TokenInfo {
                    id: t.id.clone(),
                    emission_amount: 4,
                }),
            );
            let r = fx.boxes_by_token.entry(t.id.clone()).or_insert(Recorded {
                items: vec![],
                total: Some(0),
            });
            r.items.push(b.clone());
            r.total = Some(r.items.len());
        }
    }
    // Complete negative token answers and exact script-hash index over this authored universe.
    fx.boxes_by_script_hash = Some(BTreeMap::new());
    for b in &boxes {
        let hash = hex::encode(
            ergo_primitives::digest::blake2b256(&hex::decode(&b.ergo_tree).unwrap()).as_bytes(),
        );
        let r = fx
            .boxes_by_script_hash
            .as_mut()
            .unwrap()
            .entry(hash)
            .or_default();
        r.items.push(b.clone());
        r.total = Some(r.items.len());
        let tree = ergo_sandbox::inspect::parse_tree(&hex::decode(&b.ergo_tree).unwrap()).unwrap();
        let lift = ergo_sandbox::lift_tree(&tree, true);
        for constant in map::refs::tree_refs(&lift.node).constants {
            fx.tokens.entry(constant).or_insert(None);
        }
    }
    for (id, info) in &mut fx.tokens {
        if let Some(info) = info {
            info.emission_amount = boxes
                .iter()
                .flat_map(|b| &b.tokens)
                .filter(|t| t.id == *id)
                .map(|t| t.amount)
                .sum();
        }
    }
    let options = if case["id"] == "snapshot_collision-control" {
        MapOptions {
            max_nodes: 1,
            ..MapOptions::default()
        }
    } else {
        MapOptions::default()
    };
    let m = map::map(&fx, &Seed::Address("registered-seed".into()), &options).unwrap();
    map::json::canonical(&m)
}

pub fn canonical_box(spec: &CandidateSpec, index: u16) -> WireBox {
    WireBox::hypothetical(
        spec,
        &CreationReference {
            transaction_id: "00".repeat(32),
            index,
        },
    )
    .unwrap()
}
pub fn scenario(case: &Value, vector: &Value) -> Scenario {
    let get = |name: &Value| {
        let b = case["universe"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == *name)
            .unwrap();
        let mut v = b["scenarioBox"].clone();
        v["ergoTree"] = v.as_object_mut().unwrap().remove("propositionHex").unwrap();
        // Extensions are transaction material, not a field of the canonical box.
        v["extension"] = json!({"1":{"type":"Coll[Byte]","value":"7f"}});
        v
    };
    let mut value = json!({"tree":case["treeHex"],"height":vector["height"],"selfIndex":0,"contextVars":vector["contextVars"],"activatedScriptVersion":3,"costLimit":1000000});
    for field in ["inputs", "dataInputs", "outputs"] {
        value[field] = Value::Array(vector[field].as_array().unwrap().iter().map(get).collect());
    }
    if let Some(lookup) = vector.get("avlLookup") {
        value["avl"] = json!({"t":{"keyLength":32,"entries":[["22".repeat(32),"aa"]],"operations":[{"lookup":{"key":lookup}}]}});
    }
    serde_json::from_value(value).unwrap()
}

pub fn measure(manifest: &Value, expected: &Value) -> Value {
    let mut rows = vec![];
    let mut recovered = vec![];
    let mut reachable = vec![];
    let mut dynamic_expected = vec![];
    let mut dynamic_recovered = vec![];
    let mut false_edges = vec![];
    let mut adjudicated = vec![];
    let mut unadjudicated = vec![];
    let mut sites = vec![];
    let mut legacy_sites = vec![];
    let mut supported_expected = vec![];
    let mut supported_recovered = vec![];
    for (entry, answer) in manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["cases"].as_array().unwrap())
    {
        let case = read(entry["path"].as_str().unwrap());
        assert_eq!(case["id"], answer["id"]);
        let id = case["id"].as_str().unwrap();
        let starts = (
            recovered.len(),
            reachable.len(),
            dynamic_recovered.len(),
            dynamic_expected.len(),
        );
        let map = legacy_case(&case);
        let named_id = |name: &str| {
            case["universe"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["name"] == name)
                .unwrap()["boxId"]
                .as_str()
                .unwrap()
        };
        for member in answer["discovery"]["members"].as_array().unwrap() {
            let member_id = format!("{id}/{}", member.as_str().unwrap());
            reachable.push(member_id.clone());
            let found = map["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["boxId"] == named_id(member.as_str().unwrap()));
            if found {
                recovered.push(member_id.clone());
            }
            if answer["discovery"]["supportedByM02"] == true {
                supported_expected.push(member_id.clone());
                if found {
                    supported_recovered.push(member_id);
                }
            }
        }
        let mut found_tuples = vec![];
        for (i, edge) in map["edges"].as_array().unwrap().iter().enumerate() {
            let edge_id = format!("{id}/edge/{i}");
            let from = case["universe"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["boxId"] == edge["from"]["boxId"]);
            let to = case["universe"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["boxId"] == edge["to"]["boxId"]);
            if let (Some(from), Some(to)) = (from, to) {
                let tuple = json!([from["name"], to["name"]]);
                found_tuples.push(tuple.clone());
                if answer["discovery"]["trueTuples"]
                    .as_array()
                    .unwrap()
                    .contains(&tuple)
                {
                    adjudicated.push(edge_id);
                } else if answer["discovery"]["falseTuples"]
                    .as_array()
                    .unwrap()
                    .contains(&tuple)
                {
                    adjudicated.push(edge_id.clone());
                    false_edges.push(edge_id);
                } else {
                    unadjudicated.push(edge_id);
                }
            } else {
                unadjudicated.push(edge_id);
            }
        }
        if answer["dynamic"] == true {
            for tuple in answer["discovery"]["trueTuples"].as_array().unwrap() {
                let tuple_id = format!("{id}/{}", serde_json::to_string(tuple).unwrap());
                dynamic_expected.push(tuple_id.clone());
                if found_tuples.contains(tuple) {
                    dynamic_recovered.push(tuple_id);
                }
            }
        }
        let tree = ergo_sandbox::inspect::parse_tree(
            &hex::decode(case["treeHex"].as_str().unwrap()).unwrap(),
        )
        .unwrap();
        let refs = map::refs::tree_refs(&ergo_sandbox::lift_tree(&tree, true).node);
        let dispositions:Vec<Value>=case["sites"].as_array().unwrap().iter().map(|s| {
            sites.push(s["id"].clone());
            let found=refs.slots.contains_key(s["text"].as_str().unwrap());
            if found { legacy_sites.push(s["id"].clone()); }
            json!({"id":s["id"],"legacySlotObserved":found,"disposition":if found {"observed-slot-only"} else {"unresolved"},"reason":if found {"legacy slot observation has no proof authority"} else {"no corresponding legacy slot; inventory preserved"}})
        }).collect();
        rows.push(json!({"id":id,"family":case["family"],"m02Supported":answer["discovery"]["supportedByM02"],"reachability":{"numeratorIds":&recovered[starts.0..],"denominatorIds":&reachable[starts.1..]},"dynamicRecovery":{"numeratorIds":&dynamic_recovered[starts.2..],"denominatorIds":&dynamic_expected[starts.3..]},"inputSha256":entry["sha256"],"legacyMapJson":serde_json::to_string(&map).unwrap(),"siteDispositions":dispositions,"proofStatus":"unresolved","proofReason":"legacy API emits no required-relations:v1 claims"}));
    }
    let ratio = |num: usize, den: usize| {
        if den == 0 {
            Value::Null
        } else {
            json!(num as f64 / den as f64)
        }
    };
    let count = |n: Vec<String>, d: Vec<String>| json!({"numerator":n.len(),"denominator":d.len(),"rate":ratio(n.len(),d.len()),"numeratorIds":n,"denominatorIds":d});
    let emitted = adjudicated.len() + unadjudicated.len();
    let public:Vec<Value>=[("use-lp.json","4ecaa1aac9846b1454563ae51746db95a3a40ee9f8c5f5301afbe348ae803d41"),("dexy-gold.json","905ecdef97381b92c2f0ea9b516f312bfb18082c61b24b40affa6a55555c77c7")].into_iter().map(|(file,token)| {
        let path=root().join("../map").join(file); let bytes=std::fs::read(path).unwrap();
        let fx=Fixture::from_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
        let m=map::map(&fx,&Seed::TokenId(token.into()),&MapOptions {max_depth:6,max_nodes:96,..MapOptions::default()}).unwrap();
        json!({"path":format!("../map/{file}"),"inputSha256":sha(&bytes),"publication":"existing public explorer observation; regression only","knownCompleteProtocol":false,"reachabilityDenominator":null,"legacyMapJson":serde_json::to_string(&map::json::canonical(&m)).unwrap()})
    }).collect();
    json!({"version":"mapping-legacy-results:v1","producer":"M00 real legacy map/tree_refs APIs; no new extractor","producerRevision":"b6ea1c9eb18f2b5b18f502ddda60ba30e0e70d81","nodeRevision":manifest["nodeRevision"],"manifestSha256":sha(&std::fs::read(root().join("manifest.json")).unwrap()),"expectedSha256":manifest["expected"]["sha256"],"metrics":{
        "caseAccounting":{"numerator":rows.len(),"denominator":32,"meaning":"fixture harness accounting, not mapping completeness"},
        "legacySiteObservation":{"numerator":legacy_sites.len(),"denominator":sites.len(),"rate":ratio(legacy_sites.len(),sites.len()),"numeratorIds":legacy_sites,"denominatorIds":sites},
        "closedFixtureReachability":count(recovered,reachable),"m02SupportedReachability":count(supported_recovered,supported_expected),"dynamicTupleRecovery":count(dynamic_recovered,dynamic_expected),
        "falseProposal":{"numerator":false_edges.len(),"denominator":emitted,"rate":if unadjudicated.is_empty(){ratio(false_edges.len(),emitted)}else{Value::Null},"falseIds":false_edges,"adjudicatedIds":adjudicated,"unadjudicatedIds":unadjudicated,"meaning":"unresolved targets remain unadjudicated; never call this zero false proposals"},
        "promotedFalseRelation":{"numerator":0,"denominator":0,"rate":null,"reason":"no relation API exists"},
        "necessityRecall":null,"actionSetAccuracy":null,"omissionRefutation":null,"wholeDeployedProtocolCompleteness":null,
        "unavailableReason":"legacy API produces neither checked necessities, action checks nor node-accepted omission witnesses; whole-chain denominator unknown"
    },"cases":rows,"publicRegression":public})
}

// Mapping fixture field names keep its inventory separate from the frozen
// decompiler JSON corpus. Only this test adapter constructs legacy API DTOs.
pub fn wire_spec(value: &Value) -> CandidateSpec {
    let mut value = value.clone();
    value["ergoTree"] = value
        .as_object_mut()
        .unwrap()
        .remove("propositionHex")
        .unwrap();
    serde_json::from_value(value).unwrap()
}
