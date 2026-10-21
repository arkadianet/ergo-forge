//! One-time producer for the emission-allocation vectors (`emission-vectors/`,
//! `emission-manifest.json`). Expected results come from the raw full node pipeline
//! (`ergo_validation::tx::validate_transaction`) directly, never from Forge's validate adapter or
//! `txcheck`, so the vectors test the adapter's `emission` premise against the node.
//!
//! Copy it to a temporary Cargo example and run it from the repository root, as
//! `README.md` describes for the other producers; it writes new files only.
use ergo_chain_spec::MonetaryParams;
use ergo_primitives::{digest::Digest32, writer::VlqWriter};
use ergo_sandbox::evidence::{
    wire::{CandidateSpec, CreationReference, TokenSpec, WireBox},
    CasePremises, EvidenceCase, Origin, Premise,
};
use ergo_ser::{
    ergo_box::ErgoBox,
    input::{ContextExtension, Input, SpendingProof},
    transaction::{write_transaction, Transaction},
};
use ergo_validation::{
    context::{LocalPolicy, ProtocolParams, TransactionContext, UtxoView},
    cost::{CostAccumulator, JitCost},
    tx::{
        reemission::{EmissionRuleInputs, ReemissionRuleInputs},
        validate_transaction, TxValidationCtx, TxValidationRules,
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

const HEIGHT: u32 = 100;
const ACTIVATION: u32 = 99;
const KEYLESS: &str = "10010101d17300";
/// Above the node's 100,000 ERG emission-box floor.
const EMISSION_BOX_VALUE: u64 = 100_001_000_000_000;
const REWARD_VALUE: u64 = 75_000_000_000;
const TOKEN_IN: u64 = 1_000_000_000_000;
/// `reemission_for_height(100, mainnet monetary, 99)`: emission 75 ERG >= 15 ERG, so 12 ERG.
const CORRECT_REWARD_TOKENS: u64 = 12_000_000_000;

fn main() {
    let root = Path::new("ergo-sandbox/tests/fixtures/evidence");
    fs::create_dir_all(root.join("emission-vectors")).unwrap();
    let rev = ergo_sandbox::evidence::case::engine_revision();
    let monetary = MonetaryParams::mainnet();
    let nft = "22".repeat(32);
    let token = "33".repeat(32);
    let mut entries = vec![];
    for (id, reward_tokens, expected_variant) in [
        // 1,000,000,000 more tokens to the reward box than the schedule allows; the totals still balance.
        (
            "emission-allocation-rejection",
            CORRECT_REWARD_TOKENS + 1_000_000_000,
            "ReemissionRulesViolated",
        ),
        (
            "emission-allocation-acceptance",
            CORRECT_REWARD_TOKENS,
            "accepted",
        ),
    ] {
        let mut params = ProtocolParams::mainnet_default();
        // The adapter takes the block version from the request's activated script version + 1.
        params.block_version = 1;
        let context = TransactionContext {
            height: HEIGHT,
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
            value: EMISSION_BOX_VALUE,
            ergo_tree: KEYLESS.into(),
            creation_height: HEIGHT,
            tokens: vec![
                TokenSpec {
                    id: nft.clone(),
                    amount: 1,
                },
                TokenSpec {
                    id: token.clone(),
                    amount: TOKEN_IN,
                },
            ],
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
        let input = Input {
            box_id: b.node().box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![], ContextExtension::empty()).unwrap(),
        };
        // Output 0 is the next emission box, output 1 the reward box.
        let emission_out = CandidateSpec {
            value: EMISSION_BOX_VALUE - REWARD_VALUE,
            tokens: vec![
                TokenSpec {
                    id: nft.clone(),
                    amount: 1,
                },
                TokenSpec {
                    id: token.clone(),
                    amount: TOKEN_IN - reward_tokens,
                },
            ],
            ..input_spec.clone()
        };
        let reward_out = CandidateSpec {
            value: REWARD_VALUE,
            tokens: vec![TokenSpec {
                id: token.clone(),
                amount: reward_tokens,
            }],
            ..input_spec.clone()
        };
        let tx = Transaction {
            inputs: vec![input],
            data_inputs: vec![],
            output_candidates: vec![emission_out.build().unwrap(), reward_out.build().unwrap()],
        };
        let mut w = VlqWriter::new();
        write_transaction(&mut w, &tx).unwrap();
        let bytes = w.result();
        let mut state = State(HashMap::new());
        state.0.insert(b.node().box_id().unwrap(), b.node().clone());
        let mut cost =
            CostAccumulator::new(JitCost::from_block_cost(params.max_block_cost).unwrap());
        cost.add(JitCost::from_block_cost(0).unwrap()).unwrap();
        let rules = ReemissionRuleInputs {
            check_rules: true,
            emission: Some(EmissionRuleInputs {
                monetary,
                emission_nft_id: [0x22; 32],
                emission_tree: hex::decode(KEYLESS).unwrap(),
            }),
            activation_height: ACTIVATION,
            reemission_token_id: [0x33; 32],
            pay_to_reemission_tree: hex::decode(KEYLESS).unwrap(),
        };
        let mut cx = TxValidationCtx {
            ctx: &context,
            params: &params,
            cost: &mut cost,
            last_headers: &[],
            rules: TxValidationRules {
                reemission: Some(&rules),
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
                json!({"status":"node-accepted","stage":"complete","transactionId":hex::encode(checked.tx_id()),"totalBlockCost":cost.total_block_cost()})
            }
            Err(e) => {
                let detail = format!("{e:?}");
                assert!(detail.starts_with(expected_variant), "{id}: {detail}");
                json!({"status":"node-rejected","stage":"node-validation","errorVariant":expected_variant,"detail":detail})
            }
        };
        let mut p = CasePremises::unspecified();
        p.boxes = Premise::Present {
            value: vec![b.record().clone()],
            origin: Origin::Hypothetical,
        };
        let request = json!({"formatVersion":1,"case":EvidenceCase::new(p).unwrap(),"transactionBytes":hex::encode(bytes),
   "parameters":present(json!({"minValuePerByte":params.min_value_per_byte,"maxBlockCost":params.max_block_cost,"maxBlockSize":params.max_block_size,"maxBoxSize":params.max_box_size,"maxTokensPerBox":params.max_tokens_per_box,"inputCost":params.input_cost,"dataInputCost":params.data_input_cost,"outputCost":params.output_cost,"tokenAccessCost":params.token_access_cost,"storageFeeFactor":params.storage_fee_factor,"storagePeriod":params.storage_period})),
   "blockContext":present(json!({"height":context.height,"minerPubkey":hex::encode(context.miner_pubkey),"preHeaderTimestamp":context.pre_header_timestamp,"activatedScriptVersion":context.activated_script_version,"preHeaderVersion":context.pre_header_version,"preHeaderParentId":hex::encode(context.pre_header_parent_id),"preHeaderNBits":context.pre_header_n_bits,"preHeaderVotes":context.pre_header_votes})),
   "networkRules":present(json!({"mode":"enabled",
       "description":"authored EIP-27 emission-allocation vector; the monetary schedule is the Scala mainnet default (MonetarySettings), the NFT, token and trees are hypothetical",
       "activation_height":ACTIVATION,"reemission_token_id":token,"pay_to_reemission_tree":KEYLESS,
       "emission":{"fixedRate":monetary.fixed_rate,"fixedRatePeriod":monetary.fixed_rate_period,"epochLength":monetary.epoch_length,
           "oneEpochReduction":monetary.one_epoch_reduction,"foundersInitialReward":monetary.founders_initial_reward,
           "minerRewardDelay":monetary.miner_reward_delay,"emissionNftId":nft,"emissionTree":KEYLESS}})),
   "headers":present(json!([])),"localPolicy":present(json!({"maxTransactionSize":524288})),"priorBlockCost":present(json!(0))});
        let path = format!("emission-vectors/{id}.fixture");
        let raw = serde_json::to_string_pretty(&request).unwrap() + "\n";
        fs::write(root.join(&path), &raw).unwrap();
        entries.push(json!({"id":id,"family":"transaction-pipeline","sourceKind":"authored-full-node-vector","nodeRevision":rev,"propertyVersion":"none-emission","claimStatus":"no-property-claim","publicationEligibility":"public-authored","files":[{"path":path,"sha256":hex::encode(Sha256::digest(raw.as_bytes()))}],"expected":expected}));
        println!("{id}: {expected}");
    }
    let manifest = json!({"formatVersion":1,"nodeRevision":rev,"producer":"generate_emission_vectors.rs; direct ergo_validation::tx::validate_transaction","stateScope":"hypothetical supplied state/rules; no historical or property claims","cases":entries});
    fs::write(
        root.join("emission-manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap() + "\n",
    )
    .unwrap();
}
