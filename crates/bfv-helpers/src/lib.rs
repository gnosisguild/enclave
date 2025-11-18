// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod client;
mod util;

use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_primitives::U256;
use fhe::bfv::{BfvParameters, BfvParametersBuilder, Encoding, Plaintext};
use fhe_traits::FheDecoder;
use std::sync::Arc;
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Plaintext decoding failed")]
    PlaintextDecodeFailed,
    // TODO: add errors from client.rs
    #[error("Input was not encoded correctly")]
    BadEncoding,
    #[error("Unknown parameter set: {0}")]
    UnknownParamSet(String),
}

/// Result that returns a type T or a BfvHelpersError
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, EnumString, IntoStaticStr, EnumIter)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
/// Predefined BFV parameters for common use cases
/// Note that 10 is the default value for both error1 and error2 variance
/// for both BFV and TRBFV (if not explicitly set).
pub enum BfvParams {
    // List parameter strings and variants here
    //
    /// Standard BFV development parameters set (DO NOT USE IN PRODUCTION).
    /// - Degree: 2048 (polynomial ring size)
    /// - Plaintext modulus: 1032193
    /// - Moduli: [0x3FFFFFFF000001] (provides good security level)
    #[strum(serialize = "INSECURE_SET_2048_1032193_1")]
    InsecureSet2048_1032193_1,

    /// Testing TrBFV development parameters set (DO NOT USE IN PRODUCTION).
    /// - Degree: 512
    /// - Moduli: [0xffffee001, 0xffffc4001]
    /// - Plaintext modulus: 10
    /// - Error2 Variance: 3
    #[strum(serialize = "INSECURE_SET_512_10_1")]
    InsecureSet512_10_1,

    /// Testing BFV development parameters for share encryption (DO NOT USE IN PRODUCTION).
    /// - Degree: 512
    /// - Moduli: [0x7fffffffe0001]
    /// - Plaintext modulus: 0xffffee001
    /// - Error2 Variance: 3
    #[strum(serialize = "INSECURE_SET_512_0XFFFFEE001_1")]
    InsecureSet512_0xffffee001_1,

    /// 128bits security TRBFV parameters set (PRODUCTION READY).
    /// - Degree: 8192
    /// - Plaintext modulus: 1000
    /// - Moduli: [0x00800000022a0001, 0x00800000021a0001, 0x0080000002120001, 0x0080000001f60001]
    /// - Error2 Variance: 52309181128222339698631578526730685514457152477762943514050560000
    #[strum(serialize = "SET_8192_1000_4")]
    Set8192_1000_4,

    /// 128bits security BFV parameters set (PRODUCTION READY).
    /// - Degree: 8192
    /// - Plaintext modulus: 144115188075855872
    /// - Moduli: [288230376173076481, 288230376167047169]
    #[strum(serialize = "SET_8192_144115188075855872_2")]
    Set8192_144115188075855872_2,
}

// Map for getters
impl BfvParams {
    /// Return the given param set based on the input key &str.
    pub fn get_params_by_str(key: &str) -> Result<BfvParamSet> {
        key.parse::<BfvParams>()
            .map(|k| k.into())
            .map_err(|_| Error::UnknownParamSet(key.to_string()))
    }

    /// List all the available parameter keys
    pub fn get_params_list() -> Vec<String> {
        BfvParams::iter()
            .map(|key| {
                let s: &'static str = key.into();
                s.to_string()
            })
            .collect()
    }
}

impl From<BfvParams> for BfvParamSet {
    fn from(value: BfvParams) -> Self {
        use BfvParams as B;
        match value {
            // List each new parameter set here
            B::InsecureSet2048_1032193_1 => BfvParamSet {
                degree: 2048,
                plaintext_modulus: 1032193,
                moduli: &[0x3FFFFFFF000001],
                error1_variance: None,
            },
            B::InsecureSet512_10_1 => BfvParamSet {
                degree: 512,
                moduli: &[0xffffee001, 0xffffc4001],
                plaintext_modulus: 10,
                error1_variance: Some("3"),
            },
            B::InsecureSet512_0xffffee001_1 => BfvParamSet {
                degree: 512,
                moduli: &[0x7fffffffe0001],
                plaintext_modulus: 0xffffee001,
                error1_variance: None,
            },
            B::Set8192_1000_4 => BfvParamSet {
                degree: 8192,
                plaintext_modulus: 1000,
                moduli: &[
                    0x00800000022a0001,
                    0x00800000021a0001,
                    0x0080000002120001,
                    0x0080000001f60001,
                ],
                error1_variance: Some(
                    "52309181128222339698631578526730685514457152477762943514050560000",
                ),
            },
            B::Set8192_144115188075855872_2 => BfvParamSet {
                degree: 8192,
                plaintext_modulus: 144115188075855872,
                moduli: &[288230376173076481, 288230376167047169],
                error1_variance: None,
            },
        }
    }
}

/// A consistent type representing a BFV parameter set.
///
/// This struct provides a uniform way to represent BFV parameter sets,
/// making it easy to consume them with functions like `build_bfv_params_from_set`.
#[derive(Debug, Clone, Copy)]
pub struct BfvParamSet {
    /// The degree of the polynomial ring, must be a power of 2
    pub degree: usize,
    /// The modulus for the plaintext space
    pub plaintext_modulus: u64,
    /// The moduli for the ciphertext space
    pub moduli: &'static [u64],
    /// Optional error2 variance (as decimal string). If None, defaults to "10"
    pub error1_variance: Option<&'static str>,
}

impl BfvParamSet {
    /// Build the BfvParamSet into an fhe.rs BfvParameters struct
    pub fn build(self) -> BfvParameters {
        build_bfv_params_from_set(self)
    }

    /// Build the BfvParamSet into an fhe.rs Arc<BfvParameters> struct
    pub fn build_arc(self) -> Arc<BfvParameters> {
        Arc::new(self.build())
    }
}

/// Builds BFV parameters from a `BfvParamSet`.
///
/// This is a convenience function that consumes a `BfvParamSet` struct
/// and builds the corresponding `BfvParameters` instance.
///
/// # Arguments
///
/// * `param_set` - A `BfvParamSet` containing the degree, plaintext modulus, moduli, and optional error2 variance
///
/// # Returns
///
/// Returns a `BfvParameters` instance configured with the specified parameters.
pub fn build_bfv_params_from_set(param_set: BfvParamSet) -> BfvParameters {
    build_bfv_params(
        param_set.degree,
        param_set.plaintext_modulus,
        param_set.moduli,
        param_set.error1_variance,
    )
}

/// Builds BFV parameters from a `BfvParamSet` wrapped in an `Arc`.
///
/// This is a convenience function that consumes a `BfvParamSet` struct
/// and builds the corresponding `Arc<BfvParameters>` instance for thread-safe shared ownership.
///
/// # Arguments
///
/// * `param_set` - A `BfvParamSet` containing the degree, plaintext modulus, moduli, and optional error2 variance
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` instance configured with the specified parameters.
pub fn build_bfv_params_from_set_arc(param_set: BfvParamSet) -> Arc<BfvParameters> {
    build_bfv_params_arc(
        param_set.degree,
        param_set.plaintext_modulus,
        param_set.moduli,
        param_set.error1_variance,
    )
}

/// Builds BFV (Brakerski-Fan-Vercauteren) encryption parameters.
///
/// This function supports both standard BFV and threshold BFV (trBFV) parameters.
/// If `error1_variance` is not provided (None), it defaults to "10", which matches
/// the default variance value for standard BFV.
///
/// # Arguments
///
/// * `degree` - The degree of the polynomial ring, must be a power of 2
/// * `plaintext_modulus` - The modulus for the plaintext space
/// * `moduli` - The moduli for the ciphertext space
/// * `error1_variance` - Optional error2 variance (as decimal string). Defaults to "10" if None.
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
    error1_variance: Option<&str>,
) -> BfvParameters {
    let mut builder = BfvParametersBuilder::new();
    builder
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli);

    if let Some(error1) = error1_variance {
        builder
            .set_error1_variance_str(error1)
            .unwrap_or_else(|e| panic!("Failed to set error1_variance: {}", e));
    }
    // If error1_variance is None, the builder defaults to 10

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
/// * `error1_variance` - Optional error2 variance (as decimal string). Defaults to "10" if None.
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
    error1_variance: Option<&str>,
) -> Arc<BfvParameters> {
    let mut builder = BfvParametersBuilder::new();
    builder
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli);

    if let Some(error1) = error1_variance {
        builder
            .set_error1_variance_str(error1)
            .unwrap_or_else(|e| panic!("Failed to set error1_variance: {}", e));
    }
    // If error1_variance is None, the builder defaults to 10

    builder
        .build_arc()
        .unwrap_or_else(|e| panic!("Failed to build BFV Parameters wrapped in Arc: {}", e))
}

/// Encodes BFV parameters into ABI-encoded bytes.
///
/// This function converts BFV parameters into a tuple structure of (degree, plaintext_modulus, moduli[], error1_variance)
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
        DynSolValue::String(params.get_error1_variance().to_string()),
    ]);
    value.abi_encode()
}

/// Decodes BFV parameters from ABI-encoded bytes.
///
/// This function converts ABI-encoded bytes back into BFV parameters.
/// The bytes should represent a tuple (uint256, uint256, uint256[], string) containing
/// (degree, plaintext_modulus, moduli[], error1_variance) as produced by `encode_bfv_params`.
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
        DynSolType::String,                                 // error1_variance (as decimal string)
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

            // Extract error1_variance (fourth element)
            let error1_variance: String = match &inner_values[3] {
                DynSolValue::String(val) => val.clone(),
                _ => panic!("Expected string for error1_variance"),
            };

            let params = BfvParametersBuilder::new()
                .set_degree(degree as usize)
                .set_plaintext_modulus(plaintext)
                .set_moduli(&moduli)
                .set_error1_variance_str(&error1_variance)
                .unwrap_or_else(|e| panic!("Failed to set error1_variance: {}", e))
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
/// (degree, plaintext_modulus, moduli[], error1_variance) in ABI-encoded format.
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

/// Decode Plaintext to a Vec<u64>
pub fn decode_plaintext_to_vec_u64(value: &Plaintext) -> Result<Vec<u64>> {
    let decoded = Vec::<u64>::try_decode(&value, Encoding::poly())
        .map_err(|_| Error::PlaintextDecodeFailed)?;

    Ok(decoded)
}

/// Convert from a Vec<u64> to Vec<u8>
pub fn encode_vec_u64_to_bytes(value: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for num in &value.to_vec() {
        bytes.extend_from_slice(&num.to_le_bytes());
    }
    bytes
}

/// Decode bytes to Vec<u64>
pub fn decode_bytes_to_vec_u64(bytes: &[u8]) -> Result<Vec<u64>> {
    if bytes.len() % 8 != 0 {
        return Err(Error::BadEncoding);
    }

    Ok(bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
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
        assert_eq!(params.get_error1_variance(), &BigUint::from(10u32));
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
        assert_eq!(params.get_error1_variance(), &BigUint::from(10u32));
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
        let error1_variance = "52309181128222339698631578526730685514457152477762943514050560000";

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, Some(error1_variance));
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), 10);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from_str(error1_variance).unwrap()
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
        let error1_variance = "52309181128222339698631578526730685514457152477762943514050560000";

        let params =
            build_bfv_params_arc(degree, plaintext_modulus, &moduli, Some(error1_variance));
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), 10);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from_str(error1_variance).unwrap()
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
        // Verify error1_variance is preserved (defaults to 10 for standard BFV)
        assert_eq!(decoded.get_error1_variance(), params.get_error1_variance());
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
        // Verify error1_variance is preserved
        assert_eq!(decoded.get_error1_variance(), params.get_error1_variance());
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
        let error1_variance = "52309181128222339698631578526730685514457152477762943514050560000";

        let params = build_bfv_params(degree, plaintext_modulus, &moduli, Some(error1_variance));
        let encoded = encode_bfv_params(&params);
        let decoded = decode_bfv_params(&encoded);

        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli);
        // Verify error1_variance is preserved for trBFV
        assert_eq!(
            decoded.get_error1_variance(),
            &BigUint::from_str(error1_variance).unwrap()
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
            let param_set: BfvParamSet = BfvParams::InsecureSet2048_1032193_1.into();
            assert_eq!(param_set.degree, 2048);
            assert_eq!(param_set.plaintext_modulus, 1032193);
            assert_eq!(param_set.moduli, &[0x3FFFFFFF000001]);
        }

        #[test]
        fn test_params_function() {
            let param_set = BfvParams::InsecureSet2048_1032193_1.into();
            let params = build_bfv_params_from_set(param_set);

            assert_eq!(params.degree(), param_set.degree);
            assert_eq!(params.plaintext(), param_set.plaintext_modulus);
            assert_eq!(params.moduli(), param_set.moduli);
        }

        #[test]
        fn test_params_arc_function() {
            let param_set = BfvParams::InsecureSet2048_1032193_1.into();
            let params = build_bfv_params_from_set_arc(param_set);

            assert_eq!(params.degree(), param_set.degree);
            assert_eq!(params.plaintext(), param_set.plaintext_modulus);
            assert_eq!(params.moduli(), param_set.moduli);
        }

        #[test]
        fn test_params_encoding_roundtrip() {
            let param_set = BfvParams::InsecureSet2048_1032193_1.into();
            let params = build_bfv_params_from_set(param_set);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params(&encoded);

            assert_eq!(decoded.degree(), param_set.degree);
            assert_eq!(decoded.plaintext(), param_set.plaintext_modulus);
            assert_eq!(decoded.moduli(), param_set.moduli);
            // Verify error1_variance is preserved
            assert_eq!(decoded.get_error1_variance(), params.get_error1_variance());
        }

        #[test]
        fn test_params_arc_encoding_roundtrip() {
            let param_set = BfvParams::InsecureSet2048_1032193_1.into();
            let params = build_bfv_params_from_set_arc(param_set);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params_arc(&encoded);

            assert_eq!(decoded.degree(), param_set.degree);
            assert_eq!(decoded.plaintext(), param_set.plaintext_modulus);
            assert_eq!(decoded.moduli(), param_set.moduli);
            // Verify error1_variance is preserved
            assert_eq!(decoded.get_error1_variance(), params.get_error1_variance());
        }

        #[test]
        fn test_params_trbfv_encoding_roundtrip() {
            let param_set = BfvParams::Set8192_1000_4.into();
            let params = build_bfv_params_from_set(param_set);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params(&encoded);

            assert_eq!(decoded.degree(), param_set.degree);
            assert_eq!(decoded.plaintext(), param_set.plaintext_modulus);
            assert_eq!(decoded.moduli(), param_set.moduli);
            // Verify error1_variance is preserved for trBFV
            assert_eq!(
                decoded.get_error1_variance(),
                &BigUint::from_str(param_set.error1_variance.unwrap()).unwrap()
            );
        }
    }
}
