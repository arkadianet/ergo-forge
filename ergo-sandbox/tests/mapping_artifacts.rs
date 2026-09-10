//! M01 uses the pinned M00 inventory; schema examples are separate from its membership.
#[allow(dead_code)]
mod mapping_support;
use ergo_sandbox::map::{discovery::DiscoveryMap, relations::RequiredRelations};
use mapping_support::*;
use serde_json::{json, Value};

fn example(name: &str) -> Value {
    read(name)
}
fn rejected(value: Value) {
    assert!(serde_json::from_value::<RequiredRelations>(value).is_err());
}

#[test]
fn discovery_cannot_import_as_required_execution() {
    let d = example("m01-discovery.json");
    let map: DiscoveryMap = serde_json::from_value(d.clone()).unwrap();
    assert_eq!(
        map.nominate("candidate/0").unwrap().discovery_id(),
        "candidate/0"
    );
    assert!(map.nominate("absent").is_err());
    rejected(d.clone());
    let mut renamed = d.clone();
    renamed["version"] = json!("required-relations:v1");
    rejected(renamed);
    for status in [
        "established-under-premises",
        "refuted",
        "node-accepted",
        "satisfies-declared-requirements",
    ] {
        let mut forged = example("m01-relations.json");
        forged["proposals"][0]["proposal"]["status"] = json!(status);
        rejected(forged);
        let mut forged = d.clone();
        forged["observations"][0]["status"] = json!(status);
        assert!(serde_json::from_value::<DiscoveryMap>(forged).is_err());
    }
    for version in ["discovery-map:v1", "discovery-map:v3", "action-check:v1"] {
        let mut unknown = d.clone();
        unknown["version"] = json!(version);
        assert!(serde_json::from_value::<DiscoveryMap>(unknown).is_err());
    }
    let mut forged = d;
    forged["completeActionSet"] = json!(true);
    assert!(serde_json::from_value::<DiscoveryMap>(forged).is_err());
    let p = example("m01-relations.json");
    let imported: RequiredRelations = serde_json::from_value(p.clone()).unwrap();
    assert_eq!(serde_json::to_value(imported).unwrap(), p);
    let mut unknown = p;
    unknown["version"] = json!("required-relations:v2");
    rejected(unknown);
}

#[test]
fn premise_changes_invalidate_imported_proofs() {
    let original = example("m01-relations.json");
    let parsed: RequiredRelations = serde_json::from_value(original.clone()).unwrap();
    for field in original["proposals"][0]["proposal"]["premises"]
        .as_object()
        .unwrap()
        .keys()
    {
        let mut changed = original.clone();
        let slot = &mut changed["proposals"][0]["proposal"]["premises"][field];
        match field.as_str() {
            "guard" => *slot = json!({"kind":"not","guard":{"kind":"true"}}),
            "analysisCaps" => slot["nodes"] = json!(7),
            "authenticationRoots" => slot
                .as_array_mut()
                .unwrap()
                .push(example("m01-discovery.json")["recordedBoxes"][0].clone()),
            _ => *slot = json!({"status":"missing", "reason":format!("changed {field}")}),
        }
        rejected(changed.clone());
        // Even recomputing a public digest can create only a new unresolved proposal.
        let proposal = serde_json::from_value(changed["proposals"][0]["proposal"].clone()).unwrap();
        let fresh = RequiredRelations::new(vec![proposal]).unwrap();
        assert_ne!(
            fresh.proposals()[0].claim_digest(),
            parsed.proposals()[0].claim_digest()
        );
    }
    for field in ["subject", "target"] {
        let mut changed = original.clone();
        if field == "subject" {
            changed["proposals"][0]["proposal"][field]["hex"] = json!("changed");
        } else {
            changed["proposals"][0]["proposal"][field]["selector"]["hex"] = json!("changed");
        }
        rejected(changed);
    }
    let mut duplicate_guard = original;
    duplicate_guard["proposals"][0]["proposal"]["guard"] = json!({"kind":"true"});
    rejected(duplicate_guard);
}

#[test]
fn raw_provenance_and_legacy_map_bytes_survive() {
    let example = example("m01-discovery.json");
    let imported: DiscoveryMap = serde_json::from_value(example.clone()).unwrap();
    assert_eq!(serde_json::to_value(&imported).unwrap(), example);
    assert!(imported.recorded_boxes[0].registers().value().is_none());
    assert_eq!(
        imported.recorded_boxes[1].registers().value(),
        Some(&json!({}))
    );
    assert_eq!(
        imported.recorded_boxes[2].document()["additionalRegisters"]["R4"]["renderedValue"],
        "retained"
    );
    let manifest = read("manifest.json");
    assert_eq!(
        sha(&std::fs::read(root().join("manifest.json")).unwrap()),
        "7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f"
    );
    assert_eq!(
        sha(&std::fs::read(root().join("expected.json")).unwrap()),
        "291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e"
    );
    for entry in manifest["cases"].as_array().unwrap() {
        assert_eq!(
            sha(&std::fs::read(root().join(entry["path"].as_str().unwrap())).unwrap()),
            entry["sha256"]
        );
    }
    let measured = measure(&manifest, &read("expected.json"));
    assert_eq!(measured, read("legacy-results.json"));
    for row in measured["cases"].as_array().unwrap() {
        // Compare serialized bytes against the pre-M01 snapshots, not two fresh runs.
        assert_eq!(
            serde_json::to_string(&legacy_case(&read(&format!(
                "{}.json",
                row["id"].as_str().unwrap()
            ))))
            .unwrap(),
            row["legacyMapJson"]
        );
    }
    let result = json!({"version":"mapping-artifacts-results:v1", "nodeRevision":manifest["nodeRevision"], "compilerRevision":manifest["compilerRevision"], "inventoryRegistrationRevision":manifest["registrationRevision"], "inputManifestSha256":sha(&std::fs::read(root().join("manifest.json")).unwrap()), "legacyResultsSha256":sha(&std::fs::read(root().join("legacy-results.json")).unwrap()), "byteStableCaseIds":measured["cases"].as_array().unwrap().iter().map(|r|r["id"].clone()).collect::<Vec<_>>(), "proofAuthority":"unavailable: exact-tree checker not implemented in M01", "actionAuthority":"unavailable: action checker not implemented in M01"});
    assert_eq!(result, read("m01-mapping-results.json"));
    for path in [
        "m01-discovery.json",
        "m01-relations.json",
        "m01-mapping-results.json",
    ] {
        println!(
            "{} sha256={}",
            root().join(path).display(),
            sha(&std::fs::read(root().join(path)).unwrap())
        );
    }
}
