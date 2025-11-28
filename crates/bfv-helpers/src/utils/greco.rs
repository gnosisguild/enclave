// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy_dyn_abi::{DynSolType, DynSolValue};
use anyhow::{Context, Result};
use fhe::bfv::{BfvParameters, Ciphertext};
use fhe_math::rq::{traits::TryConvertFrom, Poly, Representation};
use itertools::izip;
use ndarray::Array2;
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
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
    if centered_mod_qi < BigInt::zero() {
        (&centered_mod_qi + &qi_bigint).to_u64().unwrap_or(0)
    } else {
        centered_mod_qi.to_u64().unwrap_or(0)
    }
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
/// # Arguments
/// * `ct0is` - Greco coefficients for ct0 (one vector per modulus, standard form)
/// * `ct1is` - Greco coefficients for ct1 (one vector per modulus, standard form)
/// * `params` - BFV parameters
pub fn greco_to_bfv_ciphertext(
    ct0is: &[Vec<BigInt>],
    ct1is: &[Vec<BigInt>],
    params: &Arc<BfvParameters>,
) -> Result<Ciphertext> {
    let moduli = params.moduli();
    let degree = params.degree();

    anyhow::ensure!(
        ct0is.len() == moduli.len() && ct1is.len() == moduli.len(),
        "Mismatch in number of moduli: expected {}, got ct0={}, ct1={}",
        moduli.len(),
        ct0is.len(),
        ct1is.len()
    );

    // Convert greco coefficients to BFV format for each modulus
    let zkp_modulus = get_zkp_modulus();
    let mut ct0_coeffs_all = Vec::with_capacity(moduli.len());
    let mut ct1_coeffs_all = Vec::with_capacity(moduli.len());

    for (idx, (ct0i, ct1i, qi)) in izip!(ct0is, ct1is, moduli).enumerate() {
        anyhow::ensure!(
            ct0i.len() == degree && ct1i.len() == degree,
            "Coefficient length mismatch at modulus {}: expected {}, got ct0={}, ct1={}",
            idx,
            degree,
            ct0i.len(),
            ct1i.len()
        );

        ct0_coeffs_all.push(convert_greco_coefficients_to_bfv(ct0i, *qi, &zkp_modulus));
        ct1_coeffs_all.push(convert_greco_coefficients_to_bfv(ct1i, *qi, &zkp_modulus));
    }

    // Create Poly objects with all RNS limbs
    let ctx = params.ctx()[0].clone();
    let ct0_array = Array2::from_shape_fn((moduli.len(), degree), |(i, j)| ct0_coeffs_all[i][j]);
    let ct1_array = Array2::from_shape_fn((moduli.len(), degree), |(i, j)| ct1_coeffs_all[i][j]);

    let mut ct0_poly =
        Poly::try_convert_from(ct0_array, &ctx, false, Some(Representation::PowerBasis))
            .context("Failed to create ct0 Poly")?;
    let mut ct1_poly =
        Poly::try_convert_from(ct1_array, &ctx, false, Some(Representation::PowerBasis))
            .context("Failed to create ct1 Poly")?;

    ct0_poly.change_representation(Representation::Ntt);
    ct1_poly.change_representation(Representation::Ntt);

    Ciphertext::new(vec![ct0_poly, ct1_poly], params).context("Failed to create Ciphertext")
}

/// Deserializes greco coefficients from bytes format.
/// Format: [num_moduli: u8][for each modulus: [num_coeffs: u16][coeffs: bytes32[]]]
/// The bytes are expected to be serialized from Solidity bytes32[] arrays.
pub fn deserialize_greco_coefficients(bytes: &[u8]) -> Result<Vec<Vec<BigInt>>> {
    let mut offset = 0;

    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    // Read number of moduli
    let num_moduli = bytes[offset] as usize;
    offset += 1;

    let mut result = Vec::with_capacity(num_moduli);

    for _ in 0..num_moduli {
        if offset + 2 > bytes.len() {
            anyhow::bail!("Insufficient bytes for degree");
        }

        // Read number of coefficients
        let num_coeffs = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        offset += 2;

        if offset + num_coeffs * 32 > bytes.len() {
            anyhow::bail!("Insufficient bytes for coefficients");
        }

        let mut modulus_coeffs = Vec::with_capacity(num_coeffs);
        for _ in 0..num_coeffs {
            let coeff_bytes: [u8; 32] = bytes[offset..offset + 32]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to read coefficient bytes"))?;
            modulus_coeffs.push(bytes32_to_bigint(&coeff_bytes));
            offset += 32;
        }

        result.push(modulus_coeffs);
    }

    Ok(result)
}

/// Decodes ABI-encoded greco ciphertext from bytes32[] array.
///
/// The bytes are expected to be ABI-encoded bytes32[] arrays from Solidity contracts.
/// The array contains ct0is coefficients followed by ct1is coefficients, where each
/// coefficient is a bytes32 value. The coefficients are organized as:
/// - First `num_moduli * degree` bytes32 values are ct0is (grouped by modulus)
/// - Next `num_moduli * degree` bytes32 values are ct1is (grouped by modulus)
///
/// # Arguments
/// * `bytes` - ABI-encoded bytes32[] array containing greco ciphertext coefficients
/// * `num_moduli` - Number of moduli in the BFV parameters
/// * `degree` - Polynomial degree
///
/// # Returns
/// A tuple of (ct0is, ct1is) where each is Vec<Vec<BigInt>> (one vector per modulus)
pub fn abi_decode_greco_ciphertext(
    bytes: &[u8],
    num_moduli: usize,
    degree: usize,
) -> Result<(Vec<Vec<BigInt>>, Vec<Vec<BigInt>>)> {
    // ABI-decode the bytes to get bytes32[] array
    let array_type = DynSolType::Array(Box::new(DynSolType::FixedBytes(32)));
    let decoded = array_type
        .abi_decode(bytes)
        .context("Failed to ABI decode bytes32[] array")?;

    let bytes32_array = match decoded {
        DynSolValue::Array(arr) => arr,
        _ => anyhow::bail!("Expected array from ABI decode"),
    };

    let ct0is_bytes32_count = num_moduli * degree;

    // Split into ct0is and ct1is (each needs num_moduli * degree bytes32 values)
    anyhow::ensure!(
        bytes32_array.len() >= ct0is_bytes32_count * 2,
        "Insufficient bytes32 values: expected at least {}, got {}",
        ct0is_bytes32_count * 2,
        bytes32_array.len()
    );

    // Extract ct0is bytes32 values
    let ct0is_bytes32: Vec<DynSolValue> = bytes32_array[..ct0is_bytes32_count].to_vec();
    // Extract ct1is bytes32 values
    let ct1is_bytes32: Vec<DynSolValue> =
        bytes32_array[ct0is_bytes32_count..ct0is_bytes32_count * 2].to_vec();

    // Convert bytes32 arrays to greco coefficient format
    let mut ct0is = Vec::with_capacity(num_moduli);
    let mut ct1is = Vec::with_capacity(num_moduli);

    for i in 0..num_moduli {
        let mut ct0_modulus = Vec::with_capacity(degree);
        let mut ct1_modulus = Vec::with_capacity(degree);

        for j in 0..degree {
            let idx = i * degree + j;

            // Convert ct0 bytes32 to BigInt
            let ct0_bytes32 = match &ct0is_bytes32[idx] {
                DynSolValue::FixedBytes(b, _) => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(b.as_slice());
                    arr
                }
                _ => anyhow::bail!("Expected bytes32 for ct0 at index {}", idx),
            };
            ct0_modulus.push(bytes32_to_bigint(&ct0_bytes32));

            // Convert ct1 bytes32 to BigInt
            let ct1_bytes32 = match &ct1is_bytes32[idx] {
                DynSolValue::FixedBytes(b, _) => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(b.as_slice());
                    arr
                }
                _ => anyhow::bail!("Expected bytes32 for ct1 at index {}", idx),
            };
            ct1_modulus.push(bytes32_to_bigint(&ct1_bytes32));
        }

        ct0is.push(ct0_modulus);
        ct1is.push(ct1_modulus);
    }

    Ok((ct0is, ct1is))
}

/// Converts ABI-encoded greco ciphertext bytes to BFV ciphertext bytes.
///
/// This is a convenience function that combines ABI decoding and greco-to-BFV conversion.
///
/// # Arguments
/// * `bytes` - ABI-encoded bytes32[] array containing greco ciphertext coefficients
/// * `params` - BFV parameters
///
/// # Returns
/// Serialized BFV ciphertext bytes ready to be used with `Ciphertext::from_bytes`
pub fn abi_decode_greco_to_bfv_bytes(bytes: &[u8], params: &Arc<BfvParameters>) -> Result<Vec<u8>> {
    let degree = params.degree();
    let num_moduli = params.moduli().len();

    let (ct0is, ct1is) = abi_decode_greco_ciphertext(bytes, num_moduli, degree)?;
    let ciphertext = greco_to_bfv_ciphertext(&ct0is, &ct1is, params)?;

    use fhe_traits::Serialize;
    Ok(ciphertext.to_bytes())
}

/// Converts bytes32 (signed 256-bit, two's complement, big-endian) to BigInt
fn bytes32_to_bigint(bytes: &[u8; 32]) -> BigInt {
    // Check if negative (MSB is 1)
    let is_negative = bytes[0] >= 0x80;

    if is_negative {
        // Two's complement: invert all bits and add 1, then negate
        let mut inverted = [0u8; 32];
        for i in 0..32 {
            inverted[i] = !bytes[i];
        }

        // Add 1
        let mut carry = 1u16;
        for i in (0..32).rev() {
            let sum = inverted[i] as u16 + carry;
            inverted[i] = sum as u8;
            carry = sum >> 8;
        }

        -BigInt::from_bytes_be(Sign::Plus, &inverted)
    } else {
        BigInt::from_bytes_be(Sign::Plus, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::FixedBytes;
    use fhe::bfv::{BfvParametersBuilder, Encoding, Plaintext, PublicKey, SecretKey};
    use fhe_traits::{DeserializeParametrized, FheEncoder, Serialize};
    use greco::vectors::GrecoVectors;
    use rand::thread_rng;

    #[test]
    fn test_greco_to_bfv_ciphertext() {
        // Test with two moduli to verify multi-modulus support
        let moduli = [0xffffee001u64, 0xffffc4001u64];
        let params = BfvParametersBuilder::new()
            .set_degree(512)
            .set_plaintext_modulus(10)
            .set_moduli(&moduli)
            .build_arc()
            .unwrap();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let vote = vec![1u64, 0u64, 0u64];
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &params).unwrap();
        let (ct, u_rns, e0_rns, e1_rns) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();

        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &params).unwrap();

        let standard_vectors = greco_vectors.standard_form();
        let reconstructed_ct =
            greco_to_bfv_ciphertext(&standard_vectors.ct0is, &standard_vectors.ct1is, &params)
                .unwrap();

        assert_eq!(reconstructed_ct.c.len(), 2);
        assert_eq!(reconstructed_ct.level, 0);

        // Verify serialization/deserialization works
        let ct_bytes = reconstructed_ct.to_bytes();
        let deserialized_ct = Ciphertext::from_bytes(&ct_bytes, &params).unwrap();
        assert_eq!(deserialized_ct.c.len(), 2);

        // Verify exact coefficient recovery for all moduli
        let mut ct0_orig = ct.c[0].clone();
        let mut ct1_orig = ct.c[1].clone();
        let mut ct0_recon = reconstructed_ct.c[0].clone();
        let mut ct1_recon = reconstructed_ct.c[1].clone();

        ct0_orig.change_representation(Representation::PowerBasis);
        ct1_orig.change_representation(Representation::PowerBasis);
        ct0_recon.change_representation(Representation::PowerBasis);
        ct1_recon.change_representation(Representation::PowerBasis);

        let orig_coeffs0 = ct0_orig.coefficients();
        let recon_coeffs0 = ct0_recon.coefficients();
        let orig_coeffs1 = ct1_orig.coefficients();
        let recon_coeffs1 = ct1_recon.coefficients();

        for (mod_idx, qi) in params.moduli().iter().enumerate() {
            let orig0 = orig_coeffs0.row(mod_idx);
            let recon0 = recon_coeffs0.row(mod_idx);
            let orig1 = orig_coeffs1.row(mod_idx);
            let recon1 = recon_coeffs1.row(mod_idx);

            for (i, ((&o0, &r0), (&o1, &r1))) in orig0
                .iter()
                .zip(recon0.iter())
                .zip(orig1.iter().zip(recon1.iter()))
                .enumerate()
            {
                assert_eq!(
                    o0 % qi,
                    r0 % qi,
                    "ct0[{}] mismatch at modulus {}",
                    i,
                    mod_idx
                );
                assert_eq!(
                    o1 % qi,
                    r1 % qi,
                    "ct1[{}] mismatch at modulus {}",
                    i,
                    mod_idx
                );
            }
        }
    }

    /// Helper function to convert BigInt to bytes32 (big-endian, two's complement)
    fn bigint_to_bytes32(bigint: &BigInt) -> [u8; 32] {
        use num_bigint::Sign;
        let (sign, bytes_be) = bigint.to_bytes_be();
        let mut result = [0u8; 32];

        if sign == Sign::Minus {
            // For negative numbers, convert to two's complement
            let mut abs_bytes = vec![0u8; 32];
            let start_idx = 32usize.saturating_sub(bytes_be.len());
            abs_bytes[start_idx..].copy_from_slice(&bytes_be);

            // Invert all bits
            for i in 0..32 {
                result[i] = !abs_bytes[i];
            }

            // Add 1
            let mut carry = 1u16;
            for i in (0..32).rev() {
                let sum = result[i] as u16 + carry;
                result[i] = sum as u8;
                carry = sum >> 8;
            }
        } else {
            // For positive numbers, pad with zeros
            let start_idx = 32usize.saturating_sub(bytes_be.len());
            result[start_idx..].copy_from_slice(&bytes_be);
        }

        result
    }

    #[test]
    fn test_abi_decode_greco_ciphertext() {
        // Test with two moduli to verify multi-modulus support
        let moduli = [0xffffee001u64, 0xffffc4001u64];
        let params = BfvParametersBuilder::new()
            .set_degree(512)
            .set_plaintext_modulus(10)
            .set_moduli(&moduli)
            .build_arc()
            .unwrap();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let vote = vec![1u64, 0u64, 0u64];
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &params).unwrap();
        let (ct, u_rns, e0_rns, e1_rns) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();

        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &params).unwrap();

        let standard_vectors = greco_vectors.standard_form();
        let original_ct0is = &standard_vectors.ct0is;
        let original_ct1is = &standard_vectors.ct1is;

        // Convert greco coefficients to bytes32[] and ABI-encode
        let mut bytes32_array = Vec::new();

        // Add ct0is coefficients
        for modulus_coeffs in original_ct0is {
            for coeff in modulus_coeffs {
                let bytes32 = bigint_to_bytes32(coeff);
                bytes32_array.push(DynSolValue::FixedBytes(FixedBytes::from(bytes32), 32));
            }
        }

        // Add ct1is coefficients
        for modulus_coeffs in original_ct1is {
            for coeff in modulus_coeffs {
                let bytes32 = bigint_to_bytes32(coeff);
                bytes32_array.push(DynSolValue::FixedBytes(FixedBytes::from(bytes32), 32));
            }
        }

        // ABI-encode the bytes32[] array
        let array_value = DynSolValue::Array(bytes32_array);
        let encoded_bytes = array_value.abi_encode();

        // Test abi_decode_greco_ciphertext
        let degree = params.degree();
        let num_moduli = params.moduli().len();
        let (decoded_ct0is, decoded_ct1is) =
            abi_decode_greco_ciphertext(&encoded_bytes, num_moduli, degree).unwrap();

        // Verify the decoded coefficients match the original
        assert_eq!(decoded_ct0is.len(), original_ct0is.len());
        assert_eq!(decoded_ct1is.len(), original_ct1is.len());

        for (mod_idx, (decoded_ct0, original_ct0)) in
            decoded_ct0is.iter().zip(original_ct0is.iter()).enumerate()
        {
            assert_eq!(decoded_ct0.len(), original_ct0.len());
            for (decoded_coeff, original_coeff) in decoded_ct0.iter().zip(original_ct0.iter()) {
                assert_eq!(
                    decoded_coeff, original_coeff,
                    "ct0 coefficient mismatch at modulus {}, coefficient index",
                    mod_idx
                );
            }
        }

        for (mod_idx, (decoded_ct1, original_ct1)) in
            decoded_ct1is.iter().zip(original_ct1is.iter()).enumerate()
        {
            assert_eq!(decoded_ct1.len(), original_ct1.len());
            for (decoded_coeff, original_coeff) in decoded_ct1.iter().zip(original_ct1.iter()) {
                assert_eq!(
                    decoded_coeff, original_coeff,
                    "ct1 coefficient mismatch at modulus {}, coefficient index",
                    mod_idx
                );
            }
        }
    }

    #[test]
    fn test_abi_decode_greco_to_bfv_bytes() {
        // Test with two moduli to verify multi-modulus support
        let moduli = [0xffffee001u64, 0xffffc4001u64];
        let params = BfvParametersBuilder::new()
            .set_degree(512)
            .set_plaintext_modulus(10)
            .set_moduli(&moduli)
            .build_arc()
            .unwrap();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let vote = vec![1u64, 0u64, 0u64];
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &params).unwrap();
        let (original_ct, u_rns, e0_rns, e1_rns) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();

        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &original_ct, &pk, &params)
                .unwrap();

        let standard_vectors = greco_vectors.standard_form();

        // Convert greco coefficients to bytes32[] and ABI-encode
        let mut bytes32_array = Vec::new();

        // Add ct0is coefficients
        for modulus_coeffs in &standard_vectors.ct0is {
            for coeff in modulus_coeffs {
                let bytes32 = bigint_to_bytes32(coeff);
                bytes32_array.push(DynSolValue::FixedBytes(FixedBytes::from(bytes32), 32));
            }
        }

        // Add ct1is coefficients
        for modulus_coeffs in &standard_vectors.ct1is {
            for coeff in modulus_coeffs {
                let bytes32 = bigint_to_bytes32(coeff);
                bytes32_array.push(DynSolValue::FixedBytes(FixedBytes::from(bytes32), 32));
            }
        }

        // ABI-encode the bytes32[] array
        let array_value = DynSolValue::Array(bytes32_array);
        let encoded_bytes = array_value.abi_encode();

        // Test abi_decode_greco_to_bfv_bytes
        let bfv_bytes = abi_decode_greco_to_bfv_bytes(&encoded_bytes, &params).unwrap();

        // Deserialize the BFV ciphertext
        let reconstructed_ct = Ciphertext::from_bytes(&bfv_bytes, &params).unwrap();

        assert_eq!(reconstructed_ct.c.len(), 2);
        assert_eq!(reconstructed_ct.level, 0);

        // Verify exact coefficient recovery for all moduli
        let mut ct0_orig = original_ct.c[0].clone();
        let mut ct1_orig = original_ct.c[1].clone();
        let mut ct0_recon = reconstructed_ct.c[0].clone();
        let mut ct1_recon = reconstructed_ct.c[1].clone();

        ct0_orig.change_representation(Representation::PowerBasis);
        ct1_orig.change_representation(Representation::PowerBasis);
        ct0_recon.change_representation(Representation::PowerBasis);
        ct1_recon.change_representation(Representation::PowerBasis);

        let orig_coeffs0 = ct0_orig.coefficients();
        let recon_coeffs0 = ct0_recon.coefficients();
        let orig_coeffs1 = ct1_orig.coefficients();
        let recon_coeffs1 = ct1_recon.coefficients();

        for (mod_idx, qi) in params.moduli().iter().enumerate() {
            let orig0 = orig_coeffs0.row(mod_idx);
            let recon0 = recon_coeffs0.row(mod_idx);
            let orig1 = orig_coeffs1.row(mod_idx);
            let recon1 = recon_coeffs1.row(mod_idx);

            for (i, ((&o0, &r0), (&o1, &r1))) in orig0
                .iter()
                .zip(recon0.iter())
                .zip(orig1.iter().zip(recon1.iter()))
                .enumerate()
            {
                assert_eq!(
                    o0 % qi,
                    r0 % qi,
                    "ct0[{}] mismatch at modulus {}",
                    i,
                    mod_idx
                );
                assert_eq!(
                    o1 % qi,
                    r1 % qi,
                    "ct1[{}] mismatch at modulus {}",
                    i,
                    mod_idx
                );
            }
        }
    }
}
