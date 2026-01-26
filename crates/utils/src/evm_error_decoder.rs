// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! EVM Error Decoder
//!
//! This module provides functionality to decode EVM error selectors into
//! human-readable error names. The error mappings are generated from contract
//! ABIs using `scripts/generate_error_selectors.sh`.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::evm_error_selectors::ERROR_SELECTORS;

/// Decoded EVM error information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedEvmError {
    /// The human-readable error name (e.g., "E3Expired")
    pub name: String,
    /// The 4-byte selector as hex (e.g., "0x7b054f9a")
    pub selector: String,
    /// Parameter types if available (e.g., ["uint256", "address"])
    pub param_types: Vec<String>,
}

impl std::fmt::Display for DecodedEvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.param_types.is_empty() {
            write!(f, "{} [{}]", self.name, self.selector)
        } else {
            write!(f, "{}({}) [{}]", self.name, self.param_types.join(", "), self.selector)
        }
    }
}

/// Static lookup map built from the auto-generated ERROR_SELECTORS
static SELECTOR_MAP: LazyLock<HashMap<[u8; 4], (&'static str, &'static [(&'static str, &'static str)])>> =
    LazyLock::new(|| {
        ERROR_SELECTORS
            .iter()
            .map(|(selector, name, params)| (*selector, (*name, *params)))
            .collect()
    });                                                                                                                                                               
                                                                                                                                                                   
 /// Decode a 4-byte selector into error information
pub fn decode_selector(selector: &[u8; 4]) -> Option<DecodedEvmError> {
    SELECTOR_MAP.get(selector).map(|(name, params)| DecodedEvmError {
        name: (*name).to_string(),
        selector: format!("0x{}", hex::encode(selector)),
        param_types: params.iter().map(|(_, t)| t.to_string()).collect(),
    })
}

/// Decode EVM error data (selector + encoded params) into error information
///
/// Note: This decodes the error name and parameter types.
/// For now, parameter values are not decoded (would require full ABI decoding).
pub fn decode_evm_error(error_data: &[u8]) -> Option<DecodedEvmError> {
    if error_data.len() < 4 {
        return None;
    }

    let selector: [u8; 4] = error_data[0..4].try_into().ok()?;
    decode_selector(&selector)
}                                                                                                                                                                 
                                                                                                                                                                   
 /// Extract and decode an error selector from an error string                                                                                                     
 ///                                                                                                                                                               
 /// Searches for hex patterns like "0x7b054f9a" in the error message and                                                                                          
 /// attempts to decode them.                                                                                                                                      
 ///                                                                                                                                                               
 /// # Arguments                                                                                                                                                   
 /// * `error_str` - Error string that may contain a hex selector                                                                                                  
 ///                                                                                                                                                               
 /// # Returns                                                                                                                                                     
 /// * `Some(DecodedEvmError)` if a selector was found and decoded                                                                                                 
 /// * `None` if no valid selector was found                                                                                                                       
 pub fn extract_and_decode_from_string(error_str: &str) -> Option<DecodedEvmError> {                                                                               
     // Look for hex selector pattern (0x followed by 8 hex chars)                                                                                                 
     // The error string typically contains patterns like "0x7b054f9a" or "execution reverted: 0x..."                                                              
     let mut i = 0;                                                                                                                                                
     let bytes = error_str.as_bytes();                                                                                                                             
                                                                                                                                                                   
     while i + 10 <= bytes.len() {                                                                                                                                 
         if bytes[i] == b'0' && bytes[i + 1] == b'x' {                                                                                                             
             // Found "0x", try to parse the next 8 characters as hex                                                                                              
             let hex_slice = &error_str[i + 2..];                                                                                                                  
             if hex_slice.len() >= 8 {                                                                                                                             
                 let hex_chars = &hex_slice[..8];                                                                                                                  
                 if hex_chars.chars().all(|c| c.is_ascii_hexdigit()) {                                                                                             
                     if let Ok(selector_bytes) = hex::decode(hex_chars) {                                                                                          
                         if selector_bytes.len() == 4 {                                                                                                            
                             let selector: [u8; 4] = selector_bytes.try_into().unwrap();                                                                           
                             if let Some(decoded) = decode_selector(&selector) {                                                                                   
                                 return Some(decoded);                                                                                                             
                             }                                                                                                                                     
                         }                                                                                                                                         
                     }                                                                                                                                             
                 }                                                                                                                                                 
             }                                                                                                                                                     
         }                                                                                                                                                         
         i += 1;                                                                                                                                                   
     }                                                                                                                                                             
                                                                                                                                                                   
     None                                                                                                                                                          
 }                                                                                                                                                                 
                                                                                                                                                                   
 /// Get the error name for a given selector hex string (e.g., "0x7b054f9a")
pub fn get_error_name(selector_hex: &str) -> Option<&'static str> {
    let hex_str = selector_hex.strip_prefix("0x").unwrap_or(selector_hex);
    if hex_str.len() != 8 {
        return None;
    }

    let bytes = hex::decode(hex_str).ok()?;
    let selector: [u8; 4] = bytes.try_into().ok()?;
    SELECTOR_MAP.get(&selector).map(|(name, _)| *name)
}                                                                                                                                                                 
                                                                                                                                                                   
 #[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_selectors_loaded() {
        // Verify the auto-generated data is loaded
        assert!(!ERROR_SELECTORS.is_empty(), "ERROR_SELECTORS should not be empty");
    }

    #[test]
    fn test_selector_map_built() {
        // Verify the map is built correctly
        assert!(!SELECTOR_MAP.is_empty(), "SELECTOR_MAP should not be empty");
    }

    #[test]
    fn test_extract_from_error_string() {
        // Find a known selector from the generated data and test extraction
        if let Some((selector, name, _)) = ERROR_SELECTORS.first() {
            let selector_hex = format!("0x{}", hex::encode(selector));
            let error_str = format!("execution reverted: {}", selector_hex);

            let decoded = extract_and_decode_from_string(&error_str);
            assert!(decoded.is_some(), "Should decode error from string");
            assert_eq!(decoded.unwrap().name, *name);
        }
    }

    #[test]
    fn test_get_error_name() {
        // Test with a known selector from generated data
        if let Some((selector, name, _)) = ERROR_SELECTORS.first() {
            let selector_hex = format!("0x{}", hex::encode(selector));
            assert_eq!(get_error_name(&selector_hex), Some(*name));
        }

        // Unknown selector should return None
        assert_eq!(get_error_name("0xdeadbeef"), None);
    }

    #[test]
    fn test_unknown_selector() {
        let unknown = [0xde, 0xad, 0xbe, 0xef];
        assert!(decode_selector(&unknown).is_none());
    }

    #[test]
    fn test_decode_evm_error() {
        // Find a known selector and test decoding
        if let Some((selector, name, params)) = ERROR_SELECTORS.first() {
            let decoded = decode_evm_error(selector);
            assert!(decoded.is_some());
            let decoded = decoded.unwrap();
            assert_eq!(decoded.name, *name);
            assert_eq!(decoded.param_types.len(), params.len());
        }
    }

    #[test]
    fn test_display_impl() {
        let error_no_params = DecodedEvmError {
            name: "E3Expired".to_string(),
            selector: "0x7b054f9a".to_string(),
            param_types: vec![],
        };
        assert_eq!(format!("{}", error_no_params), "E3Expired [0x7b054f9a]");

        let error_with_params = DecodedEvmError {
            name: "E3DoesNotExist".to_string(),
            selector: "0xabcd1234".to_string(),
            param_types: vec!["uint256".to_string()],
        };
        assert_eq!(format!("{}", error_with_params), "E3DoesNotExist(uint256) [0xabcd1234]");
    }
} 
