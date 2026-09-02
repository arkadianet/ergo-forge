//! Turn user input — an address or ErgoTree hex — into tree bytes.
//!
//! Hex is tried first: addresses are base58 and trees are hex, so the two do
//! not collide in practice.

use crate::error::ApiError;
use ergo_ser::address::NetworkPrefix;

/// Longest accepted input. Real ErgoTrees are far smaller; this bounds the
/// work done before validation.
pub const MAX_INPUT_CHARS: usize = 64 * 1024;

pub fn resolve(input: &str, network: NetworkPrefix) -> Result<Vec<u8>, ApiError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(ApiError::InvalidInput("input is empty".into()));
    }
    if s.len() > MAX_INPUT_CHARS {
        return Err(ApiError::TooLarge);
    }
    if let Ok(bytes) = hex::decode(s) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    ergo_ser::address::decode_address_to_tree_bytes(s, network)
        .map_err(|e| ApiError::InvalidInput(format!("not valid ErgoTree hex or an address: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hex_resolves() {
        let bytes = resolve("1001040ad191e4c6a704047300", NetworkPrefix::Mainnet).unwrap();
        assert_eq!(hex::encode(&bytes), "1001040ad191e4c6a704047300");
    }

    #[test]
    fn a_known_mainnet_address_resolves_to_the_same_tree() {
        let addr = ergo_ser::address::encode_p2s(NetworkPrefix::Mainnet, &[0x10, 0x01, 0x04]);
        let bytes = resolve(&addr, NetworkPrefix::Mainnet).unwrap();
        assert_eq!(bytes, vec![0x10, 0x01, 0x04]);
    }

    #[test]
    fn garbage_is_invalid_input() {
        assert!(matches!(
            resolve("not a contract", NetworkPrefix::Mainnet),
            Err(ApiError::InvalidInput(_))
        ));
    }

    #[test]
    fn empty_is_invalid_input() {
        assert!(matches!(
            resolve("   ", NetworkPrefix::Mainnet),
            Err(ApiError::InvalidInput(_))
        ));
    }

    #[test]
    fn oversized_is_too_large() {
        let big = "a".repeat(MAX_INPUT_CHARS + 1);
        assert!(matches!(
            resolve(&big, NetworkPrefix::Mainnet),
            Err(ApiError::TooLarge)
        ));
    }
}
