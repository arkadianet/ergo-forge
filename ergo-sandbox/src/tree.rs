//! Reproducible address, tree and box ingestion. No implicit chain access.
use ergo_primitives::{digest::blake2b256, reader::VlqReader};
use ergo_ser::address::{self, NetworkPrefix};
use serde::Serialize;

use crate::map::source::{ChainBox, ChainSource};

// Standard Pay2SH script: sigmaProp(hash192(getVar[Coll[Byte]](1)) == hash)
// && deserializeContext[SigmaProp](1). See sigma-rust Address::script.
const SH_PREFIX: &[u8] = &[
    0x00, 0xea, 0x02, 0xd1, 0x93, 0xb4, 0xcb, 0xe4, 0xe3, 0x01, 0x0e, 0x04, 0x00, 0x04, 0x30, 0x0e,
    0x18,
];
const SH_SUFFIX: &[u8] = &[0xd4, 0x08, 0x01];

/// Address forms which decode to the exact reported bytes. Unavailable forms are null.
#[derive(Debug, Serialize)]
pub struct Addresses {
    pub p2s: String,
    pub p2pk: Option<String>,
    pub p2sh: Option<String>,
}

/// Stable ingestion output, version 1. Box registers retain serialized constant hex.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeReport {
    pub format_version: u32,
    pub tree_hex: String,
    pub network: String,
    pub addresses: Addresses,
    pub box_data: Option<ChainBox>,
    pub source: String,
    pub audit: serde_json::Value,
    /// P2SH only exposes the wrapper, never the script preimage.
    pub notes: Vec<String>,
}

/// Decode an address offline; optionally enforce the caller's expected network.
/// Checks the checksum, network/type byte, payload length and public key encoding.
pub fn decode_address(
    input: &str,
    expected: Option<NetworkPrefix>,
) -> Result<(Vec<u8>, NetworkPrefix), String> {
    let raw = bs58::decode(input)
        .into_vec()
        .map_err(|e| format!("invalid base58 address: {e}"))?;
    let header = *raw.first().ok_or("empty address")?;
    let network = match header & 0xf0 {
        0x00 => NetworkPrefix::Mainnet,
        0x10 => NetworkPrefix::Testnet,
        _ => return Err(format!("invalid network prefix byte 0x{header:02x}")),
    };
    let content = address::decode_address_content_bytes(input, expected.unwrap_or(network))
        .map_err(|e| e.to_string())?;
    let bytes = match header & 0x0f {
        1 => {
            k256::PublicKey::from_sec1_bytes(&content)
                .map_err(|e| format!("invalid P2PK public key: {e}"))?;
            address::decode_address_to_tree_bytes(input, network).map_err(|e| e.to_string())?
        }
        2 => {
            if content.len() != 24 {
                return Err(format!(
                    "invalid P2SH hash length: expected 24, got {}",
                    content.len()
                ));
            }
            [SH_PREFIX, &content, SH_SUFFIX].concat()
        }
        3 => content,
        _ => return Err(format!("unsupported address type in prefix 0x{header:02x}")),
    };
    Ok((bytes, network))
}

/// Exact 32-byte hex inputs are box IDs. Use `tree:` to disambiguate a 32-byte
/// tree; `box:` and `address:` are also accepted. Only box IDs query `source`.
pub fn ingest_tree(
    input: &str,
    network: Option<NetworkPrefix>,
    source: Option<&dyn ChainSource>,
) -> Result<TreeReport, String> {
    let input = input.trim();
    let (kind, value) = input
        .split_once(':')
        .filter(|(k, _)| matches!(*k, "tree" | "box" | "address"))
        .unwrap_or(("auto", input));
    let is_hex = !value.is_empty() && value.bytes().all(|b| b.is_ascii_hexdigit());
    let is_box = kind == "box" || (kind == "auto" && is_hex && value.len() == 64);
    let mut box_data = None;
    let (bytes, network) = if is_box {
        if value.len() != 64 || !is_hex {
            return Err("box id must be exactly 32 bytes of hex".into());
        }
        let id = value.to_ascii_lowercase();
        let mut b = source
            .ok_or("box lookup requires --source, --explorer or EXPLORER_URL")?
            .box_by_id(&id)
            .map_err(|e| e.to_string())?;
        if !b.box_id.eq_ignore_ascii_case(&id) {
            return Err("chain source returned a different box id".into());
        }
        b.box_id = id;
        let bytes = hex::decode(&b.ergo_tree).map_err(|e| format!("invalid box tree hex: {e}"))?;
        b.ergo_tree = hex::encode(&bytes);
        box_data = Some(b);
        (bytes, network.unwrap_or(NetworkPrefix::Mainnet))
    } else if kind == "tree" || (kind == "auto" && is_hex) {
        (
            hex::decode(value).map_err(|e| format!("invalid tree hex: {e}"))?,
            network.unwrap_or(NetworkPrefix::Mainnet),
        )
    } else {
        decode_address(value, network)?
    };
    crate::decompile::with_large_stack(move || report(bytes, network, box_data))
}

fn report(
    bytes: Vec<u8>,
    network: NetworkPrefix,
    box_data: Option<ChainBox>,
) -> Result<TreeReport, String> {
    let mut reader = VlqReader::new(&bytes);
    let tree = ergo_ser::ergo_tree::read_ergo_tree(&mut reader)
        .map_err(|e| format!("invalid ErgoTree: {e}"))?;
    if reader.remaining() != 0 {
        return Err("trailing bytes after ErgoTree".into());
    }
    let p2s = address::encode_p2s(network, &bytes);
    let candidate = address::encode_address(network, &tree, &bytes);
    let p2pk = (candidate != p2s
        && address::decode_address_to_tree_bytes(&candidate, network)
            .ok()
            .as_ref()
            == Some(&bytes))
    .then_some(candidate);
    let mut notes = Vec::new();
    let p2sh = if bytes.len() == SH_PREFIX.len() + 24 + SH_SUFFIX.len()
        && bytes.starts_with(SH_PREFIX)
        && bytes.ends_with(SH_SUFFIX)
    {
        let mut payload = vec![network as u8 | 2];
        payload.extend_from_slice(&bytes[SH_PREFIX.len()..SH_PREFIX.len() + 24]);
        let checksum = blake2b256(&payload);
        payload.extend_from_slice(&checksum.as_bytes()[..4]);
        notes.push("P2SH hash-check wrapper only; the underlying script preimage is unavailable and is not audited.".into());
        Some(bs58::encode(payload).into_string())
    } else {
        None
    };
    let lifted = crate::lift_tree(&tree, network == NetworkPrefix::Testnet);
    let audit = crate::audit::audit(&lifted);
    let findings: Vec<_> = audit
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "lint": f.lint, "severity": f.severity.label(), "nodeId": f.node_id,
                "irId": f.ir_id, "message": f.message, "snippet": f.snippet
            })
        })
        .collect();
    Ok(TreeReport {
        format_version: 1,
        tree_hex: hex::encode(bytes),
        network: if network == NetworkPrefix::Testnet {
            "testnet"
        } else {
            "mainnet"
        }
        .into(),
        addresses: Addresses { p2s, p2pk, p2sh },
        box_data,
        source: crate::decompile::print(&lifted.node),
        audit: serde_json::json!({"complete": matches!(audit.completeness, crate::audit::Completeness::Complete), "rawPlaceholders": lifted.raw_placeholders, "truncated": lifted.truncated, "findings": findings}),
        notes,
    })
}
