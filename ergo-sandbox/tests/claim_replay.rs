use ergo_primitives::reader::VlqReader;
use ergo_sandbox::evidence::{
    claim::PROPERTY_VERSION,
    replay::{replay, ReplayBundle},
    wire::WireBox,
};
use ergo_ser::transaction::{read_transaction, transaction_id};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
    process::Command,
};
#[path = "../../docs/p05-recovery/derive/src/recover.rs"]
mod use_recovery;
fn backed_use(bundle: &ReplayBundle) -> bool {
    static RECOVERED: std::sync::OnceLock<ergo_sandbox::evidence::validate::ValidationRequest> =
        std::sync::OnceLock::new();
    let expected = RECOVERED.get_or_init(|| {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut request = use_recovery::recover(repo).pop().unwrap();
        let archive: Value = serde_json::from_slice(
            &std::fs::read(repo.join("docs/p05-stop-evidence/public-transaction.fixture")).unwrap(),
        )
        .unwrap();
        let mut premises = request.case.premises().clone();
        premises.assumptions.insert(
            "archivedTransaction".into(),
            ergo_sandbox::evidence::Premise::supplied(archive),
        );
        request.case = ergo_sandbox::evidence::EvidenceCase::new(premises).unwrap();
        request
    });
    serde_json::to_value(&bundle.execution).unwrap() == serde_json::to_value(expected).unwrap()
}
fn policy() -> Value {
    let s = include_str!("../../docs/ROADMAP.md");
    serde_json::from_str(
        s.split("<!-- roadmap-policy:v1 -->")
            .nth(1)
            .unwrap()
            .split("```json")
            .nth(1)
            .unwrap()
            .split("```")
            .next()
            .unwrap(),
    )
    .unwrap()
}
fn fixtures() -> Vec<(Value, ReplayBundle)> {
    let p = policy();
    let unit = p["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "P05")
        .unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let path = repo.join(unit["fixtureManifest"].as_str().unwrap());
    let root = path.parent().unwrap();
    let manifest: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(manifest["formatVersion"], 1);
    let rows = manifest["cases"].as_array().unwrap();
    let ids = rows
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let required = unit["caseIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids, required);
    assert_eq!(rows.len(), ids.len());
    let mut loaded = vec![];
    // All files/revisions authenticated before any case is executed.
    for row in rows {
        assert_eq!(
            row["nodeRevision"],
            ergo_sandbox::evidence::validate::node_revision()
        );
        assert_eq!(row["propertyVersion"], PROPERTY_VERSION);
        assert_eq!(
            row["publicationEligibility"],
            "public-authored-or-public-incident"
        );
        assert_eq!(row["claimStatus"], row["expectedStatus"]);
        let mut bundle = None;
        for file in row["files"].as_array().unwrap() {
            let path = Path::new(file["path"].as_str().unwrap());
            assert!(path.components().all(|c| matches!(c, Component::Normal(_))));
            let full = root.join(path).canonicalize().unwrap();
            assert!(full.starts_with(root.canonicalize().unwrap()));
            let bytes = std::fs::read(full).unwrap();
            assert_eq!(hex::encode(Sha256::digest(&bytes)), file["sha256"]);
            if file["role"] == "bundle" {
                assert!(bundle.is_none());
                let value: Value = serde_json::from_slice(&bytes).unwrap();
                let b: ReplayBundle = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(serde_json::to_value(&b).unwrap(), value);
                assert_eq!(b.fingerprint(), row["expectedFingerprint"]);
                assert_eq!(
                    b.execution.case.premises().engine_revision,
                    row["nodeRevision"]
                );
                bundle = Some(b);
            }
        }
        let bundle = bundle.unwrap();
        let records = bundle.execution.case.premises().boxes.value().unwrap();
        let expected_origin = if row["id"] == "use-incident" {
            "source-recorded"
        } else {
            "hypothetical"
        };
        for record in records {
            assert_eq!(
                serde_json::to_value(record).unwrap()["origin"],
                expected_origin
            );
        }
        if row["id"] == "use-incident" {
            assert!(
                backed_use(&bundle),
                "public incident requires complete freshly derived source-backed context"
            );
            let archive: Value = serde_json::from_slice(
                &std::fs::read(root.join("claim-vectors/use-archive.fixture")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                bundle.execution.case.premises().assumptions["archivedTransaction"]
                    .value()
                    .unwrap(),
                &archive
            );
        } else {
            let name = if row["id"] == "sale-mutant-unpaid" {
                "sale-mutant.es"
            } else {
                "sale-fixed.es"
            };
            let source = std::fs::read_to_string(root.join("claim-vectors").join(name)).unwrap();
            assert_eq!(
                bundle.execution.case.premises().assumptions["authoredSource"]
                    .value()
                    .unwrap()["text"],
                source
            );
            let tree =
                ergo_sandbox::compile_source(&source, 0, ergo_ser::address::NetworkPrefix::Mainnet)
                    .unwrap()
                    .tree_bytes;
            assert_eq!(
                WireBox::from_record(records[0].clone())
                    .unwrap()
                    .node()
                    .candidate
                    .ergo_tree_bytes(),
                tree
            );
        }
        loaded.push((row.clone(), bundle));
    }
    loaded
}
#[test]
fn claim_requires_accepted_execution_and_violated_property() {
    let p = policy();
    let unit = p["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == "P05")
        .unwrap();
    let mut accepted = BTreeSet::new();
    let mut transactions = BTreeSet::new();
    let mut positives = 0;
    let mut controls = 0;
    let mut families = BTreeSet::new();
    let mut public_incidents = 0;
    let mut negative_claims = 0;
    let mut recorded = 0;
    let mut hypothetical = 0;
    for (row, bundle) in fixtures() {
        let r = replay(&bundle);
        assert_eq!(r["status"], row["expectedStatus"], "{}: {r}", row["id"]);
        assert_eq!(r["nodeValidated"], row["expectedNodeAccepted"]);
        if r["nodeValidated"] == true {
            accepted.insert(bundle.fingerprint());
            transactions.insert(r["execution"]["transactionId"].as_str().unwrap().to_owned());
            assert_eq!(
                r["execution"]["transactionId"],
                row["expectedTransactionId"]
            );
            assert_eq!(r["inputProvenance"], row["sourceKind"]);
        }
        if unit["positiveCaseIds"]
            .as_array()
            .unwrap()
            .contains(&row["id"])
        {
            assert_eq!(r["status"], "confirmed-violation");
        }
        if unit["negativeCaseIds"]
            .as_array()
            .unwrap()
            .contains(&row["id"])
            && r["status"] == "confirmed-violation"
        {
            negative_claims += 1;
        }
        if r["status"] == "confirmed-violation" {
            assert_eq!(r["nodeValidated"], true);
            assert_eq!(r["claim"]["bundleFingerprint"], bundle.fingerprint());
            assert!(!r["claim"]["accounting"]["extracted"]
                .as_object()
                .unwrap()
                .is_empty());
            positives += 1;
            families.insert(row["family"].as_str().unwrap().to_owned());
            if row["family"] == "public-use-incident" {
                public_incidents += 1;
            }
        }
        if r["status"] == "accepted-nonviolating" {
            controls += 1;
            assert!(r["accounting"]["extracted"].as_object().unwrap().is_empty());
            assert!(r.get("claim").is_none());
        }
        if row["sourceKind"] == "source-recorded-inputs" {
            recorded += 1;
            let archive = bundle.execution.case.premises().assumptions["archivedTransaction"]
                .value()
                .unwrap();
            let tx = read_transaction(&mut VlqReader::new(
                &hex::decode(&bundle.execution.transaction_bytes).unwrap(),
            ))
            .unwrap();
            assert_eq!(hex::encode(transaction_id(&tx).unwrap()), archive["id"]);
            for (i, b) in bundle
                .execution
                .case
                .premises()
                .boxes
                .value()
                .unwrap()
                .iter()
                .enumerate()
            {
                let b = WireBox::from_record(b.clone()).unwrap();
                let a = &archive["inputs"][i];
                assert!(
                    a.get("spendingProof").is_some(),
                    "archive proof field must be present"
                );
                assert_eq!(b.id().unwrap(), a["boxId"]);
                assert_eq!(b.node().candidate.value, a["value"].as_u64().unwrap());
                assert_eq!(
                    hex::encode(b.node().candidate.ergo_tree_bytes()),
                    a["ergoTree"]
                );
                let tokens = b
                    .node()
                    .candidate
                    .tokens
                    .iter()
                    .map(|t| (hex::encode(t.token_id.as_bytes()), t.amount))
                    .collect::<Vec<_>>();
                let archived_tokens = a["assets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| {
                        (
                            t["tokenId"].as_str().unwrap().to_owned(),
                            t["amount"].as_u64().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                assert_eq!(tokens, archived_tokens);
                let regs = a["additionalRegisters"].as_object().unwrap();
                let mut bytes = vec![regs.len() as u8];
                for n in 4..4 + regs.len() {
                    bytes.extend(
                        hex::decode(regs[&format!("R{n}")]["serializedValue"].as_str().unwrap())
                            .unwrap(),
                    );
                }
                assert_eq!(b.node().candidate.register_bytes(), bytes);

                assert_eq!(
                    hex::encode(b.node().transaction_id),
                    a["outputTransactionId"]
                );
                assert_eq!(
                    u64::from(b.node().index),
                    a["outputIndex"].as_u64().unwrap()
                );
                assert_eq!(
                    u64::from(b.node().candidate.creation_height),
                    a["outputCreatedAt"].as_u64().unwrap()
                );
                assert_eq!(
                    hex::encode(&tx.inputs[i].spending_proof.proof),
                    a["spendingProof"].as_str().unwrap_or("")
                );
                assert!(tx.inputs[i].spending_proof.extension().is_empty());
            }
            assert_eq!(
                r["execution"]["request"]["blockContext"]["origin"],
                "source-recorded"
            );
        } else {
            hypothetical += 1;
        }
    }
    let t = &p["thresholds"];
    assert!(accepted.len() as u64 >= t["acceptedBundlesMin"].as_u64().unwrap());
    assert!(transactions.len() as u64 >= t["acceptedBundlesMin"].as_u64().unwrap());
    assert!(positives >= t["confirmedPropertyClaimsMin"].as_u64().unwrap());
    assert!(families.len() as u64 >= t["claimFamiliesMin"].as_u64().unwrap());
    assert!(public_incidents >= t["publicIncidentClaimMin"].as_u64().unwrap());
    assert!(controls >= t["acceptedNonviolatingControlMin"].as_u64().unwrap());
    assert!(negative_claims <= t["benignConfirmedViolationsMax"].as_u64().unwrap());
    println!("accepted bundles: {}; positive claims: {positives}; families: {}; accepted nonviolating controls: {controls}; negative-control claims: {negative_claims}; recorded-input cases: {recorded}; hypothetical-input cases: {hypothetical}",accepted.len(),families.len());
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let rejected: ReplayBundle = serde_json::from_slice(
        &std::fs::read(repo.join("docs/p05-stop-evidence/rejected-use-candidate.fixture")).unwrap(),
    )
    .unwrap();
    assert!(
        !backed_use(&rejected),
        "previous hypothetical green must not satisfy the public gate"
    );
    let good = fixtures()
        .into_iter()
        .find(|(r, _)| r["id"] == "use-incident")
        .unwrap()
        .1;
    for field in [
        "parameters",
        "networkRules",
        "blockContext",
        "headers",
        "priorBlockCost",
    ] {
        let mut changed = serde_json::to_value(&good).unwrap();
        changed["execution"][field] =
            serde_json::to_value(&rejected.execution).unwrap()[field].clone();
        changed["execution"][field]["origin"] = json!("source-recorded");
        let changed: ReplayBundle = serde_json::from_value(changed).unwrap();
        assert!(
            !backed_use(&changed),
            "a source label cannot repair substituted {field}"
        );
    }
    // Fixed source gets a fresh hypothetical box ID, never a patched real ID.
    let all = fixtures();
    let mutant = &all
        .iter()
        .find(|(r, _)| r["id"] == "sale-mutant-unpaid")
        .unwrap()
        .1;
    let fixed = &all
        .iter()
        .find(|(r, _)| r["id"] == "sale-fixed-unpaid")
        .unwrap()
        .1;
    assert_ne!(
        mutant.property.inputs[0].box_id,
        fixed.property.inputs[0].box_id
    );
    let mut incomplete = mutant.clone();
    incomplete.execution.parameters = ergo_sandbox::evidence::Premise::missing("not supplied");
    let r = replay(&incomplete);
    assert_ne!(r["status"], "confirmed-violation");
    assert_eq!(r["nodeValidated"], false);
}
#[test]
fn unknown_policy_cannot_confirm() {
    for (_, mut b) in fixtures() {
        b.property.version = "unrecognized-policy".into();
        let r = replay(&b);
        assert_ne!(r["status"], "confirmed-violation");
        assert!(r.get("claim").is_none());
    }
    let (_, mut b) = fixtures().remove(0);
    b.property.objective = None;
    assert_eq!(replay(&b)["status"], "unsupported-property");
    let mut json = serde_json::to_value(&b).unwrap();
    json["property"]["guessedAuthorization"] = json!(true);
    assert!(serde_json::from_value::<ReplayBundle>(json).is_err());
}
#[test]
fn offline_replay_reproduces_claim() {
    let dir = std::env::temp_dir().join(format!(
        "p05-replay-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    for (row, b) in fixtures() {
        let r = replay(&b);
        let path = dir.join("case.json");
        std::fs::write(&path, serde_json::to_vec(&b).unwrap()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
            .args(["replay", path.to_str().unwrap(), "--json"])
            .current_dir(&dir)
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .output()
            .unwrap();
        let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(actual, r);
        assert_eq!(output.status.success(), row["expectedNodeAccepted"] == true);
        assert!(serde_json::from_value::<ReplayBundle>(r.clone()).is_err());
        let imported: ReplayBundle = serde_json::from_value(r["bundle"].clone()).unwrap();
        assert_eq!(replay(&imported), r);
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let (_, original) = fixtures()
        .into_iter()
        .find(|(r, _)| r["id"] == "use-incident")
        .unwrap();
    let mut value = serde_json::to_value(original).unwrap();
    for b in value["execution"]["case"]["premises"]["boxes"]["value"]
        .as_array_mut()
        .unwrap()
    {
        b["record"]["locator"] = json!(format!(
            "http://{}/must-not-fetch",
            listener.local_addr().unwrap()
        ));
    }
    let altered: ReplayBundle = serde_json::from_value(value).unwrap();
    let path = dir.join("url-case.json");
    std::fs::write(&path, serde_json::to_vec(&altered).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(["replay", path.to_str().unwrap(), "--json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        replay(&altered)
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn wrong_companion_or_policy_invalidates_claim() {
    let (_, original) = fixtures()
        .into_iter()
        .find(|(r, _)| r["id"] == "use-incident")
        .unwrap();
    let old = replay(&original);
    assert_eq!(old["status"], "confirmed-violation");
    let mut changed = original.clone();
    changed.property.inputs[1].box_id = "00".repeat(32);
    assert_ne!(changed.fingerprint(), original.fingerprint());
    assert_eq!(replay(&changed)["status"], "unsupported-property");
    let mut changed = original.clone();
    changed.property.objective = None;
    assert_ne!(changed.fingerprint(), original.fingerprint());
    assert_eq!(replay(&changed)["status"], "unsupported-property");
    let mut changed = original.clone();
    let mut p = changed.execution.case.premises().clone();
    let mut boxes = p.boxes.value().unwrap().clone();
    boxes.remove(1);
    p.boxes = ergo_sandbox::evidence::Premise::supplied(boxes);
    changed.execution.case = ergo_sandbox::evidence::EvidenceCase::new(p).unwrap();
    assert_eq!(replay(&changed)["status"], "node-rejected");
    // A legitimately different declared allowance changes both identity and result.
    let (_, mut sale) = fixtures()
        .into_iter()
        .find(|(r, _)| r["id"] == "sale-mutant-unpaid")
        .unwrap();
    let old = sale.fingerprint();
    sale.property.objective.as_mut().unwrap().terms[1].payment = None;
    assert_ne!(sale.fingerprint(), old);
    assert_eq!(replay(&sale)["status"], "accepted-nonviolating");
}
