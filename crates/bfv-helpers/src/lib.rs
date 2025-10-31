// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod client;
mod util;

use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_primitives::U256;
use fhe::bfv::{BfvParameters, BfvParametersBuilder};
use std::sync::Arc;

/// Predefined BFV parameters for common use cases
pub mod params {
    /// Standard BFV parameters sets
    /// Each set is a tuple of (degree, plaintext_modulus, moduli).
    /// Naming convention: SET_<degree>_<plaintext_modulus>_<moduli_count>

    /// Note that 10 is the default value for both error1 and error2 variance
    /// for both BFV and TRBFV (if not explicitly set).

    /// Standard development parameters set (DO NOT USE IN PRODUCTION).
    /// - Degree: 2048 (polynomial ring size)
    /// - Plaintext modulus: 1032193
    /// - Moduli: [0x3FFFFFFF000001] (provides good security level)
    pub const SET_2048_1032193_1: (usize, u64, [u64; 1]) = (2048, 1032193, [0x3FFFFFFF000001]);

    /// 128bits security TRBFV parameters set (PRODUCTION READY).
    /// - Degree: 8192
    /// - Plaintext modulus: 1000
    /// - Moduli: [0x00800000022a0001, 0x00800000021a0001, 0x0080000002120001, 0x0080000001f60001]
    /// - Error2 Variance: 52309181128222339698631578526730685514457152477762943514050560000
    pub const SET_8192_1000_4: (usize, u64, [u64; 4], &str) = (
        8192,
        1000,
        [
            0x00800000022a0001,
            0x00800000021a0001,
            0x0080000002120001,
            0x0080000001f60001,
        ],
        "52309181128222339698631578526730685514457152477762943514050560000",
    );

    /// 128bits security BFV parameters set (PRODUCTION READY).
    /// - Degree: 8192
    /// - Plaintext modulus: 144115188075855872
    /// - Moduli: [288230376173076481, 288230376167047169]
    pub const SET_8192_144115188075855872_2: (usize, u64, [u64; 2]) = (
        8192,
        144115188075855872,
        [288230376173076481, 288230376167047169],
    );
}

/// Builds BFV (Brakerski-Fan-Vercauteren) encryption parameters.
///
/// This function supports both standard BFV and threshold BFV (trBFV) parameters.
/// If `error2_variance` is not provided (None), it defaults to "10", which matches
/// the default variance value for standard BFV.
///
/// # Arguments
///
/// * `degree` - The degree of the polynomial ring, must be a power of 2
/// * `plaintext_modulus` - The modulus for the plaintext space
/// * `moduli` - The moduli for the ciphertext space
/// * `error2_variance` - Optional error2 variance (as decimal string). Defaults to "10" if None.
///
/// # Returns
///
/// Returns a `BfvParameters` instance configured with the specified parameters.
///
/// # Panics
///
/// Panics if the parameters cannot be built (e.g., invalid degree or moduli).
pub fn build_bfv_params(
    degree: usize,
    plaintext_modulus: u64,
    moduli: &[u64],
    error2_variance: Option<&str>,
) -> BfvParameters {
    let mut builder = BfvParametersBuilder::new();
    builder
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli);

    if let Some(error2) = error2_variance {
        builder
            .set_error2_variance_str(error2)
            .unwrap_or_else(|e| panic!("Failed to set error2_variance: {}", e));
    }
    // If error2_variance is None, the builder defaults to 10

    builder
        .build()
        .unwrap_or_else(|e| panic!("Failed to build BFV Parameters: {}", e))
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
/// * `error2_variance` - Optional error2 variance (as decimal string). Defaults to "10" if None.
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
    error2_variance: Option<&str>,
) -> Arc<BfvParameters> {
    let mut builder = BfvParametersBuilder::new();
    builder
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli);

    if let Some(error2) = error2_variance {
        builder
            .set_error2_variance_str(error2)
            .unwrap_or_else(|e| panic!("Failed to set error2_variance: {}", e));
    }
    // If error2_variance is None, the builder defaults to 10

    builder
        .build_arc()
        .unwrap_or_else(|e| panic!("Failed to build BFV Parameters wrapped in Arc: {}", e))
}

/// Encodes BFV parameters into ABI-encoded bytes.
///
/// This function converts BFV parameters into a tuple structure of (degree, plaintext_modulus, moduli[], error2_variance)
/// and then ABI-encodes the tuple using Solidity ABI format. The resulting bytes can be used
/// in smart contracts or for cross-platform serialization.
/// # Arguments
///
/// * `params` - The BFV parameters to encode
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the ABI-encoded parameters as a tuple (uint256, uint256, uint256[], string).
pub fn encode_bfv_params(params: &BfvParameters) -> Vec<u8> {
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
        DynSolValue::String(params.get_error2_variance().to_string()),
    ]);
    value.abi_encode()
}

/// Decodes BFV parameters from ABI-encoded bytes.
///
/// This function converts ABI-encoded bytes back into BFV parameters.
/// The bytes should represent a tuple (uint256, uint256, uint256[], string) containing
/// (degree, plaintext_modulus, moduli[], error2_variance) as produced by `encode_bfv_params`.
///
/// # Arguments
///
/// * `bytes` - The ABI-encoded bytes containing the encoded parameters
///
/// # Returns
///
/// Returns a `BfvParameters` instance decoded from the bytes.
///
/// # Panics
///
/// Panics if the decoding fails due to invalid format or parameter values.
pub fn decode_bfv_params(bytes: &[u8]) -> BfvParameters {
    // Define the expected tuple type: (uint256, uint256, uint256[], string)
    let tuple_type = DynSolType::Tuple(vec![
        DynSolType::Uint(256),                              // degree
        DynSolType::Uint(256),                              // plaintext_modulus
        DynSolType::Array(Box::new(DynSolType::Uint(256))), // moduli array
        DynSolType::String,                                 // error2_variance (as decimal string)
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

            // Extract error2_variance (fourth element)
            let error2_variance: String = match &inner_values[3] {
                DynSolValue::String(val) => val.clone(),
                _ => panic!("Expected string for error2_variance"),
            };

            let params = BfvParametersBuilder::new()
                .set_degree(degree as usize)
                .set_plaintext_modulus(plaintext)
                .set_moduli(&moduli)
                .set_error2_variance_str(&error2_variance)
                .unwrap_or_else(|e| panic!("Failed to set error2_variance: {}", e))
                .build()
                .unwrap_or_else(|e| panic!("Failed to build BFV Parameters: {}", e));

            params
        }
        _ => panic!("Expected tuple value in ABI encoding"),
    }
}

/// Decodes BFV parameters from ABI-encoded bytes and wraps them in an `Arc`.
///
/// This is a convenience function that combines `decode_bfv_params` with `Arc::new`
/// to provide thread-safe shared ownership of the decoded parameters.
/// The input bytes should represent a tuple (uint256, uint256, uint256[], string) containing
/// (degree, plaintext_modulus, moduli[], error2_variance) in ABI-encoded format.
///
/// # Arguments
///
/// * `bytes` - The ABI-encoded bytes containing the encoded parameters
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` instance decoded from the bytes.
///
/// # Panics
///
/// Panics if the decoding fails (see `decode_bfv_params`).
pub fn decode_bfv_params_arc(bytes: &[u8]) -> Arc<BfvParameters> {
    Arc::new(decode_bfv_params(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;
    use std::str::FromStr;

    #[test]
    fn test_build_bfv_params() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, None);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), 10);
        assert_eq!(params.get_error2_variance(), &BigUint::from(10u32));
    }

    #[test]
    fn test_build_bfv_params_arc() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli, None);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), 10);
        assert_eq!(params.get_error2_variance(), &BigUint::from(10u32));
    }

    #[test]
    fn test_build_trbfv_params() {
        let degree = 8192;
        let plaintext_modulus = 1000;
        let moduli = [
            0x00800000022a0001,
            0x00800000021a0001,
            0x0080000002120001,
            0x0080000001f60001,
        ];
        let error2_variance = "52309181128222339698631578526730685514457152477762943514050560000";

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, Some(error2_variance));
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), 10);
        assert_eq!(
            params.get_error2_variance(),
            &BigUint::from_str(error2_variance).unwrap()
        );
    }

    #[test]
    fn test_build_trbfv_params_arc() {
        let degree = 8192;
        let plaintext_modulus = 1000;
        let moduli = [
            0x00800000022a0001,
            0x00800000021a0001,
            0x0080000002120001,
            0x0080000001f60001,
        ];
        let error2_variance = "52309181128222339698631578526730685514457152477762943514050560000";

        let params =
            build_bfv_params_arc(degree, plaintext_modulus, &moduli, Some(error2_variance));
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), 10);
        assert_eq!(
            params.get_error2_variance(),
            &BigUint::from_str(error2_variance).unwrap()
        );
    }

    #[test]
    fn test_encoding_roundtrip() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, None);
        let encoded = encode_bfv_params(&params);
        let decoded = decode_bfv_params(&encoded);

        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli.as_slice());
        // Verify error2_variance is preserved (defaults to 10 for standard BFV)
        assert_eq!(decoded.get_error2_variance(), params.get_error2_variance());
    }

    #[test]
    fn test_encoding_deterministic() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, None);

        // Verify the encoding result is deterministic
        let encoded1 = encode_bfv_params(&params);
        let encoded2 = encode_bfv_params(&params);
        assert_eq!(encoded1, encoded2, "ABI encoding should be deterministic");
    }

    #[test]
    fn test_encoding_roundtrip_arc() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, None);
        let encoded = encode_bfv_params(&params);

        // Verify we can decode back to the original parameters with Arc
        let decoded = decode_bfv_params_arc(&encoded);
        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli.as_slice());
        // Verify error2_variance is preserved
        assert_eq!(decoded.get_error2_variance(), params.get_error2_variance());
    }

    #[test]
    fn test_encoding_roundtrip_trbfv() {
        let degree = 8192;
        let plaintext_modulus = 1000;
        let moduli = [
            0x00800000022a0001,
            0x00800000021a0001,
            0x0080000002120001,
            0x0080000001f60001,
        ];
        let error2_variance = "52309181128222339698631578526730685514457152477762943514050560000";

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, Some(error2_variance));
        let encoded = encode_bfv_params(&params);
        let decoded = decode_bfv_params(&encoded);

        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli);
        // Verify error2_variance is preserved for trBFV
        assert_eq!(
            decoded.get_error2_variance(),
            &BigUint::from_str(error2_variance).unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "Failed to ABI decode bytes")]
    fn test_decode_bfv_params_error() {
        let invalid_bytes = vec![0u8; 10];
        let _ = decode_bfv_params(&invalid_bytes);
    }

    #[cfg(test)]
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
            let params = build_bfv_params(degree, plaintext_modulus, &moduli, None);

            assert_eq!(params.degree(), degree);
            assert_eq!(params.plaintext(), plaintext_modulus);
            assert_eq!(params.moduli(), moduli);
        }

        #[test]
        fn test_params_arc_function() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli, None);

            assert_eq!(params.degree(), degree);
            assert_eq!(params.plaintext(), plaintext_modulus);
            assert_eq!(params.moduli(), moduli);
        }

        #[test]
        fn test_params_encoding_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params(degree, plaintext_modulus, &moduli, None);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params(&encoded);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(decoded.degree(), degree);
            assert_eq!(decoded.plaintext(), plaintext_modulus);
            assert_eq!(decoded.moduli(), moduli);
            // Verify error2_variance is preserved
            assert_eq!(decoded.get_error2_variance(), params.get_error2_variance());
        }

        #[test]
        fn test_params_arc_encoding_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli, None);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params_arc(&encoded);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(decoded.degree(), degree);
            assert_eq!(decoded.plaintext(), plaintext_modulus);
            assert_eq!(decoded.moduli(), moduli);
            // Verify error2_variance is preserved
            assert_eq!(decoded.get_error2_variance(), params.get_error2_variance());
        }

        #[test]
        fn test_params_trbfv_encoding_roundtrip() {
            let (degree, plaintext_modulus, moduli, error2_variance) = params::SET_8192_1000_4;
            let params =
                build_bfv_params(degree, plaintext_modulus, &moduli, Some(error2_variance));
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params(&encoded);

            assert_eq!(decoded.degree(), degree);
            assert_eq!(decoded.plaintext(), plaintext_modulus);
            assert_eq!(decoded.moduli(), moduli);
            // Verify error2_variance is preserved for trBFV
            assert_eq!(
                decoded.get_error2_variance(),
                &BigUint::from_str(error2_variance).unwrap()
            );
        }
    }
}
