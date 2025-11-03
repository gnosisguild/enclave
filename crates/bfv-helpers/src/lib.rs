// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod client;
mod util;

use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_primitives::U256;
use fhe::bfv::{BfvParameters, BfvParametersBuilder, Ciphertext, Encoding, Plaintext};
use fhe_traits::{DeserializeParametrized, FheDecoder, Serialize};
use std::{array::TryFromSliceError, sync::Arc};

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

/// Encodes BFV parameters into ABI-encoded bytes.
///
/// This function converts BFV parameters into a tuple structure of (degree, plaintext_modulus, moduli[])
/// and then ABI-encodes the tuple using Solidity ABI format. The resulting bytes can be used
/// in smart contracts or for cross-platform serialization.
///
/// # Arguments
///
/// * `params` - The BFV parameters to encode
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the ABI-encoded parameters as a tuple (uint256, uint256, uint256[]).
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
    ]);
    value.abi_encode()
}

/// Decodes BFV parameters from ABI-encoded bytes.
///
/// This function converts ABI-encoded bytes back into BFV parameters.
/// The bytes should represent a tuple (uint256, uint256, uint256[]) containing
/// (degree, plaintext_modulus, moduli[]) as produced by `encode_bfv_params`.
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

/// Encodes BFV ciphertexts into ABI-encoded bytes.
///
/// This function converts a slice of BFV ciphertexts into an array of byte arrays
/// and then ABI-encodes it using Solidity ABI format. The resulting bytes represent
/// a bytes[] array that can be used in smart contracts or for cross-platform serialization.
///
/// # Arguments
///
/// * `ciphertexts` - A slice of ciphertext objects to encode
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the ABI-encoded ciphertexts as a bytes[] array.
pub fn encode_ciphertexts(ciphertext: &[Ciphertext]) -> Vec<u8> {
    let value = DynSolValue::Array(
        ciphertext
            .iter()
            .map(|ct| DynSolValue::Bytes(ct.to_bytes()))
            .collect(),
    );
    value.abi_encode()
}

/// Decodes ABI-encoded bytes back into BFV ciphertexts.
///
/// This function takes ABI-encoded bytes representing a bytes[] array and converts
/// them back into a vector of BFV ciphertext objects.
///
/// # Arguments
///
/// * `encoded` - The ABI-encoded bytes to decode
///
/// # Returns
///
/// Returns a `Result` containing either:
/// - `Ok(Vec<Ciphertext>)` - Successfully decoded ciphertexts
/// - `Err(String)` - An error message if decoding fails
pub fn decode_ciphertexts(
    encoded: &[u8],
    params: &Arc<BfvParameters>,
) -> Result<Vec<Ciphertext>, String> {
    let byte_arrays = decode_byte_array(encoded)?;

    let mut ciphertexts = Vec::with_capacity(byte_arrays.len());
    for (i, bytes) in byte_arrays.into_iter().enumerate() {
        let ciphertext = Ciphertext::from_bytes(&bytes, params)
            .map_err(|e| format!("Failed to deserialize ciphertext {}: {}", i, e))?;
        ciphertexts.push(ciphertext);
    }

    Ok(ciphertexts)
}

/// Decodes ABI-encoded bytes into a vector of byte arrays.
pub fn decode_byte_array(encoded: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    match DynSolType::Array(Box::new(DynSolType::Bytes)).abi_decode(encoded) {
        Ok(DynSolValue::Array(arr)) => arr
            .into_iter()
            .map(|item| match item {
                DynSolValue::Bytes(bytes) => Ok(bytes),
                _ => Err("Expected bytes".to_string()),
            })
            .collect(),
        _ => Err("Decode failed".to_string()),
    }
}

/// Encodes an array of Plaintext to an ABI encoded array of bytes
/// where each bytes field is a byte encoded Vec<u64>
pub fn encode_plaintexts(plaintext: &[Plaintext]) -> Result<Vec<u8>, String> {
    let value = DynSolValue::Array(
        plaintext
            .iter()
            .map(|pt| {
                Vec::<u64>::try_decode(pt, Encoding::poly())
                    .map(|v| {
                        // Convert Vec<u64> to bytes by concatenating each u64's bytes
                        let bytes: Vec<u8> = v.iter().flat_map(|&num| num.to_be_bytes()).collect();
                        DynSolValue::Bytes(bytes)
                    })
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<_, String>>()?,
    );
    Ok(value.abi_encode())
}

/// Decodes ABI encoded bytes[] where each bytes is an encoded Plaintext to Vec<Vec<u64>>
pub fn decode_plaintexts(encoded: &[u8]) -> Result<Vec<Vec<u64>>, String> {
    decode_byte_array(encoded)?
        .into_iter()
        .map(|bytes| {
            bytes
                .chunks_exact(8)
                .map(|c| {
                    c.try_into()
                        .map(u64::from_be_bytes)
                        .map_err(|e: TryFromSliceError| e.to_string())
                })
                .collect()
        })
        .collect()
}

/// Decodes BFV parameters from ABI-encoded bytes and wraps them in an `Arc`.
///
/// This is a convenience function that combines `decode_bfv_params` with `Arc::new`
/// to provide thread-safe shared ownership of the decoded parameters.
/// The input bytes should represent a tuple (uint256, uint256, uint256[]) containing
/// (degree, plaintext_modulus, moduli[]) in ABI-encoded format.
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
    use anyhow::Result;
    use fhe::bfv::{Encoding, Plaintext};

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
    fn test_encoding_roundtrip() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let encoded = encode_bfv_params(&params);
        let decoded = decode_bfv_params(&encoded);

        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli.as_slice());
    }

    #[test]
    fn test_encoding_deterministic() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);

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

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let encoded = encode_bfv_params(&params);

        // Verify we can decode back to the original parameters with Arc
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
        fn test_params_encoding_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params(degree, plaintext_modulus, &moduli);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params(&encoded);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(decoded.degree(), degree);
            assert_eq!(decoded.plaintext(), plaintext_modulus);
            assert_eq!(decoded.moduli(), moduli);
        }

        #[test]
        fn test_params_arc_encoding_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
            let encoded = encode_bfv_params(&params);
            let decoded = decode_bfv_params_arc(&encoded);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(decoded.degree(), degree);
            assert_eq!(decoded.plaintext(), plaintext_modulus);
            assert_eq!(decoded.moduli(), moduli);
        }

        #[test]
        fn test_real_bfv_params() -> Result<()> {
            let decoded = decode_bfv_params_arc(&hex::decode("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000fc00100000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000003fffffff000001")?);
            Ok(())
        }

        #[test]
        fn test_real_bfv_params_2() -> Result<()> {
            let bytes = [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 192, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 63, 255, 255, 255, 0,
                0, 1,
            ];

            let params = decode_bfv_params_arc(&bytes);
            assert_eq!(params.plaintext(), 1032193);
            Ok(())
        }
    }

    #[cfg(test)]
    mod ciphertext_tests {
        use super::*;
        use anyhow::*;
        use fhe::bfv::{PublicKey, SecretKey};
        use fhe_traits::{FheEncoder, FheEncrypter};
        use rand::{thread_rng, Rng};

        #[test]
        fn test_ciphertext_encoding() -> Result<()> {
            let number = 31415u64;
            let mut rng = thread_rng();
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = Arc::new(build_bfv_params(degree, plaintext_modulus, &moduli));
            let sk = SecretKey::random(&params, &mut rng);
            let pk = PublicKey::new(&sk, &mut rng);
            let pt = Plaintext::try_encode(&[number], Encoding::poly(), &params)?;
            let ct = pk.try_encrypt(&pt, &mut rng)?;
            let encoded = encode_ciphertexts(&[ct.clone()]);
            let decoded = decode_ciphertexts(&encoded, &params).map_err(|e| anyhow!("{e}"))?;
            assert_eq!(decoded, vec![ct]);
            Ok(())
        }

        #[test]
        fn test_ciphertext_encoding_fuzz_multiple() -> Result<()> {
            let mut rng = thread_rng();
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = Arc::new(build_bfv_params(degree, plaintext_modulus, &moduli));
            let sk = SecretKey::random(&params, &mut rng);
            let pk = PublicKey::new(&sk, &mut rng);

            // Test with 100 iterations, each with 1-10 random ciphertexts
            for _ in 0..100 {
                let count = rng.gen_range(1..=10);
                let mut ciphertexts = Vec::new();

                for _ in 0..count {
                    let number = rng.gen::<u32>() as u64; // XXX: generating a u64 fails with assertion failed: *x < 4 * self.p.p
                    let pt = Plaintext::try_encode(&[number], Encoding::poly(), &params)?;
                    let ct = pk.try_encrypt(&pt, &mut rng)?;
                    ciphertexts.push(ct);
                }

                let encoded = encode_ciphertexts(&ciphertexts);
                let decoded = decode_ciphertexts(&encoded, &params).map_err(|e| anyhow!("{e}"))?;
                assert_eq!(decoded, ciphertexts);
            }

            Ok(())
        }

        #[test]
        fn test_plaintext_encoding_roundtrip() -> Result<()> {
            let mut raw: Vec<Vec<u64>> = vec![vec![1243, 567890], vec![31415, 926535]];
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = Arc::new(build_bfv_params(degree, plaintext_modulus, &moduli));

            let plaintexts = raw
                .clone()
                .into_iter()
                .map(|r| {
                    Plaintext::try_encode(&r, Encoding::poly(), &params).map_err(|e| anyhow!("{e}"))
                })
                .collect::<Result<Vec<_>>>()?;

            let encoded = encode_plaintexts(&plaintexts).map_err(|e| anyhow!("{e}"))?;
            println!("encoded length = {}", encoded.len());
            let decoded = decode_plaintexts(&encoded).map_err(|e| anyhow!("{e}"))?;
            // resize the original vec to account for padding.
            raw.iter_mut().for_each(|v| v.resize(degree, 0));
            assert_eq!(raw, decoded);
            Ok(())
        }
    }
}
