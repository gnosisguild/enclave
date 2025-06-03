use alloy::dyn_abi::{DynSolType, DynSolValue};
use alloy::primitives::U256;
use fhe_rs::bfv::{BfvParameters, BfvParametersBuilder};
use fhe_traits::{Deserialize, Serialize};
use std::sync::Arc;
/// Predefined BFV parameters for common use cases
pub mod params {
    /// Standard BFV parameters sets
    /// Each set is a tuple of (degree, plaintext_modulus, moduli).
    /// Naming convention: SET_<degree>_<plaintext_modulus>_<moduli_count>

    /// - Degree: 2048 (polynomial ring size)
    /// - Plaintext modulus: 1032193
    /// - Moduli: [0x3FFFFFFF000001] (provides good security level)
    pub const SET_2048_1032193_1: (usize, u64, [u64; 1]) = (
        2048,               // degree
        1032193,            // plaintext_modulus
        [0x3FFFFFFF000001], // moduli
    );
}

/// Builds BFV (Brakerski-Fan-Vercauteren) encryption parameters.
///
/// # Arguments
///
/// * `degree` - The degree of the polynomial ring, must be a power of 2
/// * `plaintext_modulus` - The modulus for the plaintext space
/// * `moduli` - The moduli for the ciphertext space
///
/// # Returns
///
/// Returns a `BfvParameters` instance configured with the specified parameters.
///
/// # Panics
///
/// Panics if the parameters cannot be built (e.g., invalid degree or moduli).
pub fn build_bfv_params(degree: usize, plaintext_modulus: u64, moduli: &[u64]) -> BfvParameters {
    match BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build()
    {
        Ok(params) => params,
        Err(e) => panic!("Failed to build BFV Parameters: {}", e),
    }
}

/// Builds BFV encryption parameters wrapped in an `Arc` for shared ownership.
///
/// This function is similar to `build_bfv_params` but returns the parameters
/// wrapped in an `Arc` for thread-safe shared ownership.
///
/// # Arguments
///
/// * `degree` - The degree of the polynomial ring, must be a power of 2
/// * `plaintext_modulus` - The modulus for the plaintext space
/// * `moduli` - The moduli for the ciphertext space
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` instance configured with the specified parameters.
///
/// # Panics
///
/// Panics if the parameters cannot be built (e.g., invalid degree or moduli).
pub fn build_bfv_params_arc(
    degree: usize,
    plaintext_modulus: u64,
    moduli: &[u64],
) -> Arc<BfvParameters> {
    match BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build_arc()
    {
        Ok(params) => params,
        Err(e) => panic!("Failed to build BFV Parameters wrapped in Arc: {}", e),
    }
}

/// Serializes BFV parameters into ABI-encoded bytes.
///
/// This function converts BFV parameters into a tuple structure of (degree, plaintext_modulus, moduli[])
/// and then ABI-encodes the tuple using Solidity ABI format. The resulting bytes can be used
/// in smart contracts or for cross-platform serialization.
///
/// # Arguments
///
/// * `params` - The BFV parameters to serialize
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the ABI-encoded parameters as a tuple (uint256, uint256, uint256[]).
pub fn serialize_bfv_params(params: &BfvParameters) -> Vec<u8> {
    let value = DynSolValue::Tuple(vec![
        DynSolValue::Uint(U256::from(params.degree()), 256),
        DynSolValue::Uint(U256::from(params.plaintext()), 256),
        DynSolValue::Array(
            params
                .moduli()
                .iter()
                .map(|val| DynSolValue::Uint(U256::from(*val), 256))
                .collect(),
        ),
    ]);
    value.abi_encode_params()
}

/// Deserializes BFV parameters from ABI-encoded bytes.
///
/// This function converts ABI-encoded bytes back into BFV parameters.
/// The bytes should represent a tuple (uint256, uint256, uint256[]) containing
/// (degree, plaintext_modulus, moduli[]) as produced by `serialize_bfv_params`.
///
/// # Arguments
///
/// * `bytes` - The ABI-encoded bytes containing the serialized parameters
///
/// # Returns
///
/// Returns a `BfvParameters` instance deserialized from the bytes.
///
/// # Panics
///
/// Panics if the deserialization fails due to invalid format or parameter values.
pub fn deserialize_bfv_params(bytes: &[u8]) -> BfvParameters {
    // Define the expected tuple type: (uint256, uint256, uint256[])
    let tuple_type = DynSolType::Tuple(vec![
        DynSolType::Uint(256),                              // degree
        DynSolType::Uint(256),                              // plaintext_modulus
        DynSolType::Array(Box::new(DynSolType::Uint(256))), // moduli array
    ]);

    let decoded = tuple_type
        .abi_decode(bytes)
        .expect("Failed to ABI decode bytes");

    match decoded {
        DynSolValue::Tuple(inner_values) => {
            // Extract degree (first element)
            let degree: u64 = match &inner_values[0] {
                DynSolValue::Uint(val, _) => {
                    (*val).try_into().expect("Failed to convert degree to u64")
                }
                _ => panic!("Expected uint256 for degree"),
            };

            // Extract plaintext modulus (second element)
            let plaintext: u64 = match &inner_values[1] {
                DynSolValue::Uint(val, _) => (*val)
                    .try_into()
                    .expect("Failed to convert plaintext to u64"),
                _ => panic!("Expected uint256 for plaintext modulus"),
            };

            // Extract moduli array (third element)
            let moduli: Vec<u64> = match &inner_values[2] {
                DynSolValue::Array(moduli_array) => moduli_array
                    .iter()
                    .map(|val| match val {
                        DynSolValue::Uint(modulus, _) => (*modulus)
                            .try_into()
                            .expect("Failed to convert modulus to u64"),
                        _ => panic!("Expected uint256 for modulus value"),
                    })
                    .collect::<Vec<_>>(),
                _ => panic!("Expected array for moduli"),
            };

            let params = BfvParametersBuilder::new()
                .set_degree(degree as usize)
                .set_plaintext_modulus(plaintext)
                .set_moduli(&moduli)
                .build()
                .expect("Failed to build BFV Parameters");

            params
        }
        _ => panic!("Expected tuple value in ABI encoding"),
    }
}

/// Deserializes BFV parameters from ABI-encoded bytes and wraps them in an `Arc`.
///
/// This is a convenience function that combines `deserialize_bfv_params` with `Arc::new`
/// to provide thread-safe shared ownership of the deserialized parameters.
/// The input bytes should represent a tuple (uint256, uint256, uint256[]) containing
/// (degree, plaintext_modulus, moduli[]) in ABI-encoded format.
///
/// # Arguments
///
/// * `bytes` - The ABI-encoded bytes containing the serialized parameters
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` instance deserialized from the bytes.
///
/// # Panics
///
/// Panics if the deserialization fails (see `deserialize_bfv_params`).
pub fn deserialize_bfv_params_arc(bytes: &[u8]) -> Arc<BfvParameters> {
    Arc::new(deserialize_bfv_params(bytes))
}

/// ABI-encodes BFV parameters as bytes using the Solidity ABI format.
///
/// This function takes BFV parameters, converts them to a tuple structure,
/// and then wraps the result in a DynSolValue::Bytes before ABI-encoding.
/// This creates a double-encoded structure: the outer layer is bytes,
/// and the inner layer is the tuple (uint256, uint256, uint256[]).
///
/// # Arguments
///
/// * `params` - The BFV parameters to encode
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the ABI-encoded parameters wrapped as bytes.
pub fn encode_bfv_params(params: &BfvParameters) -> Vec<u8> {
    DynSolValue::Bytes(serialize_bfv_params(params)).abi_encode_params()
}

/// ABI-decodes BFV parameters from double-encoded Solidity ABI format.
///
/// This function takes ABI-encoded bytes where the outer layer is bytes type,
/// and the inner layer contains the serialized BFV parameters. It first decodes
/// the outer bytes layer, then uses the native BFV deserialization on the inner bytes.
///
/// # Arguments
///
/// * `bytes` - The double ABI-encoded bytes containing the parameters
///
/// # Returns
///
/// Returns a `BfvParameters` instance deserialized from the bytes.
///
/// # Panics
///
/// Panics if the decoding/deserialization fails.
pub fn decode_bfv_params(bytes: &[u8]) -> BfvParameters {
    let bytes_type = DynSolType::Bytes;
    let decoded = bytes_type
        .abi_decode(bytes)
        .expect("Failed to ABI decode bytes");

    match decoded {
        DynSolValue::Bytes(inner_bytes) => {
            BfvParameters::try_deserialize(&inner_bytes).expect("Could not decode Bfv Params")
        }
        _ => panic!("Expected bytes value in ABI encoding"),
    }
}

/// ABI-decodes BFV parameters from double-encoded Solidity ABI format and wraps them in an `Arc`.
///
/// This function is similar to `decode_bfv_params` but returns the parameters
/// wrapped in an `Arc` for thread-safe shared ownership.
///
/// # Arguments
///
/// * `bytes` - The double ABI-encoded bytes containing the parameters
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` containing the deserialized parameters.
///
/// # Panics
///
/// Panics if the decoding/deserialization fails.
pub fn decode_bfv_params_arc(bytes: &[u8]) -> Arc<BfvParameters> {
    Arc::new(decode_bfv_params(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_bfv_params() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
    }

    #[test]
    fn test_build_bfv_params_arc() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
    }

    #[test]
    fn test_raw_serialization_roundtrip() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let serialized = serialize_bfv_params(&params);
        let deserialized = deserialize_bfv_params(&serialized);

        assert_eq!(deserialized.degree(), degree);
        assert_eq!(deserialized.plaintext(), plaintext_modulus);
        assert_eq!(deserialized.moduli(), moduli.as_slice());
    }

    #[test]
    fn test_abi_encoding_roundtrip() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);

        // First serialize to raw bytes
        let serialized = serialize_bfv_params(&params);

        // Then ABI encode the raw bytes
        let encoded = encode_bfv_params(&params);

        // Verify the encoded result is deterministic
        let encoded_again = encode_bfv_params(&params);
        assert_eq!(
            encoded, encoded_again,
            "ABI encoding should be deterministic"
        );

        // Verify the ABI-encoded result is different from the raw serialized bytes
        assert_ne!(
            encoded, serialized,
            "ABI-encoded result should be different from raw serialized bytes"
        );

        // Verify we can ABI-decode and deserialize back to the original parameters
        let decoded = decode_bfv_params(&encoded);
        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli.as_slice());
    }

    #[test]
    fn test_abi_encoding_roundtrip_arc() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let encoded = encode_bfv_params(&params);

        // Verify we can ABI-decode and deserialize back to the original parameters with Arc
        let decoded = decode_bfv_params_arc(&encoded);
        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli.as_slice());
    }

    #[test]
    #[should_panic(expected = "Failed to ABI decode bytes")]
    fn test_decode_bfv_params_error() {
        let invalid_bytes = vec![0u8; 10];
        let _ = decode_bfv_params(&invalid_bytes);
    }

    mod params_tests {
        use super::*;

        #[test]
        fn test_params_constant() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(degree, 2048);
            assert_eq!(plaintext_modulus, 1032193);
            assert_eq!(moduli, [0x3FFFFFFF000001]);
        }

        #[test]
        fn test_params_function() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params(degree, plaintext_modulus, &moduli);

            assert_eq!(params.degree(), degree);
            assert_eq!(params.plaintext(), plaintext_modulus);
            assert_eq!(params.moduli(), moduli);
        }

        #[test]
        fn test_params_arc_function() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

            assert_eq!(params.degree(), degree);
            assert_eq!(params.plaintext(), plaintext_modulus);
            assert_eq!(params.moduli(), moduli);
        }

        #[test]
        fn test_params_serialization_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params(degree, plaintext_modulus, &moduli);
            let serialized = serialize_bfv_params(&params);
            let deserialized = deserialize_bfv_params(&serialized);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(deserialized.degree(), degree);
            assert_eq!(deserialized.plaintext(), plaintext_modulus);
            assert_eq!(deserialized.moduli(), moduli);
        }

        #[test]
        fn test_params_arc_serialization_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
            let serialized = serialize_bfv_params(&params);
            let deserialized = deserialize_bfv_params_arc(&serialized);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(deserialized.degree(), degree);
            assert_eq!(deserialized.plaintext(), plaintext_modulus);
            assert_eq!(deserialized.moduli(), moduli);
        }
    }
}
