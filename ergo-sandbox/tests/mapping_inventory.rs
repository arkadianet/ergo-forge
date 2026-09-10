//! M00 locks an independently authored denominator before any extractor exists.
//! These are fixture gates, not a required-relations proof API.
mod mapping_support;
use mapping_support::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const MANIFEST_SHA256: &str = "7f956f559b3912348f6759ab0e1855cd90056fcbf94fe47c2305fb668805c53f";
const EXPECTED_SHA256: &str = "291923d76f2de8a69deec82cc049d233d87d91f816295312ed89f9291e13405e";
const FAMILIES: [&str; 16] = [
    "literal_spend",
    "self_alias",
    "action_alternatives",
    "data_only",
    "output_only",
    "dead_check",
    "script_identity",
    "token_identity",
    "tokenless",
    "exists_literal",
    "computed_index",
    "register_identity",
    "context_code",
    "context_scope",
    "avl_identity",
    "snapshot_collision",
];

fn check_inventory(
    manifest: &[u8],
    expected: &[u8],
    load: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(Value, Value), String> {
    if sha(manifest) != MANIFEST_SHA256 || sha(expected) != EXPECTED_SHA256 {
        return Err(
            "registered denominator/answer digest changed; versioned governing decision required"
                .into(),
        );
    }
    check_structure(manifest, expected, load)
}

fn check_structure(
    manifest: &[u8],
    expected: &[u8],
    load: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(Value, Value), String> {
    let manifest: Value = serde_json::from_slice(manifest).map_err(|e| e.to_string())?;
    let expected: Value = serde_json::from_slice(expected).map_err(|e| e.to_string())?;
    if manifest["version"] != "mapping-inventory:v1" || expected["version"] != "mapping-expected:v1"
    {
        return Err("unknown version".into());
    }
    let expected_ids: BTreeSet<String> = FAMILIES
        .iter()
        .flat_map(|f| [format!("{f}-positive"), format!("{f}-control")])
        .collect();
    for document in [&manifest, &expected] {
        let rows = document["cases"].as_array().ok_or("missing cases")?;
        let ids: BTreeSet<String> = rows
            .iter()
            .map(|r| r["id"].as_str().unwrap_or("").to_owned())
            .collect();
        if rows.len() != 32 {
            return Err("missing member: expected 32 cases".into());
        }
        if ids.len() != rows.len() {
            return Err("duplicate member".into());
        }
        if ids != expected_ids {
            return Err("unregistered member".into());
        }
    }
    // Truth is independently authored, not inferable from JSON shape. Compare
    // answer content to the pinned oracle separately from document digests.
    let oracle: Value = serde_json::from_str(include_str!("fixtures/mapping/expected.json"))
        .map_err(|e| e.to_string())?;
    for answer in expected["cases"].as_array().unwrap() {
        let original = oracle["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == answer["id"])
            .ok_or("unknown answer")?;
        if answer != original {
            return Err("independent answer content changed".into());
        }
    }
    for entry in manifest["cases"].as_array().unwrap() {
        let bytes = load(entry["path"].as_str().unwrap())?;
        if sha(&bytes) != entry["sha256"] {
            return Err(format!("fixture digest changed: {}", entry["id"]));
        }
        let case: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if case["id"] != entry["id"] || case["family"] != entry["family"] {
            return Err("case identity changed".into());
        }
        let tree = hex::decode(case["treeHex"].as_str().ok_or("missing tree")?)
            .map_err(|e| e.to_string())?;
        if sha(case["source"].as_str().unwrap().as_bytes()) != entry["sourceSha256"]
            || sha(&tree) != entry["treeSha256"]
            || sha(&serde_json::to_vec(&case["universe"]).unwrap()) != entry["canonicalSha256"]
        {
            return Err("source/tree/canonical hash mismatch".into());
        }
    }
    Ok((manifest, expected))
}
fn inventory() -> (Value, Value) {
    check_inventory(
        &std::fs::read(root().join("manifest.json")).unwrap(),
        &std::fs::read(root().join("expected.json")).unwrap(),
        |p| std::fs::read(root().join(p)).map_err(|e| e.to_string()),
    )
    .unwrap()
}
fn diagnostic(path: &str) {
    println!(
        "evidence {} sha256={}",
        root().join(path).display(),
        sha(&std::fs::read(root().join(path)).unwrap())
    );
}

#[test]
fn inventory_has_32_pinned_cases_and_independent_answers() {
    let (m, e) = inventory();
    let mb = std::fs::read(root().join("manifest.json")).unwrap();
    let eb = std::fs::read(root().join("expected.json")).unwrap();
    let disk = |p: &str| std::fs::read(root().join(p)).map_err(|e| e.to_string());
    let mut missing = m.clone();
    missing["cases"].as_array_mut().unwrap().pop();
    let mut duplicate = m.clone();
    duplicate["cases"][1] = duplicate["cases"][0].clone();
    for (invalid, reason) in [
        (missing, "missing member: expected 32 cases"),
        (duplicate, "duplicate member"),
    ] {
        assert_eq!(
            check_structure(&serde_json::to_vec(&invalid).unwrap(), &eb, disk).unwrap_err(),
            reason
        );
    }
    let mut wrong_answer = e.clone();
    wrong_answer["cases"][0]["truth"] = json!("false");
    assert_eq!(
        check_structure(&mb, &serde_json::to_vec(&wrong_answer).unwrap(), disk).unwrap_err(),
        "independent answer content changed"
    );
    // Equivalent reserialization passes structure but still fails the byte pin.
    let serialized = serde_json::to_vec(&m).unwrap();
    check_structure(&serialized, &eb, disk).unwrap();
    assert!(check_inventory(&serialized, &eb, disk)
        .unwrap_err()
        .contains("digest changed"));
    assert_eq!(
        check_inventory(&mb, &eb, |_| Err("fixture missing".into())).unwrap_err(),
        "fixture missing"
    );
    assert!(check_inventory(&mb, &eb, |p| {
        let mut bytes = disk(p)?;
        bytes.push(b' ');
        Ok(bytes)
    })
    .unwrap_err()
    .starts_with("fixture digest changed:"));
    for entry in m["cases"].as_array().unwrap() {
        let c = read(entry["path"].as_str().unwrap());
        let a = e["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == c["id"])
            .unwrap();
        let sites: Vec<Value> = c["sites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let source = c["source"].as_str().unwrap();
                assert_eq!(
                    &source[s["start"].as_u64().unwrap() as usize
                        ..s["end"].as_u64().unwrap() as usize],
                    s["text"].as_str().unwrap()
                );
                s["id"].clone()
            })
            .collect();
        assert_eq!(json!(sites), a["referenceSiteIds"]);
        assert!(!sites.is_empty());
        let ids: Vec<&Value> = c["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| &v["id"])
            .collect();
        assert_eq!(
            ids,
            a["actions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| &v["id"])
                .collect::<Vec<_>>()
        );
        assert_eq!(
            ids.len(),
            ids.iter()
                .map(|id| id.as_str().unwrap())
                .collect::<BTreeSet<_>>()
                .len()
        );
        assert!(!a["permittedPremises"].as_array().unwrap().is_empty());
    }
    diagnostic("manifest.json");
    diagnostic("expected.json");
    println!("32 cases / 16 paired families; missing, duplicate, altered fixture and altered answer rejected");
}

#[test]
fn legacy_metrics_are_measured_without_baseline_changes() {
    let (m, e) = inventory();
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    for (path, digest) in m["baselineArtifacts"].as_object().unwrap() {
        assert_eq!(
            sha(&std::fs::read(workspace.join(path)).unwrap()),
            digest.as_str().unwrap(),
            "baseline changed: {path}"
        );
    }
    let roadmap = std::fs::read_to_string(workspace.join("docs/ROADMAP.md")).unwrap();
    let block = roadmap
        .split("<!-- roadmap-policy:v1 -->")
        .nth(1)
        .unwrap()
        .split("```json")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let policy: Value = serde_json::from_str(block).unwrap();
    authenticate_policy(&policy, m["protectedPolicySha256"].as_str().unwrap()).unwrap();
    let measured = measure(&m, &e);
    assert_eq!(
        measured,
        read("legacy-results.json"),
        "legacy results must equal real API output and registered member IDs"
    );
    diagnostic("legacy-results.json");
    println!("{}", serde_json::to_string(&measured["metrics"]).unwrap());
}

#[test]
fn fixture_constructs_execute_and_unsupported_members_remain() {
    let (m, e) = inventory();
    let mut count = 0;
    let mut unsupported = vec![];
    let cargo = std::fs::read_to_string(root().join("../../../../Cargo.toml")).unwrap();
    assert!(cargo.contains(m["nodeRevision"].as_str().unwrap()));
    for (entry, answer) in m["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(e["cases"].as_array().unwrap())
    {
        let c = read(entry["path"].as_str().unwrap());
        assert_eq!(
            hex::encode(compile(c["source"].as_str().unwrap())),
            c["treeHex"]
        );
        let mut box_ids = BTreeSet::new();
        for b in c["universe"].as_array().unwrap() {
            let spec = wire_spec(&b["spec"]);
            let wire = canonical_box(&spec, b["reference"]["index"].as_u64().unwrap() as u16);
            assert_eq!(
                hex::encode(compile(b["source"].as_str().unwrap())),
                b["spec"]["propositionHex"]
            );
            assert_eq!(hex::encode(wire.bytes().unwrap()), b["bytes"]);
            assert_eq!(wire.id().unwrap(), b["boxId"]);
            assert!(box_ids.insert(wire.id().unwrap()));
        }
        if answer["supported"] == false {
            assert!(!answer["unsupportedReason"].as_str().unwrap().is_empty());
            unsupported.push(c["id"].clone());
        }
        for (v, a) in c["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .zip(answer["actions"].as_array().unwrap())
        {
            let result = ergo_sandbox::eval_scenario(&scenario(&c, v)).unwrap();
            assert_eq!(
                serde_json::to_value(result.verdict).unwrap(),
                a["engineVerdict"],
                "{}/{}: {:?}",
                c["id"],
                v["id"],
                result.error
            );
            assert!(
                result.error.is_none(),
                "required construct failed: {:?}",
                result.error
            );
            assert_eq!(result.tree_hex, c["treeHex"]);
            count += 1;
        }
    }
    assert_eq!(count, 67);
    assert_eq!(unsupported.len(), 18);
    println!("{count} real engine vectors across 32 cases; {} unsupported members retained: {unsupported:?}; scenario reduction only, no node acceptance or necessity certification",unsupported.len());
    diagnostic("manifest.json");
    diagnostic("expected.json");
}

// D00/D01/D02 explicitly governed additions are authenticated BEFORE projection.
// Never regenerate the original M00 manifest or anchor from today's policy.
fn authenticate_policy(policy: &Value, anchor: &str) -> Result<(), String> {
    let original_bytes = include_bytes!("../../docs/discovery/original-policy.json");
    let resolution_bytes = include_bytes!("../../docs/discovery/permitted-d00-resolution.json");
    if sha(original_bytes) != "46f295bbcbc99324bea21000755259828d663cac68753cec80779d1d642249b0"
        || sha(resolution_bytes)
            != "9657cd03e5639911635d460db685ed573f1f985c785ebb34a357f2ed8ef28286"
    {
        return Err("authentication snapshot changed".into());
    }
    let original: Value = serde_json::from_slice(original_bytes).map_err(|e| e.to_string())?;
    let resolution: Value = serde_json::from_slice(resolution_bytes).map_err(|e| e.to_string())?;
    let mut allowed = original.clone();
    let implemented = match policy["units"]
        .as_array()
        .and_then(|u| u.last())
        .map(|u| &u["implemented"])
    {
        Some(Value::Bool(b)) => *b,
        _ => return Err("invalid implementation flag".into()),
    };
    allowed["units"].as_array_mut().unwrap().push(json!({
        "id":"D00","depends":["M04"],"days":2,"package":"ergo-sandbox",
        "target":"property_inventory","tests":["property_inventory_pins_24_cases_and_independent_answers",
        "reference_executions_and_legacy_results_are_reproduced","transfer_registration_and_exposure_are_accounted"],
        "implemented":true
    }));
    allowed["units"].as_array_mut().unwrap().push(json!({
        "id":"D01","depends":["D00"],"days":2,"package":"ergo-sandbox",
        "target":"property_schema","tests":["property_versions_units_and_limits_fail_closed",
        "bindings_never_infer_missing_roles_or_authority","declaration_identity_binds_all_semantic_premises"],
        "implemented":true
    }));
    allowed["units"].as_array_mut().unwrap().push(json!({
        "id":"D02","depends":["D01"],"days":3,"package":"ergo-sandbox",
        "target":"property_evaluation","tests":["four_property_families_match_independent_operands",
        "guards_missing_fields_and_overflow_preserve_unknowns","bounded_response_requires_a_complete_accepted_linked_trace",
        "property_evaluation_requires_fresh_node_acceptance"],"implemented":implemented
    }));
    allowed["completedThrough"] = json!(if implemented { "D02" } else { "D01" });
    allowed["resolvedStopRecords"]
        .as_array_mut()
        .unwrap()
        .push(resolution);
    if policy != &allowed {
        return Err("unauthorized protected policy change".into());
    }
    let mut projected = policy.clone();
    projected["units"] = json!(original["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["id"].as_str().unwrap().starts_with('P'))
        .collect::<Vec<_>>());
    projected["resolvedStopRecords"] = original["resolvedStopRecords"].clone();
    projected["completedThrough"] = json!("P08");
    if sha(&serde_json::to_vec(&projected).unwrap()) != anchor {
        return Err("original M00 policy anchor mismatch".into());
    }
    Ok(())
}

#[test]
fn protected_policy_rejects_unauthorized_additions_and_mutations() {
    let text = include_str!("../../docs/ROADMAP.md");
    let block = text
        .split("<!-- roadmap-policy:v1 -->")
        .nth(1)
        .unwrap()
        .split("```json")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let p: Value = serde_json::from_str(block).unwrap();
    let anchor = "41729240759244bd3edc833e31359bfe50d8430f0b5a2e27ebc727706363cd10";
    authenticate_policy(&p, anchor).unwrap();
    // Exercise both legal scheduling states and the real checker for all mutations.
    for implemented in [false, true] {
        let mut valid = p.clone();
        valid["units"].as_array_mut().unwrap().last_mut().unwrap()["implemented"] =
            json!(implemented);
        valid["completedThrough"] = json!(if implemented { "D02" } else { "D01" });
        authenticate_policy(&valid, anchor).unwrap();
    }
    let reject = |bad: Value| assert!(authenticate_policy(&bad, anchor).is_err());
    // Type-preserving mutations reach valid-looking numeric thresholds, flags,
    // IDs, dependencies, exact test names and resolution hashes, recursively.
    fn mutations(v: &Value) -> Vec<Value> {
        let mut out = vec![];
        match v {
            Value::Bool(b) => out.push(json!(!b)),
            Value::Number(n) => out.push(json!(n.as_f64().unwrap() + 1.0)),
            Value::String(s) => out.push(json!(format!("{s}0"))),
            Value::Array(a) => {
                for (i, member) in a.iter().enumerate() {
                    for altered in mutations(member) {
                        let mut bad = v.clone();
                        bad[i] = altered;
                        out.push(bad);
                    }
                    let mut bad = v.clone();
                    bad.as_array_mut().unwrap().remove(i);
                    out.push(bad);
                }
            }
            Value::Object(o) => {
                for (key, member) in o {
                    for altered in mutations(member) {
                        let mut bad = v.clone();
                        bad[key] = altered;
                        out.push(bad);
                    }
                }
            }
            Value::Null => out.push(json!(0)),
        }
        out
    }
    let typed = mutations(&p);
    let mutation_count = typed.len();
    for bad in typed {
        reject(bad);
    }
    println!("{mutation_count} type-preserving field/member mutations rejected");
    // Every top-level protected field, not just fields included in today's digest.
    for key in p.as_object().unwrap().keys() {
        let mut bad = p.clone();
        bad.as_object_mut().unwrap().remove(key);
        reject(bad);
        let mut bad = p.clone();
        bad[key] = json!("tampered");
        reject(bad);
    }
    let mut bad = p.clone();
    bad["unexpected"] = json!(true);
    reject(bad);
    for collection in ["units", "resolvedStopRecords"] {
        for i in 0..p[collection].as_array().unwrap().len() {
            let mut bad = p.clone();
            bad[collection].as_array_mut().unwrap().remove(i);
            reject(bad);
            let mut bad = p.clone();
            bad[collection]
                .as_array_mut()
                .unwrap()
                .push(p[collection][i].clone());
            reject(bad);
            for key in p[collection][i].as_object().unwrap().keys() {
                let mut bad = p.clone();
                bad[collection][i][key] = json!("tampered");
                reject(bad);
            }
            let mut bad = p.clone();
            bad[collection][i]["unexpected"] = json!(true);
            reject(bad);
        }
        let mut bad = p.clone();
        bad[collection].as_array_mut().unwrap().reverse();
        reject(bad);
    }
    let mut bad = p.clone();
    let mut d01 = p["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "D01")
        .unwrap()
        .clone();
    d01["id"] = json!("D02");
    bad["units"].as_array_mut().unwrap().push(d01);
    reject(bad);
    let mut bad = p.clone();
    bad["resolvedStopRecords"]
        .as_array_mut()
        .unwrap()
        .push(json!({"unit":"M05"}));
    reject(bad);
    // Even a coherent rollback of the previously permitted D00 scheduling
    // pair is now a protected-field mutation, not another allowed addition.
    let mut rollback = p.clone();
    let units = rollback["units"].as_array_mut().unwrap();
    units.iter_mut().find(|u| u["id"] == "D00").unwrap()["implemented"] = json!(false);
    rollback["completedThrough"] = json!("M04");
    reject(rollback);
    let mut rollback = p.clone();
    rollback["units"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|u| u["id"] == "D01")
        .unwrap()["implemented"] = json!(false);
    rollback["completedThrough"] = json!("D00");
    reject(rollback);
    // Keep the exact D01-shaped append rejections, and add D02-shaped probes.
    for source in ["D01", "D02"] {
        for id in ["D02", "D03", "D04"] {
            let mut bad = p.clone();
            let mut unit = p["units"]
                .as_array()
                .unwrap()
                .iter()
                .find(|u| u["id"] == source)
                .unwrap()
                .clone();
            unit["id"] = json!(id);
            unit["implemented"] = json!(false);
            bad["units"].as_array_mut().unwrap().push(unit);
            reject(bad);
        }
    }
    assert!(authenticate_policy(&p, &"0".repeat(64)).is_err());
    println!("original P/M registrations, P05 resolution, exact D00/D01/D02 additions, every protected field: tampering rejected by the real authentication function");
}
