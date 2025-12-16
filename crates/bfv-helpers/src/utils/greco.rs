// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy_dyn_abi::{DynSolType, DynSolValue};
use alloy_primitives::I256;
use fhe::bfv::{BfvParameters, Ciphertext};
use fhe_math::rq::{traits::TryConvertFrom, Poly, Representation};
use itertools::izip;
use ndarray::Array2;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use shared::constants::get_zkp_modulus;
use std::sync::Arc;

/// Converts a greco coefficient (centered, in standard form) to BFV format [0, qi).
/// Standard-form coefficients are centered coefficients reduced mod ZKP modulus.
/// If standard_form >= zkp_modulus/2, it represents a negative centered coefficient.
fn convert_greco_coefficient_to_bfv(centered_coeff: &BigInt, qi: u64, zkp_modulus: &BigInt) -> u64 {
    let qi_bigint = BigInt::from(qi);
    let half_zkp = zkp_modulus / 2u64;

    // Recover centered coefficient mod qi
    // If standard_form >= zkp_modulus/2, it's a negative centered value: centered = standard_form - zkp_modulus
    let centered_mod_qi = if centered_coeff >= &half_zkp {
        (centered_coeff - zkp_modulus) % &qi_bigint
    } else {
        centered_coeff % &qi_bigint
    };

    // Un-center: convert from [-(qi-1)/2, (qi-1)/2] to [0, qi)
    // Use modular arithmetic: (centered_mod_qi + qi) % qi ensures result is in [0, qi)
    let result = (&centered_mod_qi + &qi_bigint) % &qi_bigint;
    result
        .to_u64()
        .expect("Result should be in [0, qi) and fit in u64")
}

/// Converts greco-formatted coefficients (reversed, centered) to BFV coefficients.
fn convert_greco_coefficients_to_bfv(
    greco_coeffs: &[BigInt],
    qi: u64,
    zkp_modulus: &BigInt,
) -> Vec<u64> {
    greco_coeffs
        .iter()
        .rev()
        .map(|coeff| convert_greco_coefficient_to_bfv(coeff, qi, zkp_modulus))
        .collect()
}

/// Converts greco-formatted coefficients back to a BFV ciphertext.
///
/// Takes greco-formatted coefficients (centered, reversed, in standard form) and reconstructs
/// the BFV ciphertext. Conversion is exact modulo qi for each modulus.
///
/// # Safety
/// This function assumes valid input:
/// - `ct0is` and `ct1is` must have length equal to the number of moduli
/// - Each coefficient vector must have length equal to the polynomial degree
///
/// # Arguments
/// * `ct0is` - Greco coefficients for ct0 (one vector per modulus, standard form)
/// * `ct1is` - Greco coefficients for ct1 (one vector per modulus, standard form)
/// * `params` - BFV parameters
pub fn greco_to_bfv_ciphertext(
    ct0is: &[Vec<BigInt>],
    ct1is: &[Vec<BigInt>],
    params: &Arc<BfvParameters>,
) -> Ciphertext {
    let moduli = params.moduli();
    let degree = params.degree();

    // Convert greco coefficients to BFV format for each modulus
    let zkp_modulus = get_zkp_modulus();
    let mut ct0_coeffs_all = Vec::with_capacity(moduli.len());
    let mut ct1_coeffs_all = Vec::with_capacity(moduli.len());

    for (ct0i, ct1i, qi) in izip!(ct0is, ct1is, moduli) {
        ct0_coeffs_all.push(convert_greco_coefficients_to_bfv(ct0i, *qi, &zkp_modulus));
        ct1_coeffs_all.push(convert_greco_coefficients_to_bfv(ct1i, *qi, &zkp_modulus));
    }

    // Create Poly objects with all RNS limbs
    let ctx = params.ctx()[0].clone();
    let ct0_array = Array2::from_shape_fn((moduli.len(), degree), |(i, j)| ct0_coeffs_all[i][j]);
    let ct1_array = Array2::from_shape_fn((moduli.len(), degree), |(i, j)| ct1_coeffs_all[i][j]);

    let mut ct0_poly =
        Poly::try_convert_from(ct0_array, &ctx, false, Some(Representation::PowerBasis))
            .expect("Failed to create ct0 Poly: invalid coefficient format");
    let mut ct1_poly =
        Poly::try_convert_from(ct1_array, &ctx, false, Some(Representation::PowerBasis))
            .expect("Failed to create ct1 Poly: invalid coefficient format");

    ct0_poly.change_representation(Representation::Ntt);
    ct1_poly.change_representation(Representation::Ntt);

    Ciphertext::new(vec![ct0_poly, ct1_poly], params)
        .expect("Failed to create Ciphertext: invalid polynomial format")
}

/// Decodes ABI-encoded greco ciphertext from bytes32[] array.
///
/// The bytes are expected to be ABI-encoded bytes32[] arrays from Solidity contracts.
/// The array contains ct0is coefficients followed by ct1is coefficients, where each
/// coefficient is a bytes32 value. The coefficients are organized as:
/// - First `num_moduli * degree` bytes32 values are ct0is (grouped by modulus)
/// - Next `num_moduli * degree` bytes32 values are ct1is (grouped by modulus)
///
/// # Safety
/// This function assumes valid input:
/// - `bytes` must be valid ABI-encoded bytes32[] array
/// - Array must contain exactly `2 * num_moduli * degree` bytes32 values
/// - All values must be valid bytes32 FixedBytes
///
/// # Arguments
/// * `bytes` - ABI-encoded bytes32[] array containing greco ciphertext coefficients
/// * `params` - BFV parameters (used to determine num_moduli and degree)
///
/// # Returns
/// A tuple of (ct0is, ct1is) where each is Vec<Vec<BigInt>> (one vector per modulus)
pub fn abi_decode_greco_ciphertext(
    bytes: &[u8],
    params: &Arc<BfvParameters>,
) -> (Vec<Vec<BigInt>>, Vec<Vec<BigInt>>) {
    let degree = params.degree();
    let num_moduli = params.moduli().len();

    // ABI-decode the bytes to get bytes32[] array
    let array_type = DynSolType::Array(Box::new(DynSolType::FixedBytes(32)));
    let decoded = array_type
        .abi_decode(bytes)
        .expect("Failed to ABI decode bytes32[] array: invalid encoding");

    let bytes32_array = match decoded {
        DynSolValue::Array(arr) => arr,
        _ => panic!("Expected array from ABI decode, got invalid type"),
    };

    let ct0is_bytes32_count = num_moduli * degree;

    // Split into ct0is and ct1is (use slices to avoid unnecessary allocations)
    let (ct0is_bytes32, ct1is_bytes32) = bytes32_array.split_at(ct0is_bytes32_count);

    // Helper function to extract bytes32 from DynSolValue (assumes valid input)
    fn extract_bytes32(value: &DynSolValue) -> [u8; 32] {
        match value {
            DynSolValue::FixedBytes(b, _) => b
                .as_slice()
                .try_into()
                .expect("Invalid bytes32 length: expected 32 bytes"),
            _ => panic!("Expected bytes32 FixedBytes, got invalid type"),
        }
    }

    // Convert bytes32 arrays to greco coefficient format
    let mut ct0is = Vec::with_capacity(num_moduli);
    let mut ct1is = Vec::with_capacity(num_moduli);

    for i in 0..num_moduli {
        let mut ct0_modulus = Vec::with_capacity(degree);
        let mut ct1_modulus = Vec::with_capacity(degree);

        for j in 0..degree {
            let idx = i * degree + j;

            // Convert ct0 and ct1 bytes32 to BigInt
            let ct0_bytes32 = extract_bytes32(&ct0is_bytes32[idx]);
            let ct1_bytes32 = extract_bytes32(&ct1is_bytes32[idx]);

            ct0_modulus.push(bytes32_to_bigint(&ct0_bytes32));
            ct1_modulus.push(bytes32_to_bigint(&ct1_bytes32));
        }

        ct0is.push(ct0_modulus);
        ct1is.push(ct1_modulus);
    }

    (ct0is, ct1is)
}

/// Converts bytes32 (signed 256-bit, two's complement, big-endian) to BigInt
fn bytes32_to_bigint(bytes: &[u8; 32]) -> BigInt {
    // Use I256::from_be_bytes which handles two's complement conversion automatically
    let i256 = I256::from_be_bytes(*bytes);

    // Convert I256 to BigInt via its string representation
    // I256 handles two's complement correctly, so we can use its Display implementation
    use std::str::FromStr;
    BigInt::from_str(&i256.to_string())
        .expect("I256::to_string() should always produce a valid BigInt string")
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::FixedBytes;
    use fhe::bfv::{Encoding, Plaintext, PublicKey, SecretKey};
    use fhe_traits::{DeserializeParametrized, FheEncoder, Serialize};
    use greco::bounds::GrecoBounds;
    use greco::vectors::GrecoVectors;
    use rand::thread_rng;

    /// Helper function to set up test parameters, keys, and ciphertext
    fn setup_test() -> (
        Arc<BfvParameters>,
        Ciphertext,
        (Vec<Vec<BigInt>>, Vec<Vec<BigInt>>),
    ) {
        use crate::{BfvParamSet, BfvParamSets};
        let params = BfvParamSet::from(BfvParamSets::InsecureSet512_10_1).build_arc();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let vote = vec![1u64, 0u64, 0u64];
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &params).unwrap();
        let (ct, u_rns, e0_rns, e1_rns) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();

        let (_, bounds) = GrecoBounds::compute(&params, 0).unwrap();

        let bit_pk =
            shared::template::calculate_bit_width(&bounds.pk_bounds[0].to_string()).unwrap();

        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &params, bit_pk)
                .unwrap();
        let standard_vectors = greco_vectors.standard_form();

        (params, ct, (standard_vectors.ct0is, standard_vectors.ct1is))
    }

    /// Helper function to convert BigInt to bytes32 (big-endian, two's complement)
    fn bigint_to_bytes32(bigint: &BigInt) -> [u8; 32] {
        use std::str::FromStr;
        let i256 = I256::from_str(&bigint.to_string())
            .expect("BigInt should fit in I256 range for bytes32 conversion");
        i256.to_be_bytes()
    }

    #[test]
    fn test_greco_to_bfv_ciphertext() {
        let (params, original_ct, (ct0is, ct1is)) = setup_test();

        let reconstructed_ct = greco_to_bfv_ciphertext(&ct0is, &ct1is, &params);

        assert_eq!(reconstructed_ct.c.len(), original_ct.c.len());
        assert_eq!(reconstructed_ct.level, original_ct.level);

        // Verify serialization/deserialization works
        let ct_bytes = reconstructed_ct.to_bytes();
        let deserialized_ct = Ciphertext::from_bytes(&ct_bytes, &params).unwrap();
        assert_eq!(deserialized_ct.c.len(), original_ct.c.len());
    }

    #[test]
    fn test_abi_decode_greco_ciphertext_round_trip() {
        let (params, original_ct, (ct0is, ct1is)) = setup_test();

        // Convert greco coefficients to bytes32[] and ABI-encode
        let mut bytes32_array = Vec::new();
        for coeffs in [&ct0is, &ct1is] {
            for modulus_coeffs in coeffs {
                for coeff in modulus_coeffs {
                    let bytes32 = bigint_to_bytes32(coeff);
                    bytes32_array.push(DynSolValue::FixedBytes(FixedBytes::from(bytes32), 32));
                }
            }
        }

        let encoded_bytes = DynSolValue::Array(bytes32_array).abi_encode();

        // Test full round-trip: ABI decode -> greco -> BFV.
        let (ct0is, ct1is) = abi_decode_greco_ciphertext(&encoded_bytes, &params);
        let reconstructed_ct = greco_to_bfv_ciphertext(&ct0is, &ct1is, &params);

        assert_eq!(reconstructed_ct.c.len(), original_ct.c.len());
        assert_eq!(reconstructed_ct.level, original_ct.level);
    }
}
