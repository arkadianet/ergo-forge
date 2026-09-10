use ergo_primitives::{digest::ModifierId, reader::VlqReader, writer::VlqWriter};
use ergo_ser::{
    ergo_box::{serialize_ergo_box, ErgoBox, ErgoBoxCandidate},
    ergo_tree::read_ergo_tree,
    input::{ContextExtension, DataInput, Input, SpendingProof},
    register::read_registers,
    sigma_type::SigmaType,
    sigma_value::SigmaValue,
    token::{Token, TokenId},
    transaction::{bytes_to_sign, transaction_id, write_transaction, Transaction},
};
use serde_json::json;
fn main() {
    let tree = hex::decode("10010101d17300").unwrap();
    let regs = hex::decode("01040e").unwrap();
    let candidate = ErgoBoxCandidate::try_from_raw_parts(
        10_000_000,
        read_ergo_tree(&mut VlqReader::new(&tree)).unwrap(),
        tree.clone(),
        700_000,
        vec![Token {
            token_id: TokenId::from_bytes([0x33; 32]),
            amount: 9,
        }],
        read_registers(&mut VlqReader::new(&regs)).unwrap(),
        regs,
    )
    .unwrap();
    let a = ErgoBox {
        candidate: candidate.clone(),
        transaction_id: ModifierId::from_bytes([0x11; 32]),
        index: 0,
    };
    let b = ErgoBox {
        candidate: candidate.clone(),
        transaction_id: ModifierId::from_bytes([0x22; 32]),
        index: 7,
    };
    let mut extension = ContextExtension::empty();
    extension
        .values
        .insert(0, (SigmaType::SInt, SigmaValue::Int(7)));
    let mut output = candidate.clone();
    output.value = 9_000_000;
    let tx = Transaction {
        inputs: vec![Input {
            box_id: a.box_id().unwrap(),
            spending_proof: SpendingProof::new(vec![1, 2, 3], extension).unwrap(),
        }],
        data_inputs: vec![DataInput {
            box_id: b.box_id().unwrap(),
        }],
        output_candidates: vec![output],
    };
    let mut w = VlqWriter::new();
    write_transaction(&mut w, &tx).unwrap();
    let mut outputs = vec![];
    let id = transaction_id(&tx).unwrap();
    for (i, c) in tx.output_candidates.iter().enumerate() {
        let o = ErgoBox {
            candidate: c.clone(),
            transaction_id: id,
            index: i as u16,
        };
        outputs.push(json!({"bytes":hex::encode(serialize_ergo_box(&o).unwrap()),"boxId":hex::encode(o.box_id().unwrap().as_bytes())}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"formatVersion":1,"engineRevision":"9468043396e5daa2828211bcff4234bc70fae4f0","origin":"hypothetical","nodeValidated":false,"description":"Authored codec vectors generated directly with pinned ergo-ser APIs; proof 010203 is dummy material, not an acceptance witness.","candidate":{"value":10000000,"ergoTree":hex::encode(tree),"creationHeight":700000,"tokens":[{"id":"33".repeat(32),"amount":9}],"registers":"01040e"},"boxes":[{"transactionId":"11".repeat(32),"index":0,"bytes":hex::encode(serialize_ergo_box(&a).unwrap()),"boxId":hex::encode(a.box_id().unwrap().as_bytes())},{"transactionId":"22".repeat(32),"index":7,"bytes":hex::encode(serialize_ergo_box(&b).unwrap()),"boxId":hex::encode(b.box_id().unwrap().as_bytes())}],"transactions":[{"bytes":hex::encode(w.result()),"bytesToSign":hex::encode(bytes_to_sign(&tx).unwrap()),"id":hex::encode(id.as_bytes()),"outputs":outputs}]})).unwrap());
}
