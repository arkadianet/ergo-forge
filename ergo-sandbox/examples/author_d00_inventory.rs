//! One-time offline fixture serialization, no validator or property evaluator.
use ergo_primitives::writer::VlqWriter;
use ergo_sandbox::evidence::{
    validate::ValidationRequest,
    wire::{CandidateSpec, CreationReference, TokenSpec, WireBox, WireTransaction},
    EvidenceCase, Origin, Premise,
};
use ergo_ser::{
    input::{ContextExtension, Input, SpendingProof},
    sigma_value::write_constant,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
fn sha(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}
fn hyp<T>(value: T) -> Premise<T> {
    Premise::Present {
        value,
        origin: Origin::Hypothetical,
    }
}
fn spec(
    b: &Value,
    sources: &BTreeMap<String, String>,
    mint: Option<&str>,
    height: u32,
) -> CandidateSpec {
    let mut w = VlqWriter::new();
    w.put_u8(b["registers"].as_array().unwrap().len() as u8);
    for r in b["registers"].as_array().unwrap() {
        let (t, v) =
            ergo_sandbox::scenario::parse_typed_value(r["type"].as_str().unwrap(), &r["value"])
                .unwrap();
        write_constant(&mut w, &t, &v).unwrap();
    }
    CandidateSpec {
        value: b["value"].as_u64().unwrap(),
        ergo_tree: sources[b["sourceId"].as_str().unwrap()].clone(),
        creation_height: height,
        registers: hex::encode(w.result()),
        tokens: b["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| TokenSpec {
                id: if t["id"] == "first-input-id" {
                    mint.unwrap().into()
                } else {
                    t["id"].as_str().unwrap().into()
                },
                amount: t["amount"].as_u64().unwrap(),
            })
            .collect(),
    }
}
fn main() {
    let root = Path::new("ergo-sandbox/tests/fixtures/properties");
    assert!(
        !root.join("manifest.json").exists(),
        "accepted inventory cannot be overwritten; use a new version"
    );
    let draft: Value =
        serde_json::from_slice(&fs::read(root.join("authored-inputs.json")).unwrap()).unwrap();
    let sources: BTreeMap<String, String> = draft["sources"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, s)| {
            (
                name.clone(),
                hex::encode(
                    ergo_sandbox::compile_source(
                        s.as_str().unwrap(),
                        0,
                        ergo_ser::address::NetworkPrefix::Mainnet,
                    )
                    .unwrap()
                    .tree_bytes,
                ),
            )
        })
        .collect();
    fs::create_dir_all(root.join("cases")).unwrap();
    let mut entries = vec![];
    for c in draft["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let mut bs: Vec<WireBox> = c["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, b)| {
                WireBox::hypothetical(
                    &spec(b, &sources, None, 100),
                    &CreationReference {
                        transaction_id: sha(id.as_bytes()),
                        index: i as u16,
                    },
                )
                .unwrap()
            })
            .collect();
        let mint = bs[0].id().unwrap();
        let mut references = vec![];
        for (step, outputs) in c["steps"].as_array().unwrap().iter().enumerate() {
            let height = 100 + step as u32;
            let out_specs: Vec<_> = outputs
                .as_array()
                .unwrap()
                .iter()
                .map(|b| spec(b, &sources, Some(&mint), height))
                .collect();
            let tx = WireTransaction::build(
                bs.iter()
                    .map(|b| Input {
                        box_id: b.node().box_id().unwrap(),
                        spending_proof: SpendingProof::new(vec![], ContextExtension::empty())
                            .unwrap(),
                    })
                    .collect(),
                vec![],
                &out_specs,
            )
            .unwrap();
            let mut request: ValidationRequest = serde_json::from_str(include_str!(
                "../tests/fixtures/evidence/node-vectors/accepted-keyless-spend.fixture"
            ))
            .unwrap();
            let mut p = request.case.premises().clone();
            p.target_bytes = hyp(hex::encode(bs[0].node().candidate.ergo_tree_bytes()));
            p.boxes = hyp(bs.iter().map(|b| b.record().clone()).collect());
            p.assumptions.insert("D00RootAndSchedule".into(),hyp(json!({"case":id,"origin":"hypothetical","height":height,"premises":c["rootPremises"],"linkageStatus":"not-checked-D00"})));
            request.case = EvidenceCase::new(p).unwrap();
            request.transaction_bytes = hex::encode(tx.bytes());
            let mut context = request.block_context.value().unwrap().clone();
            context.height = height;
            request.block_context = hyp(context);
            let old_property = json!({"version":"recognized-attacker-receipts-v1","inputs":bs.iter().enumerate().map(|(i,b)|json!({"boxId":b.id().unwrap(),"role":if i==0 {"protected"} else {"attacker"}})).collect::<Vec<_>>(),"attackerPublicKeys":[],"objective":{"terms":[]}});
            let _: ergo_sandbox::evidence::claim::Property =
                serde_json::from_value(old_property.clone()).unwrap();
            let outputs = tx.output_boxes().unwrap();
            references.push(json!({"index":step,"request":request,"transactionId":tx.id(),"inputSpecs":bs.iter().map(|b|json!({"boxId":b.id().unwrap(),"bytes":hex::encode(b.bytes().unwrap())})).collect::<Vec<_>>(),"outputSpecs":out_specs,"outputBoxes":outputs.iter().map(|b|b.record()).collect::<Vec<_>>(),"legacyProperty":old_property}));
            bs = outputs;
        }
        let value = json!({"version":"property-reference:v1","id":id,"family":c["family"],"authoring":c,"sources":draft["sources"],"treeHexBySource":sources,"references":references});
        let path = format!("cases/{id}.fixture");
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(root.join(&path), &bytes).unwrap();
        entries.push(json!({"id":id,"family":c["family"],"path":path,"sha256":sha(&bytes),"propertySha256":sha(&serde_json::to_vec(&c["property"]).unwrap()),"publicationStatus":"public-synthetic-authored","exposure":"author-supplied-regression-only"}));
    }
    let sources_meta:Vec<Value>=draft["sources"].as_object().unwrap().iter().map(|(id,s)|json!({"id":id,"sourceSha256":sha(s.as_str().unwrap().as_bytes()),"treeSha256":sha(&hex::decode(&sources[id]).unwrap())})).collect();
    let mut pins = serde_json::Map::new();
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "ergo-sandbox/Cargo.toml",
        "ergo-sandbox/src/evidence/claim.rs",
        "ergo-sandbox/src/evidence/replay.rs",
        "ergo-sandbox/src/evidence/validate.rs",
        "ergo-sandbox/src/compile.rs",
        "docs/roadmap-metrics.json",
        "examples/mutants/answer-key.json",
        "ergo-sandbox/tests/fixtures/mapping/manifest.json",
        "ergo-sandbox/tests/fixtures/mapping/expected.json",
        "ergo-sandbox/tests/fixtures/mapping/legacy-results.json",
    ] {
        pins.insert(path.into(), json!(sha(&fs::read(path).unwrap())));
    }
    let manifest = json!({"version":"property-inventory:v1","nodeRevision":ergo_sandbox::evidence::validate::node_revision(),"compilerRevision":ergo_sandbox::evidence::validate::node_revision(),"forgeRevision":String::from_utf8(std::process::Command::new("git").args(["rev-parse","HEAD"]).output().unwrap().stdout).unwrap().trim(),"dirtyTree":true,"cases":entries,"sources":sources_meta,"baselineArtifacts":pins,"denominators":{"synthetic":24,"supported":16,"violations":8,"controls":8,"boundary":8,"transfer":null,"eligibleDiscovery":null},"budgets":{"d00PersonDays":2,"nominationPersonHours":4,"expressionNodes":32,"roles":8,"boxesPerCollection":16,"tokensPerBox":8,"transactionsPerTrace":8,"responseHorizonMax":4},"answerFile":"expected.json","answerSha256":sha(&fs::read(root.join("expected.json")).unwrap()),"authoringSha256":sha(&fs::read(root.join("authored-inputs.json")).unwrap()),"transferSha256":sha(&fs::read(root.join("transfer-registration.json")).unwrap()),"reviewStatus":"incomplete-independent-human-review-missing","newPropertyProducer":"unsupported-not-implemented","discoveryCredit":0});
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    println!("Frozen {} reference rows without running validator or property evaluator; manifest {} answers {}",manifest["cases"].as_array().unwrap().len(),sha(&fs::read(root.join("manifest.json")).unwrap()),manifest["answerSha256"]);
}
