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
    let mut policy: Value = serde_json::from_str(block).unwrap();
    policy["units"]
        .as_array_mut()
        .unwrap()
        .retain(|u| !u["id"].as_str().unwrap().starts_with('M'));
    policy["completedThrough"] = json!("P08");
    assert_eq!(
        sha(&serde_json::to_vec(&policy).unwrap()),
        m["protectedPolicySha256"]
    );
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
