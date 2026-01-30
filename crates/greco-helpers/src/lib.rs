// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_zk_helpers::utils::get_zkp_modulus;
use fhe::bfv::{BfvParameters, Ciphertext, PublicKey};
use fhe_math::rq::Representation;
use num_bigint::BigInt;
use std::sync::Arc;

/// Converts a BFV coefficient (in [0, qi)) to centered format [-(qi-1)/2, (qi-1)/2].
fn convert_bfv_coefficient_to_centered(coeff: u64, qi: u64) -> BigInt {
    let qi_bigint = BigInt::from(qi);
    let coeff_bigint = BigInt::from(coeff);

    // Center: convert from [0, qi) to [-(qi-1)/2, (qi-1)/2]
    // If coeff > qi/2, it represents a negative centered value
    if coeff > (qi / 2) {
        &coeff_bigint - &qi_bigint
    } else {
        coeff_bigint
    }
}

/// Converts BFV coefficients to greco-formatted coefficients (centered, reversed, standard form).
fn convert_bfv_coefficients_to_greco(
    bfv_coeffs: &[u64],
    qi: u64,
    zkp_modulus: &BigInt,
) -> Vec<BigInt> {
    bfv_coeffs
        .iter()
        .rev()
        .map(|coeff| {
            let centered = convert_bfv_coefficient_to_centered(*coeff, qi);
            // Reduce mod ZKP modulus to get standard form
            // Handle negative values correctly
            let centered_mod = centered % zkp_modulus;
            if centered_mod < BigInt::from(0) {
                centered_mod + zkp_modulus
            } else {
                centered_mod
            }
        })
        .collect()
}

/// Converts a BFV ciphertext to Greco format.
///
/// Takes a BFV ciphertext and converts it to Greco format, returning ct0is and ct1is
/// as vectors of coefficient vectors (one vector per modulus, standard form).
///
/// # Arguments
/// * `ct` - BFV ciphertext
/// * `params` - BFV parameters
///
/// # Returns
/// A tuple of (ct0is, ct1is) where each is Vec<Vec<BigInt>> (one vector per modulus)
pub fn bfv_ciphertext_to_greco(
    ct: &Ciphertext,
    params: &Arc<BfvParameters>,
) -> (Vec<Vec<BigInt>>, Vec<Vec<BigInt>>) {
    let moduli = params.moduli();
    let degree = params.degree();
    let zkp_modulus = get_zkp_modulus();

    let ct0_poly = &ct.c[0];
    let ct1_poly = &ct.c[1];

    let mut ct0_power = ct0_poly.clone();
    let mut ct1_power = ct1_poly.clone();
    ct0_power.change_representation(Representation::PowerBasis);
    ct1_power.change_representation(Representation::PowerBasis);

    let mut ct0is = Vec::with_capacity(moduli.len());
    let mut ct1is = Vec::with_capacity(moduli.len());

    for (i, qi) in moduli.iter().enumerate() {
        let mut ct0_modulus = Vec::with_capacity(degree);
        let mut ct1_modulus = Vec::with_capacity(degree);

        for j in 0..degree {
            let ct0_coeff = ct0_power.coefficients()[(i, j)];
            let ct1_coeff = ct1_power.coefficients()[(i, j)];

            ct0_modulus.push(ct0_coeff);
            ct1_modulus.push(ct1_coeff);
        }

        ct0is.push(convert_bfv_coefficients_to_greco(
            &ct0_modulus,
            *qi,
            &zkp_modulus,
        ));
        ct1is.push(convert_bfv_coefficients_to_greco(
            &ct1_modulus,
            *qi,
            &zkp_modulus,
        ));
    }

    (ct0is, ct1is)
}

/// Converts a BFV public key to Greco format.
///
/// Takes a BFV public key and converts it to Greco format, returning pk0is and pk1is
/// as vectors of coefficient vectors (one vector per modulus, standard form).
///
/// # Arguments
/// * `pk` - BFV public key
/// * `params` - BFV parameters
///
/// # Returns
/// A tuple of (pk0is, pk1is) where each is Vec<Vec<BigInt>> (one vector per modulus)
pub fn bfv_public_key_to_greco(
    pk: &PublicKey,
    params: &Arc<BfvParameters>,
) -> (Vec<Vec<BigInt>>, Vec<Vec<BigInt>>) {
    let moduli = params.moduli();
    let degree = params.degree();
    let zkp_modulus = get_zkp_modulus();

    // Access pk0 and pk1 polynomials from the public key
    // PublicKey has a .c field that is a Ciphertext, which contains .c with the polynomials
    let pk0_poly = &pk.c.c[0];
    let pk1_poly = &pk.c.c[1];

    // Convert polynomials to power basis representation to access coefficients
    let mut pk0_power = pk0_poly.clone();
    let mut pk1_power = pk1_poly.clone();
    pk0_power.change_representation(Representation::PowerBasis);
    pk1_power.change_representation(Representation::PowerBasis);

    let mut pk0is = Vec::with_capacity(moduli.len());
    let mut pk1is = Vec::with_capacity(moduli.len());

    // Extract coefficients for each modulus
    for (i, qi) in moduli.iter().enumerate() {
        let mut pk0_modulus = Vec::with_capacity(degree);
        let mut pk1_modulus = Vec::with_capacity(degree);

        for j in 0..degree {
            // Access coefficient at (modulus_index, coefficient_index)
            let pk0_coeff = pk0_power.coefficients()[(i, j)];
            let pk1_coeff = pk1_power.coefficients()[(i, j)];

            pk0_modulus.push(pk0_coeff);
            pk1_modulus.push(pk1_coeff);
        }

        // Convert to greco format (centers, reverses, and reduces mod ZKP modulus)
        pk0is.push(convert_bfv_coefficients_to_greco(
            &pk0_modulus,
            *qi,
            &zkp_modulus,
        ));
        pk1is.push(convert_bfv_coefficients_to_greco(
            &pk1_modulus,
            *qi,
            &zkp_modulus,
        ));
    }

    (pk0is, pk1is)
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_config::bfv_config::DEFAULT_BFV_PRESET;
    use e3_fhe_params::BfvParamSet;
    use e3_zk_helpers::utils::calculate_bit_width;
    use fhe::bfv::{Encoding, Plaintext, PublicKey, SecretKey};
    use fhe_traits::FheEncoder;
    use greco::bounds::GrecoBounds;
    use greco::vectors::GrecoVectors;
    use rand::thread_rng;

    #[test]
    fn test_bfv_public_key_to_greco() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        // Get expected pk0is and pk1is from GrecoVectors
        let (_, bounds) = GrecoBounds::compute(&params, 0).unwrap();
        let bit_pk = calculate_bit_width(&bounds.pk_bounds[0].to_string()).unwrap();

        let vote = vec![1u64, 0u64, 0u64];
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &params).unwrap();
        let (ct, u_rns, e0_rns, e1_rns) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();

        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &params, bit_pk)
                .unwrap();
        let standard_vectors = greco_vectors.standard_form();
        let expected_pk0is = &standard_vectors.pk0is;
        let expected_pk1is = &standard_vectors.pk1is;

        // Convert using our function
        let (actual_pk0is, actual_pk1is) = bfv_public_key_to_greco(&pk, &params);

        // Verify the structure matches
        assert_eq!(actual_pk0is.len(), expected_pk0is.len());
        assert_eq!(actual_pk1is.len(), expected_pk1is.len());
        assert_eq!(actual_pk0is.len(), params.moduli().len());

        // Verify coefficients match for each modulus
        for (i, (actual_pk0i, expected_pk0i)) in
            actual_pk0is.iter().zip(expected_pk0is.iter()).enumerate()
        {
            assert_eq!(
                actual_pk0i.len(),
                expected_pk0i.len(),
                "pk0is[{}] length mismatch",
                i
            );
            assert_eq!(
                actual_pk0i.len(),
                params.degree(),
                "pk0is[{}] should have degree coefficients",
                i
            );

            for (j, (actual_coeff, expected_coeff)) in
                actual_pk0i.iter().zip(expected_pk0i.iter()).enumerate()
            {
                assert_eq!(
                    actual_coeff, expected_coeff,
                    "pk0is[{}][{}] coefficient mismatch",
                    i, j
                );
            }
        }

        for (i, (actual_pk1i, expected_pk1i)) in
            actual_pk1is.iter().zip(expected_pk1is.iter()).enumerate()
        {
            assert_eq!(
                actual_pk1i.len(),
                expected_pk1i.len(),
                "pk1is[{}] length mismatch",
                i
            );
            assert_eq!(
                actual_pk1i.len(),
                params.degree(),
                "pk1is[{}] should have degree coefficients",
                i
            );

            for (j, (actual_coeff, expected_coeff)) in
                actual_pk1i.iter().zip(expected_pk1i.iter()).enumerate()
            {
                assert_eq!(
                    actual_coeff, expected_coeff,
                    "pk1is[{}][{}] coefficient mismatch",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_bfv_ciphertext_to_greco() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        // Get expected ct0is and ct1is from GrecoVectors
        let (_, bounds) = GrecoBounds::compute(&params, 0).unwrap();
        let bit_pk = calculate_bit_width(&bounds.pk_bounds[0].to_string()).unwrap();

        let vote = vec![1u64, 0u64, 0u64];
        let pt = Plaintext::try_encode(&vote, Encoding::poly(), &params).unwrap();
        let (ct, u_rns, e0_rns, e1_rns) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();

        let greco_vectors =
            GrecoVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &params, bit_pk)
                .unwrap();
        let standard_vectors = greco_vectors.standard_form();
        let expected_ct0is = &standard_vectors.ct0is;
        let expected_ct1is = &standard_vectors.ct1is;

        // Convert using our function
        let (actual_ct0is, actual_ct1is) = bfv_ciphertext_to_greco(&ct, &params);

        // Verify the structure matches
        assert_eq!(actual_ct0is.len(), expected_ct0is.len());
        assert_eq!(actual_ct1is.len(), expected_ct1is.len());
        assert_eq!(actual_ct0is.len(), params.moduli().len());

        // Verify coefficients match for each modulus
        for (i, (actual_ct0i, expected_ct0i)) in
            actual_ct0is.iter().zip(expected_ct0is.iter()).enumerate()
        {
            assert_eq!(
                actual_ct0i.len(),
                expected_ct0i.len(),
                "ct0is[{}] length mismatch",
                i
            );
            assert_eq!(
                actual_ct0i.len(),
                params.degree(),
                "ct0is[{}] should have degree coefficients",
                i
            );

            for (j, (actual_coeff, expected_coeff)) in
                actual_ct0i.iter().zip(expected_ct0i.iter()).enumerate()
            {
                assert_eq!(
                    actual_coeff, expected_coeff,
                    "ct0is[{}][{}] coefficient mismatch",
                    i, j
                );
            }
        }

        for (i, (actual_ct1i, expected_ct1i)) in
            actual_ct1is.iter().zip(expected_ct1is.iter()).enumerate()
        {
            assert_eq!(
                actual_ct1i.len(),
                expected_ct1i.len(),
                "ct1is[{}] length mismatch",
                i
            );
            assert_eq!(
                actual_ct1i.len(),
                params.degree(),
                "ct1is[{}] should have degree coefficients",
                i
            );

            for (j, (actual_coeff, expected_coeff)) in
                actual_ct1i.iter().zip(expected_ct1i.iter()).enumerate()
            {
                assert_eq!(
                    actual_coeff, expected_coeff,
                    "ct1is[{}][{}] coefficient mismatch",
                    i, j
                );
            }
        }
    }
}
