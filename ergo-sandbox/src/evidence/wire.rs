//! Strict node-wire construction. Byte identity is not transaction acceptance.
use ergo_primitives::{digest::ModifierId, reader::VlqReader, writer::VlqWriter};
use ergo_ser::{
    ergo_box::{parse_ergo_box_bytes, serialize_ergo_box, ErgoBox, ErgoBoxCandidate},
    ergo_tree::read_ergo_tree,
    input::{DataInput, Input},
    register::read_registers,
    token::{Token, TokenId},
    transaction::{
        bytes_to_sign, read_transaction, transaction_id, write_transaction, Transaction,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{CasePremises, EvidenceCase, Origin, Premise, RecordedBox};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenSpec {
    pub id: String,
    pub amount: u64,
}

/// Every field is required. Registers are the node's dense R4–R9 wire encoding,
/// including the count byte ("00" means explicitly empty).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateSpec {
    pub value: u64,
    pub ergo_tree: String,
    pub creation_height: u32,
    pub tokens: Vec<TokenSpec>,
    pub registers: String,
}
impl CandidateSpec {
    pub fn build(&self) -> Result<ErgoBoxCandidate, String> {
        let tree_bytes = decode(&self.ergo_tree)?;
        let tree = read_ergo_tree(&mut VlqReader::new(&tree_bytes)).map_err(error)?;
        let register_bytes = decode(&self.registers)?;
        let registers = read_registers(&mut VlqReader::new(&register_bytes)).map_err(error)?;
        let tokens = self
            .tokens
            .iter()
            .map(|t| {
                Ok(Token {
                    token_id: TokenId::from_bytes(id_bytes(&t.id)?),
                    amount: t.amount,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        // The node checks parsed/raw consistency and consumes the entire tree
        // and register encoding. Never use from_trusted_raw_parts here.
        ErgoBoxCandidate::try_from_raw_parts(
            self.value,
            tree,
            tree_bytes,
            self.creation_height,
            tokens,
            registers,
            register_bytes,
        )
        .map_err(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreationReference {
    pub transaction_id: String,
    pub index: u16,
}

/// Original material required to import an allegedly observed box. No defaults
/// and no repair of its claimed ID. The tree is required for the node's
/// standalone parser, including non-size-delimited trees.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoxMaterial {
    pub bytes: String,
    pub ergo_tree: String,
    pub box_id: String,
}

#[derive(Debug, Clone)]
pub struct WireBox {
    node: ErgoBox,
    record: RecordedBox,
}
impl WireBox {
    pub fn hypothetical(
        spec: &CandidateSpec,
        reference: &CreationReference,
    ) -> Result<Self, String> {
        Self::hypothetical_node(spec.build()?, reference)
    }
    fn hypothetical_node(
        candidate: ErgoBoxCandidate,
        reference: &CreationReference,
    ) -> Result<Self, String> {
        let node = ErgoBox {
            candidate,
            transaction_id: ModifierId::from_bytes(id_bytes(&reference.transaction_id)?),
            index: reference.index,
        };
        let bytes = serialize_ergo_box(&node).map_err(error)?;
        let material = BoxMaterial {
            bytes: hex::encode(&bytes),
            ergo_tree: hex::encode(node.candidate.ergo_tree_bytes()),
            box_id: hex::encode(node.box_id().map_err(error)?.as_bytes()),
        };
        let result = Self::from_record(RecordedBox::new(
            serde_json::to_value(material).map_err(error)?,
            Origin::Hypothetical,
            None,
        )?)?;
        if result.node != node {
            return Err("node fields disagree with serialized box".into());
        }
        Ok(result)
    }
    pub fn recorded(
        material: BoxMaterial,
        locator: String,
        revision: Option<String>,
    ) -> Result<Self, String> {
        Self::from_record(crate::map::source::record_box(
            serde_json::to_value(material).map_err(error)?,
            locator,
            revision,
        )?)
    }
    pub fn from_record(record: RecordedBox) -> Result<Self, String> {
        // Validate imported provenance too; RecordedBox alone is a raw DTO.
        let mut premises = CasePremises::unspecified();
        premises.boxes = Premise::supplied(vec![record.clone()]);
        EvidenceCase::new(premises)?;
        let material: BoxMaterial =
            serde_json::from_value(record.document().clone()).map_err(error)?;
        let bytes = decode(&material.bytes)?;
        let tree = decode(&material.ergo_tree)?;
        let node = parse_ergo_box_bytes(&bytes, &tree).map_err(error)?;
        if serialize_ergo_box(&node).map_err(error)? != bytes {
            return Err("noncanonical box encoding; input was not rewritten".into());
        }
        if node.box_id().map_err(error)?.as_bytes() != &id_bytes(&material.box_id)? {
            return Err("claimed box ID disagrees with full serialization".into());
        }
        Ok(Self { node, record })
    }
    pub fn node(&self) -> &ErgoBox {
        &self.node
    }
    pub fn record(&self) -> &RecordedBox {
        &self.record
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        serialize_ergo_box(&self.node).map_err(error)
    }
    pub fn id(&self) -> Result<String, String> {
        Ok(hex::encode(self.node.box_id().map_err(error)?.as_bytes()))
    }
}

/// Immutable node transaction with checked byte/field consistency. Proofs are
/// retained as supplied, but neither checked nor created by this module.
#[derive(Debug, Clone)]
pub struct WireTransaction {
    node: Transaction,
    bytes: Vec<u8>,
    message: Vec<u8>,
    id: String,
}
impl WireTransaction {
    pub fn build(
        inputs: Vec<Input>,
        data_inputs: Vec<DataInput>,
        outputs: &[CandidateSpec],
    ) -> Result<Self, String> {
        Self::from_node(Transaction {
            inputs,
            data_inputs,
            output_candidates: outputs
                .iter()
                .map(CandidateSpec::build)
                .collect::<Result<_, _>>()?,
        })
    }
    pub fn from_node(node: Transaction) -> Result<Self, String> {
        let mut writer = VlqWriter::new();
        write_transaction(&mut writer, &node).map_err(error)?;
        let bytes = writer.result();
        let parsed = parse_transaction(&bytes)?;
        if parsed != node {
            return Err("node fields disagree with serialized transaction".into());
        }
        Self::checked(parsed, bytes)
    }
    pub fn from_bytes(bytes: &[u8], claimed_id: &str) -> Result<Self, String> {
        let result = Self::checked(parse_transaction(bytes)?, bytes.to_vec())?;
        if id_bytes(claimed_id)? != id_bytes(&result.id)? {
            return Err("claimed transaction ID disagrees with node bytes_to_sign".into());
        }
        Ok(result)
    }
    fn checked(node: Transaction, bytes: Vec<u8>) -> Result<Self, String> {
        let mut writer = VlqWriter::new();
        write_transaction(&mut writer, &node).map_err(error)?;
        if writer.result() != bytes {
            return Err("noncanonical transaction encoding; input was not rewritten".into());
        }
        let message = bytes_to_sign(&node).map_err(error)?;
        let id = hex::encode(transaction_id(&node).map_err(error)?.as_bytes());
        Ok(Self {
            node,
            bytes,
            message,
            id,
        })
    }
    pub fn node(&self) -> &Transaction {
        &self.node
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn bytes_to_sign(&self) -> &[u8] {
        &self.message
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn output_boxes(&self) -> Result<Vec<WireBox>, String> {
        self.node
            .output_candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                WireBox::hypothetical_node(
                    candidate.clone(),
                    &CreationReference {
                        transaction_id: self.id.clone(),
                        index: u16::try_from(index).map_err(error)?,
                    },
                )
            })
            .collect()
    }
    /// Attach exact transaction/proof/extension bytes and resolved box records to
    /// a P01 case. Only reference association is checked, not spend validity.
    /// Existing premises are preserved; occupied state/wire slots are rejected.
    pub fn bind_case(
        &self,
        case: &EvidenceCase,
        inputs: &[WireBox],
        data_inputs: &[WireBox],
    ) -> Result<EvidenceCase, String> {
        if case.premises().engine_revision != super::case::engine_revision() {
            return Err("case engine revision differs".into());
        }
        if inputs.len() != self.node.inputs.len()
            || data_inputs.len() != self.node.data_inputs.len()
        {
            return Err("resolved box count differs from transaction references".into());
        }
        for (b, input) in inputs.iter().zip(&self.node.inputs) {
            if b.node.box_id().map_err(error)? != input.box_id {
                return Err("input box reference mismatch".into());
            }
        }
        for (b, input) in data_inputs.iter().zip(&self.node.data_inputs) {
            if b.node.box_id().map_err(error)? != input.box_id {
                return Err("data input box reference mismatch".into());
            }
        }
        let mut p = case.premises().clone();
        if p.boxes.value().is_some() || p.assumptions.contains_key("wireTransaction") {
            return Err(
                "case already contains state/wire material; construct a new case explicitly".into(),
            );
        }
        let previous_state = serde_json::to_value(&p.boxes).map_err(error)?;
        p.boxes = Premise::supplied(
            inputs
                .iter()
                .chain(data_inputs)
                .map(|b| b.record.clone())
                .collect(),
        );
        p.assumptions.insert("wireTransaction".into(), Premise::supplied(json!({
            "formatVersion":1,"method":"canonical-wire-construction","nodeValidated":false,
            "bytes":hex::encode(&self.bytes),"bytesToSign":hex::encode(&self.message),"transactionId":self.id,
            "inputCount":inputs.len(),"dataInputCount":data_inputs.len(),"previousStatePremise":previous_state,
        })));
        EvidenceCase::new(p)
    }
}
fn parse_transaction(bytes: &[u8]) -> Result<Transaction, String> {
    let mut reader = VlqReader::new(bytes);
    let node = read_transaction(&mut reader).map_err(error)?;
    if !reader.is_empty() {
        return Err("trailing bytes after transaction".into());
    }
    Ok(node)
}
fn decode(s: &str) -> Result<Vec<u8>, String> {
    if s.is_empty() {
        return Err("wire bytes are required; no empty fallback".into());
    }
    hex::decode(s).map_err(error)
}
fn id_bytes(s: &str) -> Result<[u8; 32], String> {
    decode(s)?
        .try_into()
        .map_err(|_| "ID must be exactly 32 bytes".into())
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
