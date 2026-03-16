// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::sol;
use alloy::sol_types::SolInterface;

sol!(
    #[derive(Debug)]
    Enclave,
    "../../packages/enclave-contracts/artifacts/contracts/Enclave.sol/Enclave.json"
);

sol!(
    #[derive(Debug)]
    #[sol(ignore_unlinked)]
    CiphernodeRegistryOwnable,
    "../../packages/enclave-contracts/artifacts/contracts/registry/CiphernodeRegistryOwnable.sol/CiphernodeRegistryOwnable.json"
);

sol!(
    #[derive(Debug)]
    SlashingManager,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/ISlashingManager.sol/ISlashingManager.json"
);

/// Try to decode raw revert data into a human-readable error string.
pub fn decode_error(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }

    if let Ok(err) = Enclave::EnclaveErrors::abi_decode(data) {
        return Some(format!("{err:?}"));
    }
    if let Ok(err) = CiphernodeRegistryOwnable::CiphernodeRegistryOwnableErrors::abi_decode(data) {
        return Some(format!("{err:?}"));
    }
    if let Ok(err) = SlashingManager::SlashingManagerErrors::abi_decode(data) {
        return Some(format!("{err:?}"));
    }

    None
}

/// Extract hex revert data from an error string and try to decode it.
pub fn decode_error_from_str(error_str: &str) -> Option<String> {
    let data = extract_revert_data(error_str)?;
    decode_error(&data)
}

/// Format an anyhow error, replacing raw hex revert data with decoded error if possible.
/// Returns the decoded error string if decoding succeeds, otherwise the original error.
pub fn format_evm_error(err: &anyhow::Error) -> String {
    let error_str = format!("{err:?}");
    decode_error_from_str(&error_str).unwrap_or(error_str)
}

/// Find the longest hex string (0x...) with at least 4 bytes (8 hex chars) in an error message.
fn extract_revert_data(error_str: &str) -> Option<Vec<u8>> {
    error_str
        .match_indices("0x")
        .filter_map(|(idx, _)| {
            let rest = &error_str[idx + 2..];
            let hex_end = rest
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(rest.len());
            let hex_str = &rest[..hex_end];
            if hex_str.len() >= 8 {
                hex::decode(hex_str).ok()
            } else {
                None
            }
        })
        .max_by_key(|data| data.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::sol_types::SolError;

    #[test]
    fn test_decode_known_errors() {
        // CiphertextOutputNotPublished(uint256 e3Id) with e3Id = 1
        let mut data = Enclave::CiphertextOutputNotPublished::SELECTOR.to_vec();
        data.extend_from_slice(&[0u8; 31]);
        data.push(1); // e3Id = 1
        let decoded = decode_error(&data).unwrap();
        assert!(
            decoded.contains("CiphertextOutputNotPublished"),
            "got: {decoded}"
        );
    }

    #[test]
    fn test_decode_parameterless_error() {
        // CommitteeNotRequested()
        let data = CiphernodeRegistryOwnable::CommitteeNotRequested::SELECTOR.to_vec();
        let decoded = decode_error(&data).unwrap();
        assert!(decoded.contains("CommitteeNotRequested"), "got: {decoded}");
    }

    #[test]
    fn test_decode_from_error_string() {
        // Simulate an alloy error string containing hex revert data
        let selector = hex::encode(Enclave::CiphertextOutputNotPublished::SELECTOR);
        let param = "0000000000000000000000000000000000000000000000000000000000000001";
        let error_str = format!(
            "server returned an error response: error code 3: execution reverted, data: \"0x{selector}{param}\""
        );
        let decoded = decode_error_from_str(&error_str).unwrap();
        assert!(
            decoded.contains("CiphertextOutputNotPublished"),
            "got: {decoded}"
        );
    }

    #[test]
    fn test_decode_unknown_error() {
        let data = vec![0xde, 0xad, 0xbe, 0xef];
        assert!(decode_error(&data).is_none());
    }

    #[test]
    fn test_extract_revert_data_too_short() {
        assert!(extract_revert_data("0x1234").is_none());
    }
}
