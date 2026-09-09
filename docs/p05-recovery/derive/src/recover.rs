use ergo_primitives::{group_element::GroupElement, reader::VlqReader};
use ergo_sandbox::evidence::{
    validate::{validate, ValidationRequest},
    wire::{BoxMaterial, CandidateSpec, CreationReference, TokenSpec, WireBox, WireTransaction},
    CasePremises, EvidenceCase,
};
use ergo_ser::{
    autolykos::AutolykosSolution,
    extension::{Extension, ExtensionField},
    header::{serialize_header, Header},
    input::{ContextExtension, Input, SpendingProof},
    sigma_value::read_constant,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn load(p: &str) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn bytes<const N: usize>(v: &Value) -> [u8; N] {
    hex::decode(v.as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap()
}
fn header(v: &Value) -> Header {
    assert_eq!(v["version"], 4);
    Header {
        version: 4,
        parent_id: bytes::<32>(&v["parentId"]).into(),
        ad_proofs_root: bytes::<32>(&v["adProofsRoot"]).into(),
        transactions_root: bytes::<32>(&v["transactionsRoot"]).into(),
        state_root: bytes::<33>(&v["stateRoot"]).into(),
        timestamp: v["timestamp"].as_u64().unwrap(),
        extension_root: bytes::<32>(&v["extensionHash"]).into(),
        n_bits: v["nBits"].as_u64().unwrap().try_into().unwrap(),
        height: v["height"].as_u64().unwrap().try_into().unwrap(),
        votes: bytes(&v["votes"]),
        unparsed_bytes: vec![],
        solution: AutolykosSolution::V2 {
            pk: GroupElement::from_bytes(bytes(&v["powSolutions"]["pk"])),
            nonce: bytes(&v["powSolutions"]["n"]),
        },
    }
}
fn candidate(v: &Value, height: u32) -> CandidateSpec {
    let regs = v["additionalRegisters"].as_object().unwrap();
    let mut register_bytes = vec![regs.len().try_into().unwrap()];
    for i in 4..4 + regs.len() {
        let r = &regs[&format!("R{i}")];
        let r = if r.is_string() {
            r
        } else {
            &r["serializedValue"]
        };
        register_bytes.extend(hex::decode(r.as_str().unwrap()).unwrap());
    }
    CandidateSpec {
        value: v["value"].as_u64().unwrap(),
        ergo_tree: v["ergoTree"].as_str().unwrap().into(),
        creation_height: height,
        tokens: v["assets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| TokenSpec {
                id: t["tokenId"].as_str().unwrap().into(),
                amount: t["amount"].as_u64().unwrap(),
            })
            .collect(),
        registers: hex::encode(register_bytes),
    }
}
pub fn recover(repo: &Path) -> Vec<ValidationRequest> {
    let directory = repo.join("docs/p05-recovery");
    let dir = directory.to_str().unwrap();
    let block = load(&format!("{dir}/node-block.fixture"));
    let epoch = load(&format!("{dir}/epoch-block.fixture"));
    let chain = load(&format!("{dir}/epoch-chain.fixture"));
    let hs = chain.as_array().unwrap();
    assert_eq!(
        hs.len() as u64,
        block["header"]["height"].as_u64().unwrap() - epoch["header"]["height"].as_u64().unwrap()
            + 1
    );
    assert_eq!(hs[0]["id"], epoch["header"]["id"]);
    assert_eq!(hs.last().unwrap()["id"], block["header"]["id"]);
    for (i, h) in hs.iter().enumerate() {
        let (_, id) = serialize_header(&header(h)).unwrap();
        assert_eq!(hex::encode(id.as_bytes()), h["id"]);
        if i > 0 {
            assert_eq!(h["parentId"], hs[i - 1]["id"]);
            assert_eq!(
                h["height"].as_u64().unwrap(),
                hs[i - 1]["height"].as_u64().unwrap() + 1
            );
        }
    }
    // Bind the recorded transaction IDs and proof witnesses to the header commitment.
    // Witness derivation is recorded from pinned ergo-validation/src/block/validate.rs.
    let txs = block["blockTransactions"]["transactions"]
        .as_array()
        .unwrap();
    let ids: Vec<Vec<u8>> = txs
        .iter()
        .map(|t| hex::decode(t["id"].as_str().unwrap()).unwrap())
        .collect();
    let witnesses: Vec<Vec<u8>> = txs
        .iter()
        .map(|t| {
            let proofs: Vec<u8> = t["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|i| {
                    hex::decode(i["spendingProof"]["proofBytes"].as_str().unwrap()).unwrap()
                })
                .collect();
            ergo_primitives::digest::blake2b256(&proofs).as_bytes()[1..].to_vec()
        })
        .collect();
    assert_eq!(
        ergo_crypto::merkle::transactions_root(
            &ids.iter().map(Vec::as_slice).collect::<Vec<_>>(),
            Some(&witnesses.iter().map(Vec::as_slice).collect::<Vec<_>>())
        ),
        bytes::<32>(&block["header"]["transactionsRoot"])
    );
    let e = &epoch["extension"];
    let ext = Extension {
        header_id: bytes::<32>(&e["headerId"]).into(),
        fields: e["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| ExtensionField {
                key: bytes(&f[0]),
                value: hex::decode(f[1].as_str().unwrap()).unwrap(),
            })
            .collect(),
    };
    let fields: Vec<_> = ext
        .fields
        .iter()
        .map(|f| (f.key.as_slice(), f.value.as_slice()))
        .collect();
    assert_eq!(
        ergo_crypto::merkle::extension_root(&fields),
        bytes::<32>(&epoch["header"]["extensionHash"])
    );
    let active = ergo_validation::active_params::parse_active_params(
        &ext,
        epoch["header"]["height"]
            .as_u64()
            .unwrap()
            .try_into()
            .unwrap(),
    )
    .unwrap();
    let p = ergo_validation::ProtocolParams::from_active(&active);
    let spec = ergo_chain_spec::ChainSpec::mainnet();
    let rules = spec.reemission.as_ref().unwrap();
    let trees = spec.emission_script_trees().unwrap();
    let h = &block["header"];
    let context = json!({"height":h["height"],"minerPubkey":h["powSolutions"]["pk"],"preHeaderTimestamp":h["timestamp"],"activatedScriptVersion":active.block_version-1,"preHeaderVersion":h["version"],"preHeaderParentId":h["parentId"],"preHeaderNBits":h["nBits"],"preHeaderVotes":bytes::<3>(&h["votes"])});
    let headers: Vec<String> = hs
        .iter()
        .rev()
        .skip(1)
        .take(10)
        .map(|h| hex::encode(serialize_header(&header(h)).unwrap().0))
        .collect();
    let present = |v: Value| json!({"status":"present","origin":"source-recorded","value":v});
    let params = json!({"minValuePerByte":p.min_value_per_byte,"maxBlockCost":p.max_block_cost,"maxBlockSize":p.max_block_size,"maxBoxSize":p.max_box_size,"maxTokensPerBox":p.max_tokens_per_box,"inputCost":p.input_cost,"dataInputCost":p.data_input_cost,"outputCost":p.output_cost,"tokenAccessCost":p.token_access_cost,"storageFeeFactor":p.storage_fee_factor,"storagePeriod":p.storage_period});
    println!("epoch parameters: {params}");
    let network = json!({"mode":"enabled","description":"Pinned ergo-chain-spec ChainSpec::mainnet; EIP-27, source-recorded network configuration, not a network inferred from defaults","activation_height":rules.activation_height,"reemission_token_id":hex::encode(rules.reemission_token_id.as_bytes()),"pay_to_reemission_tree":hex::encode(trees.pay_to_reemission)});
    let retrievals = load(&format!("{dir}/retrievals.json"));
    for row in retrievals.as_array().unwrap() {
        if let Some(file) = row["file"].as_str() {
            let raw = fs::read(format!("{dir}/{file}")).unwrap();
            assert_eq!(hex::encode(Sha256::digest(raw)), row["sha256"]);
        }
    }
    let mut prior = 0;
    let mut requests = vec![];
    for i in 0..6 {
        let archive = if i < 5 {
            load(&format!("{dir}/prior-{i}.fixture"))
        } else {
            load(
                repo.join("docs/p05-stop-evidence/public-transaction.fixture")
                    .to_str()
                    .unwrap(),
            )
        };
        let tx = &block["blockTransactions"]["transactions"][i];
        assert_eq!(tx["id"], archive["id"]);
        assert_eq!(archive["index"], i);
        assert_eq!(archive["blockId"], h["id"]);
        assert!(tx["dataInputs"].as_array().unwrap().is_empty());
        assert!(archive["dataInputs"].as_array().unwrap().is_empty());
        let mut boxes = vec![];
        let mut inputs = vec![];
        for (j, v) in archive["inputs"].as_array().unwrap().iter().enumerate() {
            let b = WireBox::hypothetical(
                &candidate(
                    v,
                    v["outputCreatedAt"].as_u64().unwrap().try_into().unwrap(),
                ),
                &CreationReference {
                    transaction_id: v["outputTransactionId"].as_str().unwrap().into(),
                    index: v["outputIndex"].as_u64().unwrap().try_into().unwrap(),
                },
            )
            .unwrap();
            assert_eq!(b.id().unwrap(), v["boxId"]);
            assert_eq!(tx["inputs"][j]["boxId"], v["boxId"]);
            let m: BoxMaterial = serde_json::from_value(b.record().document().clone()).unwrap();
            boxes.push(WireBox::recorded(m,format!("recorded explorer transaction {} input {j}; docs/p05-recovery/retrievals.json",archive["id"]),Some(archive["id"].as_str().unwrap().into())).unwrap());
            let proof = &tx["inputs"][j]["spendingProof"];
            let mut extension = ContextExtension::empty();
            for (k, v) in proof["extension"].as_object().unwrap() {
                let raw = hex::decode(v.as_str().unwrap()).unwrap();
                let mut reader = VlqReader::new(&raw);
                let constant = read_constant(&mut reader).unwrap();
                assert!(reader.is_empty());
                extension.values.insert(k.parse().unwrap(), constant);
            }
            inputs.push(Input {
                box_id: bytes::<32>(&v["boxId"]).into(),
                spending_proof: SpendingProof::new(
                    hex::decode(proof["proofBytes"].as_str().unwrap()).unwrap(),
                    extension,
                )
                .unwrap(),
            });
        }
        let outputs: Vec<_> = tx["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| candidate(v, v["creationHeight"].as_u64().unwrap().try_into().unwrap()))
            .collect();
        let wire = WireTransaction::build(inputs, vec![], &outputs).unwrap();
        assert_eq!(wire.id(), tx["id"]);
        let case = wire
            .bind_case(
                &EvidenceCase::new(CasePremises::unspecified()).unwrap(),
                &boxes,
                &[],
            )
            .unwrap();
        let mut request = json!({"formatVersion":1,"case":case,"transactionBytes":hex::encode(wire.bytes()),"parameters":present(params.clone()),"networkRules":present(network.clone()),"blockContext":present(context.clone()),"headers":present(json!(headers)),"localPolicy":{"status":"present","origin":"caller-supplied","value":{"maxTransactionSize":p.max_block_size}},"priorBlockCost":present(json!(prior))});
        request["case"]["premises"]["assumptions"]["contextDerivation"] = json!({"status":"present","origin":"source-recorded","value":{"sourceDirectory":"docs/p05-recovery","retrievalManifest":retrievals,"epochHeader":epoch["header"]["id"],"incidentHeader":h["id"],"priorTransactions":i,"priorCostDerivation":"fresh pinned-node validation of every preceding transaction in order","network":"explicit mainnet chain-spec at pinned revision","headerWindow":"preceding ten linked, node-serialized headers; newest first","membership":"source-recorded, not authenticated historical UTXO inclusion"}});
        let r: ValidationRequest = serde_json::from_value(request).unwrap();
        match validate(&r) {
            Ok(a) => {
                let result = a.report();
                println!(
                    "index {i}: accepted {}, total cost {}",
                    wire.id(),
                    result["totalBlockCost"]
                );
                prior = result["totalBlockCost"].as_u64().unwrap();
                requests.push(r);
            }
            Err(e) => panic!("index {i}: rejected: {}: {}", e.stage, e.detail),
        }
    }
    requests
}
