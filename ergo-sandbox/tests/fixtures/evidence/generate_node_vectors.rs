//! One-time P03 vector producer. Expected results come from the raw full node
//! pipeline directly, never Forge's validate adapter or txcheck.
use ergo_primitives::{digest::Digest32, writer::VlqWriter};
use ergo_sandbox::evidence::{
    wire::{CandidateSpec, CreationReference, TokenSpec, WireBox},
    CasePremises, EvidenceCase, Origin, Premise,
};
use ergo_ser::{
    ergo_box::ErgoBox,
    input::{ContextExtension, Input, SpendingProof},
    sigma_type::SigmaType,
    sigma_value::SigmaValue,
    transaction::{write_transaction, Transaction},
};
use ergo_validation::{
    context::{LocalPolicy, ProtocolParams, TransactionContext, UtxoView},
    cost::{CostAccumulator, JitCost},
    tx::{
        reemission::ReemissionRuleInputs, validate_transaction, TxValidationCtx, TxValidationRules,
    },
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::Path};
struct State(HashMap<Digest32, ErgoBox>);
impl UtxoView for State {
    fn get_box(&self, id: &Digest32) -> Option<ErgoBox> {
        self.0.get(id).cloned()
    }
}
fn present(v: Value) -> Value {
    json!({"status":"present","origin":"hypothetical","value":v})
}
fn main() {
    let root = Path::new("ergo-sandbox/tests/fixtures/evidence");
    fs::create_dir_all(root.join("node-vectors")).unwrap();
    let rev = ergo_sandbox::evidence::case::engine_revision();
    let ids = [
        ("accepted-keyless-spend", "accepted", "complete"),
        ("duplicate-input-rejection", "DuplicateInput", "structural"),
        (
            "future-output-rejection",
            "OutputFromFuture",
            "output-heights",
        ),
        (
            "canonical-encoding-rejection",
            "NonCanonical",
            "canonical-encoding",
        ),
        (
            "output-constraint-rejection",
            "OutputValueTooLow",
            "structural",
        ),
        ("aggregate-cost-rejection", "CostExceeded", "aggregate-cost"),
        (
            "missing-utxo-rejection",
            "InputBoxNotFound",
            "utxo-resolution",
        ),
        ("storage-rent-acceptance", "accepted", "complete"),
        (
            "reemission-rule-rejection",
            "ReemissionRulesViolated",
            "node-validation",
        ),
    ];
    let mut entries = vec![];
    for (id, expected_variant, stage) in ids {
        let rent = id == "storage-rent-acceptance";
        let reemission = id == "reemission-rule-rejection";
        let params = ProtocolParams::mainnet_default();
        let context = TransactionContext {
            height: if rent { params.storage_period } else { 100 },
            miner_pubkey: hex::decode(
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            )
            .unwrap()
            .try_into()
            .unwrap(),
            pre_header_timestamp: 1_700_000_000_000,
            activated_script_version: 0,
            pre_header_version: 1,
            pre_header_parent_id: [0x44; 32],
            pre_header_n_bits: 117586360,
            pre_header_votes: [0; 3],
        };
        let input_spec = CandidateSpec {
            value: 1_000_000,
            ergo_tree: if rent {
                "10010100d17300"
            } else {
                "10010101d17300"
            }
            .into(),
            creation_height: if rent { 0 } else { 100 },
            tokens: if reemission {
                vec![TokenSpec {
                    id: "33".repeat(32),
                    amount: 1,
                }]
            } else {
                vec![]
            },
            registers: "00".into(),
        };
        let b = WireBox::hypothetical(
            &input_spec,
            &CreationReference {
                transaction_id: "11".repeat(32),
                index: 0,
            },
        )
        .unwrap();
        let mut extension = ContextExtension::empty();
        if rent {
            extension
                .values
                .insert(127, (SigmaType::SShort, SigmaValue::Short(0)));
        }
        let input = Input {
            box_id: b.node().box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![], extension).unwrap(),
        };
        let mut out = input_spec.clone();
        out.ergo_tree = "10010101d17300".into();
        out.creation_height = context.height;
        if id == "future-output-rejection" {
            out.creation_height += 1;
        }
        if id == "output-constraint-rejection" {
            out.value = 1;
        }
        let tx = Transaction {
            inputs: if id == "duplicate-input-rejection" {
                vec![input.clone(), input]
            } else {
                vec![input]
            },
            data_inputs: vec![],
            output_candidates: vec![out.build().unwrap()],
        };
        let mut w = VlqWriter::new();
        write_transaction(&mut w, &tx).unwrap();
        let mut bytes = w.result();
        if id == "canonical-encoding-rejection" {
            assert_eq!(bytes[0], 1);
            bytes.splice(0..1, [0x81, 0]);
        }
        let mut state = State(HashMap::new());
        if id != "missing-utxo-rejection" {
            state.0.insert(b.node().box_id().unwrap(), b.node().clone());
        }
        let prior = if id == "aggregate-cost-rejection" {
            params.max_block_cost - 1_000
        } else {
            0
        };
        let mut cost =
            CostAccumulator::new(JitCost::from_block_cost(params.max_block_cost).unwrap());
        cost.add(JitCost::from_block_cost(prior).unwrap()).unwrap();
        let rules = reemission.then(|| ReemissionRuleInputs {
            activation_height: 99,
            reemission_token_id: [0x33; 32],
            pay_to_reemission_tree: hex::decode("10010101d17300").unwrap(),
        });
        let mut cx = TxValidationCtx {
            ctx: &context,
            params: &params,
            cost: &mut cost,
            last_headers: &[],
            rules: TxValidationRules {
                reemission: rules.as_ref(),
            },
        };
        let expected = match validate_transaction(
            &bytes,
            &state,
            &LocalPolicy {
                max_transaction_size: 524_288,
            },
            &mut cx,
        ) {
            Ok(checked) => {
                assert_eq!(expected_variant, "accepted", "{id}");
                json!({"status":"node-accepted","stage":stage,"transactionId":hex::encode(checked.tx_id()),"totalBlockCost":cost.total_block_cost()})
            }
            Err(e) => {
                let detail = format!("{e:?}");
                assert!(detail.starts_with(expected_variant), "{id}: {detail}");
                json!({"status":"node-rejected","stage":stage,"errorVariant":expected_variant,"detail":detail})
            }
        };
        let mut p = CasePremises::unspecified();
        p.boxes = Premise::Present {
            value: if id == "missing-utxo-rejection" {
                vec![]
            } else {
                vec![b.record().clone()]
            },
            origin: Origin::Hypothetical,
        };
        let request = json!({"formatVersion":1,"case":EvidenceCase::new(p).unwrap(),"transactionBytes":hex::encode(bytes),
   "parameters":present(json!({"minValuePerByte":params.min_value_per_byte,"maxBlockCost":params.max_block_cost,"maxBlockSize":params.max_block_size,"maxBoxSize":params.max_box_size,"maxTokensPerBox":params.max_tokens_per_box,"inputCost":params.input_cost,"dataInputCost":params.data_input_cost,"outputCost":params.output_cost,"tokenAccessCost":params.token_access_cost,"storageFeeFactor":params.storage_fee_factor,"storagePeriod":params.storage_period})),
   "blockContext":present(json!({"height":context.height,"minerPubkey":hex::encode(context.miner_pubkey),"preHeaderTimestamp":context.pre_header_timestamp,"activatedScriptVersion":context.activated_script_version,"preHeaderVersion":context.pre_header_version,"preHeaderParentId":hex::encode(context.pre_header_parent_id),"preHeaderNBits":context.pre_header_n_bits,"preHeaderVotes":context.pre_header_votes})),
   "networkRules":present(if reemission{json!({"mode":"enabled","description":"authored EIP-27 enabled rule vector","activation_height":99,"reemission_token_id":"33".repeat(32),"pay_to_reemission_tree":"10010101d17300"})}else{json!({"mode":"disabled","description":"authored keyless/rent vector","reason":"explicit hypothetical network without EIP-27"})}),
   "headers":present(json!([])),"localPolicy":present(json!({"maxTransactionSize":524288})),"priorBlockCost":present(json!(prior))});
        let path = format!("node-vectors/{id}.fixture");
        let raw = serde_json::to_string_pretty(&request).unwrap() + "\n";
        fs::write(root.join(&path), &raw).unwrap();
        entries.push(json!({"id":id,"family":"transaction-pipeline","sourceKind":"authored-full-node-vector","nodeRevision":rev,"propertyVersion":"none-P03","claimStatus":"no-property-claim","publicationEligibility":"public-authored","files":[{"path":path,"sha256":hex::encode(Sha256::digest(raw.as_bytes()))}],"expected":expected}));
        println!("{id}: {expected}");
    }
    let manifest = json!({"formatVersion":1,"nodeRevision":rev,"producer":"generate_node_vectors.rs; direct ergo_validation::tx::validate_transaction","stateScope":"hypothetical supplied state/rules; no historical or property claims","cases":entries});
    fs::write(
        root.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap() + "\n",
    )
    .unwrap();
}
