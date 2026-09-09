mod recover;
use sha2::{Digest, Sha256};
fn main() {
    let repo = std::env::current_dir().unwrap();
    let requests = recover::recover(&repo);
    for (i, r) in requests.iter().enumerate() {
        std::fs::write(
            format!("docs/p05-recovery/derived-request-{i}.fixture"),
            serde_json::to_vec_pretty(r).unwrap(),
        )
        .unwrap();
    }
    if std::env::args().any(|a| a == "--write-claim") {
        let root = repo.join("ergo-sandbox/tests/fixtures/evidence");
        let path = root.join("claim-vectors/use-incident.fixture");
        let mut bundle: ergo_sandbox::evidence::replay::ReplayBundle =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        bundle.execution = requests.last().unwrap().clone();
        let mut p = bundle.execution.case.premises().clone();
        let archive: serde_json::Value = serde_json::from_slice(
            &std::fs::read("docs/p05-stop-evidence/public-transaction.fixture").unwrap(),
        )
        .unwrap();
        p.assumptions.insert(
            "archivedTransaction".into(),
            ergo_sandbox::evidence::Premise::supplied(archive),
        );
        bundle.execution.case = ergo_sandbox::evidence::EvidenceCase::new(p).unwrap();
        let result = ergo_sandbox::evidence::replay::replay(&bundle);
        assert_eq!(result["status"], "confirmed-violation");
        std::fs::write(&path, serde_json::to_vec_pretty(&bundle).unwrap()).unwrap();
        std::fs::copy(
            "docs/p05-stop-evidence/public-transaction.fixture",
            root.join("claim-vectors/use-archive.fixture"),
        )
        .unwrap();
        let mp = root.join("claim-manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&mp).unwrap()).unwrap();
        for row in manifest["cases"].as_array_mut().unwrap() {
            if row["id"] == "use-incident" {
                row["expectedFingerprint"] = bundle.fingerprint().into();
                for f in row["files"].as_array_mut().unwrap() {
                    f["sha256"] = hex::encode(Sha256::digest(
                        std::fs::read(root.join(f["path"].as_str().unwrap())).unwrap(),
                    ))
                    .into();
                }
            }
        }
        std::fs::write(mp, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        println!("USE property claim: {}", result["status"]);
    }
}
