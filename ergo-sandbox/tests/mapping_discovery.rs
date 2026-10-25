//! M02 discovery gates consume the unchanged M00 independent inventory.
#[allow(dead_code)]
mod mapping_support;
use ergo_sandbox::{
    evidence::case::Premise,
    map::{
        discover_refs::{discover, Caps, DiscoveryRun, Site, Supply},
        discovery::{DiscoveryMap, MatchStatus},
        relations::RequiredRelations,
    },
};
use mapping_support::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn result_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("docs/mapping/m02-mapping-results.json")
}
fn inventory() -> (Value, Value) {
    assert_eq!(
        sha(&std::fs::read(root().join("manifest.json")).unwrap()),
        "7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f"
    );
    assert_eq!(
        sha(&std::fs::read(root().join("expected.json")).unwrap()),
        "291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e"
    );
    let m = read("manifest.json");
    let e = read("expected.json");
    let ids: BTreeSet<_> = m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].clone().to_string())
        .collect();
    assert_eq!(ids.len(), 32);
    assert_eq!(m["cases"].as_array().unwrap().len(), 32);
    assert_eq!(
        ids,
        e["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].clone().to_string())
            .collect()
    );
    for row in m["cases"].as_array().unwrap() {
        assert_eq!(
            sha(&std::fs::read(root().join(row["path"].as_str().unwrap())).unwrap()),
            row["sha256"]
        );
    }
    (m, e)
}
fn supply(c: &Value) -> Supply {
    let records = c["universe"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            canonical_box(
                &wire_spec(&b["spec"]),
                b["reference"]["index"].as_u64().unwrap() as u16,
            )
            .record()
            .clone()
        })
        .collect::<Vec<_>>();
    Supply {
        expected_records: Some(records.len()),
        records,
        queries: vec![json!({"kind":"supplied-finite-universe","snapshot":c["snapshot"]})],
        snapshot: Premise::supplied(c["snapshot"].clone()),
        missing_pages: vec![],
        unsupported_queries: vec![],
        inconsistencies: vec![],
    }
}
fn run(c: &Value, s: &Supply, caps: &Caps) -> DiscoveryRun {
    discover(
        &hex::decode(c["treeHex"].as_str().unwrap()).unwrap(),
        c["subjectId"].as_str().unwrap(),
        &c["sites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| Site {
                id: s["id"].as_str().unwrap().into(),
                text: s["text"].as_str().unwrap().into(),
            })
            .collect::<Vec<_>>(),
        s,
        caps,
    )
}
fn count(n: Vec<String>, d: Vec<String>) -> Value {
    json!({"numerator":n.len(),"denominator":d.len(),"rate":if d.is_empty(){Value::Null}else{json!(n.len() as f64/d.len() as f64)},"numeratorIds":n,"denominatorIds":d})
}
fn measured() -> Value {
    let (m, e) = inventory();
    let mut rows = vec![];
    let mut all = vec![];
    let mut recovered = vec![];
    let mut supported = vec![];
    let mut supported_recovered = vec![];
    let mut site_ids = vec![];
    let mut accounted = vec![];
    let mut false_ids = vec![];
    let mut adjudicated = vec![];
    let mut unadjudicated = vec![];
    let mut dynamic_den = vec![];
    let mut dynamic_num = vec![];
    let mut splits = vec![];
    for entry in m["cases"].as_array().unwrap() {
        let c = read(entry["path"].as_str().unwrap());
        let a = e["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == c["id"])
            .unwrap();
        let id = c["id"].as_str().unwrap();
        let result = run(&c, &supply(&c), &Caps::default());
        let names = |target: &str| {
            c["universe"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["boxId"] == target)
                .unwrap()["name"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let mut found = BTreeSet::new();
        let mut tuples = BTreeSet::new();
        for o in &result.map.observations {
            for target in &o.proposed_targets {
                let name = names(target);
                let tuple = json!(["A", name]);
                let edge = format!("{id}/{}", o.id);
                if a["discovery"]["trueTuples"]
                    .as_array()
                    .unwrap()
                    .contains(&tuple)
                {
                    adjudicated.push(edge.clone());
                } else if a["discovery"]["falseTuples"]
                    .as_array()
                    .unwrap()
                    .contains(&tuple)
                {
                    adjudicated.push(edge.clone());
                    false_ids.push(edge.clone());
                } else {
                    unadjudicated.push(edge);
                }
                // Hints and unresolved candidates never count as recovery.
                if o.status == MatchStatus::ObservedMatch {
                    found.insert(name);
                    tuples.insert(tuple.to_string());
                }
            }
        }
        let mut dn = vec![];
        let mut dd = vec![];
        for member in a["discovery"]["members"].as_array().unwrap() {
            let name = member.as_str().unwrap();
            let mid = format!("{id}/{name}");
            all.push(mid.clone());
            if found.contains(name) {
                recovered.push(mid.clone());
            }
            if a["discovery"]["supportedByM02"] == true {
                supported.push(mid.clone());
                if found.contains(name) {
                    supported_recovered.push(mid);
                }
            }
        }
        if a["dynamic"] == true {
            for tuple in a["discovery"]["trueTuples"].as_array().unwrap() {
                let tid = format!("{id}/{tuple}");
                dd.push(tid.clone());
                dynamic_den.push(tid.clone());
                if tuples.contains(&tuple.to_string()) {
                    dn.push(tid.clone());
                    dynamic_num.push(tid);
                }
            }
            splits.push(json!({"id":id,"family":c["family"],"supported":a["discovery"]["supportedByM02"],"recovery":count(dn,dd)}));
        }
        for s in c["sites"].as_array().unwrap() {
            site_ids.push(s["id"].as_str().unwrap().to_owned());
        }
        for s in &result.sites {
            assert!(!s.reason.is_empty());
            accounted.push(s.id.clone());
        }
        rows.push(json!({"id":id,"inputSha256":entry["sha256"],"m02Supported":a["discovery"]["supportedByM02"],"recoveredMembers":found,"discovery":result}));
    }
    assert_eq!(accounted, site_ids);
    assert_eq!(
        supported_recovered, supported,
        "fixed supported denominator must be recovered exactly"
    );
    let emitted = adjudicated.len() + unadjudicated.len();
    json!({"version":"mapping-results:m02:v1","producer":"map::discover_refs::discover","baseRevision":"ca38fd4508a3144345e38c5301fcb5bf0dcc4443","nodeRevision":m["nodeRevision"],"compilerRevision":m["compilerRevision"],"manifestSha256":sha(&std::fs::read(root().join("manifest.json")).unwrap()),"expectedSha256":m["expected"]["sha256"],"metrics":{
        "caseAccounting":{"numerator":rows.len(),"denominator":32},"siteAccounting":count(accounted,site_ids),"supportedRecall":count(supported_recovered,supported),"allCaseRecall":count(recovered,all),"dynamicRecovery":count(dynamic_num,dynamic_den),"dynamicByFamilyAndSupport":splits,
        "falseProposal":{"numerator":false_ids.len(),"denominator":emitted,"rate":if emitted==0 || !unadjudicated.is_empty(){Value::Null}else{json!(false_ids.len() as f64/emitted as f64)},"falseIds":false_ids,"adjudicatedIds":adjudicated,"unadjudicatedIds":unadjudicated},
        "promotedFalseRelation":{"numerator":0,"denominator":0,"rate":null,"reason":"discovery has no establishment API; M03 not implemented"},"necessityRecall":null,"actionSetAccuracy":null,"omissionRefutation":null,"wholeDeployedProtocolCompleteness":null,
        "meaning":"finite supplied hypothetical material only; lexical site accounting is not exact-tree correspondence; neither observation nor hint confers necessity or simultaneous availability"
    },"cases":rows})
}
#[test]
fn supported_reference_recall_and_all_site_accounting() {
    // Additional non-corpus controls exercise shapes rather than case IDs.
    let b = read("literal_spend-positive.json")["universe"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "B")
        .unwrap()
        .clone();
    let hash = hex::encode(
        ergo_primitives::digest::blake2b256(
            &hex::decode(b["spec"]["propositionHex"].as_str().unwrap()).unwrap(),
        )
        .as_bytes(),
    );
    for source in [
        format!("sigmaProp(blake2b256(INPUTS(1).propositionBytes) == fromBase16(\"{hash}\"))"),
        format!(
            "{{ val b = INPUTS(1); sigmaProp(b.id == fromBase16(\"{}\")) }}",
            b["boxId"].as_str().unwrap()
        ),
    ] {
        let c = derived(&source);
        let r = run(&c, &supply(&c), &Caps::default());
        assert_eq!(r.map.observations.len(), 1);
        assert_eq!(
            r.map.observations[0].proposed_targets,
            vec![b["boxId"].as_str().unwrap()]
        );
    }
    let result = measured();
    assert_eq!(
        result,
        serde_json::from_slice::<Value>(&std::fs::read(result_path()).unwrap()).unwrap()
    );
    println!("{}", serde_json::to_string(&result["metrics"]).unwrap());
    println!(
        "{} sha256={}",
        result_path().display(),
        sha(&std::fs::read(result_path()).unwrap())
    );
}
#[test]
fn false_hints_never_become_required_relations() {
    let results = measured();
    assert!(
        results["metrics"]["falseProposal"]["numerator"]
            .as_u64()
            .unwrap()
            > 0
    );
    for row in results["cases"].as_array().unwrap() {
        let d = row["discovery"]["map"].clone();
        let map: DiscoveryMap = serde_json::from_value(d.clone()).unwrap();
        assert!(serde_json::from_value::<RequiredRelations>(d.clone()).is_err());
        let mut renamed = d;
        renamed["version"] = json!("required-relations:v1");
        assert!(serde_json::from_value::<RequiredRelations>(renamed).is_err());
        for o in &map.observations {
            assert_eq!(map.nominate(&o.id).unwrap().discovery_id(), o.id);
        }
    }
    for name in [
        "literal_spend-control",
        "dead_check-control",
        "data_only-control",
        "output_only-control",
        "exists_literal-control",
    ] {
        let c = read(&format!("{name}.json"));
        let r = run(&c, &supply(&c), &Caps::default());
        assert!(r
            .map
            .observations
            .iter()
            .any(|o| o.status == MatchStatus::ObservedMatch));
    }
    let c = read("snapshot_collision-control.json");
    let r = run(&c, &supply(&c), &Caps::default());
    assert!(!r.map.observations.is_empty());
    assert!(r
        .map
        .observations
        .iter()
        .all(|o| o.status == MatchStatus::Hint));
    println!("false proposals retained: {}; discovery imports, renamed envelopes, optional and non-spending observations confer no relation authority",results["metrics"]["falseProposal"]);
}
// Derived in memory only; registered source/tree/answers are never rewritten.
fn derived(source: &str) -> Value {
    let mut c = read("literal_spend-positive.json");
    let tree = hex::encode(compile(source));
    c["treeHex"] = json!(tree);
    let a = c["universe"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|b| b["name"] == "A")
        .unwrap();
    a["spec"]["propositionHex"] = json!(tree);
    let wire = canonical_box(
        &wire_spec(&a["spec"]),
        a["reference"]["index"].as_u64().unwrap() as u16,
    );
    a["boxId"] = json!(wire.id().unwrap());
    c["subjectId"] = json!(wire.id().unwrap());
    c
}
fn unresolved(r: &DiscoveryRun, reason: &str) {
    assert!(
        r.map.unresolved_reasons.iter().any(|s| s.contains(reason)),
        "{reason}: {:?}",
        r.map.unresolved_reasons
    );
    assert!(r
        .map
        .observations
        .iter()
        .all(|o| o.status != MatchStatus::ObservedMatch));
}
#[test]
fn caps_missing_pages_and_computed_identities_stay_unresolved() {
    inventory();
    let c = read("exists_literal-positive.json");
    let s = supply(&c);
    for caps in [
        Caps {
            nodes: 0,
            ..Caps::default()
        },
        Caps {
            tree_bytes: 0,
            ..Caps::default()
        },
        Caps {
            boxes: 1,
            ..Caps::default()
        },
        Caps {
            comparisons: 0,
            ..Caps::default()
        },
    ] {
        unresolved(&run(&c, &s, &caps), "cap");
    }
    let mut bad = s.clone();
    bad.records.push(bad.records[0].clone());
    bad.expected_records = Some(bad.records.len());
    unresolved(&run(&c, &bad, &Caps::default()), "source-inconsistency");
    let mut bad = s.clone();
    let mut doc = bad.records[0].document().clone();
    doc["boxId"] = json!("00".repeat(32));
    bad.records[0] = ergo_sandbox::evidence::case::RecordedBox::new(
        doc,
        ergo_sandbox::evidence::case::Origin::Hypothetical,
        None,
    )
    .unwrap();
    unresolved(&run(&c, &bad, &Caps::default()), "source-inconsistency");
    let mut bad = s.clone();
    bad.inconsistencies
        .push("responses disagree at reported height".into());
    unresolved(&run(&c, &bad, &Caps::default()), "source-inconsistency");
    for name in [
        "computed_index-positive",
        "computed_index-control",
        "avl_identity-positive",
        "avl_identity-control",
    ] {
        let c = read(&format!("{name}.json"));
        let r = run(&c, &supply(&c), &Caps::default());
        unresolved(
            &r,
            if name.starts_with("computed") {
                "computed-index"
            } else {
                "avl-identity"
            },
        );
        assert!(r.map.observations.is_empty());
        assert!(r.sites.iter().all(|s| s.status == MatchStatus::Unresolved));
    }
    let exact = Caps {
        comparisons: s.records.len(),
        ..Caps::default()
    };
    assert!(run(&c, &s, &exact)
        .map
        .unresolved_reasons
        .iter()
        .all(|s| !s.contains("cap")));
    let partial = Caps {
        comparisons: s.records.len() - 1,
        ..Caps::default()
    };
    unresolved(&run(&c, &s, &partial), "comparison-cap");
    let mut missing = s.clone();
    missing.missing_pages.push("page/2".into());
    missing.expected_records = Some(10);
    missing
        .unsupported_queries
        .push("historical-unspentness".into());
    missing.snapshot = Premise::missing("no coherent snapshot");
    let r = run(&c, &missing, &Caps::default());
    for reason in [
        "missing-page",
        "unknown-or-incomplete-inventory",
        "unsupported-query",
        "unknown-snapshot",
    ] {
        assert!(r.map.unresolved_reasons.iter().any(|s| s.contains(reason)));
    }
    let mut malformed = c.clone();
    malformed["treeHex"] = json!("ff");
    unresolved(&run(&malformed, &s, &Caps::default()), "parse-failure");
    println!("tree/node/box/comparison caps, missing pages, unknown inventory/snapshot, unsupported queries, duplicate/inconsistent source, parse failure and all four computed/AVL members exercised");
}
