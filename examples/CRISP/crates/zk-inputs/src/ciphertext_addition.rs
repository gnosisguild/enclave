// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_polynomial::{reduce_coefficients, CrtPolynomial, Polynomial};
use e3_zk_helpers::commitments::compute_ciphertext_commitment;
use e3_zk_helpers::utils::get_zkp_modulus;
use eyre::{Context, Result};
use fhe::bfv::BfvParameters;
use fhe::bfv::Ciphertext;
use num_bigint::BigInt;
use std::sync::Arc;

/// Set of inputs for validation of a ciphertext addition.
///
/// This struct contains all the necessary data to prove that a ciphertext addition
/// was performed correctly in the zero-knowledge proof system.
#[derive(Clone, Debug)]
pub struct CiphertextAdditionInputs {
    pub prev_ct0is: CrtPolynomial,
    pub prev_ct1is: CrtPolynomial,
    pub sum_ct0is: CrtPolynomial,
    pub sum_ct1is: CrtPolynomial,
    pub r0is: CrtPolynomial,
    pub r1is: CrtPolynomial,
    pub prev_ct_commitment: BigInt,
}

impl CiphertextAdditionInputs {
    /// Computes the ciphertext addition inputs for zero-knowledge proof validation.
    ///
    /// # Arguments
    /// * `prev_ct` - The existing ciphertext to add to
    /// * `ct` - The ciphertext being added (from Greco)
    /// * `sum_ct` - The result of the ciphertext addition
    /// * `params` - BFV parameters
    /// * `bit_ct` - Bit width for ciphertext bounds (used for packing)
    ///
    /// # Returns
    /// CiphertextAdditionInputs containing all necessary proof data
    pub fn compute(
        prev_ct: &Ciphertext,
        ct: &Ciphertext,
        sum_ct: &Ciphertext,
        params: Arc<BfvParameters>,
        bit_ct: u32,
    ) -> Result<CiphertextAdditionInputs> {
        let n = params.degree();
        let moduli = params.moduli();

        let mut crt_polynomials = [
            CrtPolynomial::from_fhe_polynomial(&prev_ct.c[0]),
            CrtPolynomial::from_fhe_polynomial(&prev_ct.c[1]),
            CrtPolynomial::from_fhe_polynomial(&ct.c[0]),
            CrtPolynomial::from_fhe_polynomial(&ct.c[1]),
            CrtPolynomial::from_fhe_polynomial(&sum_ct.c[0]),
            CrtPolynomial::from_fhe_polynomial(&sum_ct.c[1]),
        ];

        // fhe-math stores coefficients in ascending degree (c_0, c_1, …). But here we want
        // that each limb is stored in **descending** order (a_n, …, a_0) so circuit evaluation can use Horner's
        // method in one forward pass: `result = result * x + coefficients[i]` from i = 0,
        // i.e. P(x) = ((…((a_n·x + a_{n-1})·x + …)·x + a_0), with no extra reversing or reindexing.
        for c in &mut crt_polynomials {
            c.reverse();
            c.reduce_and_center(&moduli)?;
        }

        let [prev_ct0, prev_ct1, ct0, ct1, sum_ct0, sum_ct1] = crt_polynomials;

        // Compute quotient polynomials: r = (sum_centered - (ct_centered + prev_ct_centered)) / qi.
        // For ciphertext addition: sum_centered = ct_centered + prev_ct_centered + r * qi.
        // So: r = (sum_centered - (ct_centered + prev_ct_centered)) / qi.
        let r0 = Self::compute_quotient(&sum_ct0, &ct0, &prev_ct0, n, &moduli)
            .with_context(|| "Failed to compute r0 quotient")?;
        let r1 = Self::compute_quotient(&sum_ct1, &ct1, &prev_ct1, n, &moduli)
            .with_context(|| "Failed to compute r1 quotient")?;

        let prev_ct_commitment = compute_ciphertext_commitment(&prev_ct0, &prev_ct1, bit_ct);

        Ok(CiphertextAdditionInputs {
            prev_ct0is: prev_ct0,
            prev_ct1is: prev_ct1,
            sum_ct0is: sum_ct0,
            sum_ct1is: sum_ct1,
            r0is: r0,
            r1is: r1,
            prev_ct_commitment,
        })
    }

    /// Converts the inputs to standard form by reducing coefficients modulo the ZKP modulus.
    ///
    /// # Returns
    /// A new CiphertextAdditionInputs with coefficients reduced to the ZKP modulus
    pub fn standard_form(&self) -> Self {
        let zkp_modulus = &get_zkp_modulus();
        CiphertextAdditionInputs {
            prev_ct0is: reduce_crt_polynomial(&self.prev_ct0is, zkp_modulus),
            prev_ct1is: reduce_crt_polynomial(&self.prev_ct1is, zkp_modulus),
            prev_ct_commitment: self.prev_ct_commitment.clone() % zkp_modulus,
            sum_ct0is: reduce_crt_polynomial(&self.sum_ct0is, zkp_modulus),
            sum_ct1is: reduce_crt_polynomial(&self.sum_ct1is, zkp_modulus),
            r0is: reduce_crt_polynomial(&self.r0is, zkp_modulus),
            r1is: reduce_crt_polynomial(&self.r1is, zkp_modulus),
        }
    }

    /// Computes the quotient CRT polynomial `(sum - (a + b)) / q_i` per modulus.
    ///
    /// For each limb index `i`, divides `sum_i - (a_i + b_i)` by the modulus `q_i`.
    /// Used when verifying that sum ciphertext equals a + b and recovering the
    /// quotient (small integer) from the difference.
    ///
    /// # Arguments
    ///
    /// * `sum` - CRT polynomial of the sum ciphertext
    /// * `a` - CRT polynomial of the first ciphertext
    /// * `b` - CRT polynomial of the second ciphertext
    /// * `n` - polynomial degree (number of coefficients per limb)
    /// * `moduli` - moduli for each CRT limb
    ///
    /// # Returns
    ///
    /// The quotient CRT polynomial, or an error if division is not exact or the
    /// quotient is not in `{-1, 0, 1}`.
    fn compute_quotient(
        sum: &CrtPolynomial,
        a: &CrtPolynomial,
        b: &CrtPolynomial,
        _n: usize,
        moduli: &[u64],
    ) -> Result<CrtPolynomial> {
        let num_moduli = moduli.len();

        let mut quotient_limbs = Vec::with_capacity(num_moduli);

        for i in 0..num_moduli {
            let sum_limb = sum.limb(i);
            let a_limb = a.limb(i);
            let b_limb = b.limb(i);
            let qi = Polynomial::constant(BigInt::from(moduli[i]));

            let diff = sum_limb.sub(&a_limb.add(b_limb));
            let (q_poly, _remainder) = diff
                .div(&qi)
                .map_err(|e| eyre::eyre!("division by modulus q_i at index {}: {}", i, e))?;

            for (j, q) in q_poly.coefficients().iter().enumerate() {
                if *q < (-1).into() || *q > 1.into() {
                    return Err(eyre::eyre!(
                        "Quotient out of range [-1, 1] at modulus index {}, coeff {}: quotient = {}",
                        i,
                        j,
                        q
                    ));
                }
            }

            quotient_limbs.push(q_poly);
        }

        Ok(CrtPolynomial::new(quotient_limbs))
    }
}

/// Reduces all coefficients of a CRT polynomial modulo the given modulus.
fn reduce_crt_polynomial(crt_poly: &CrtPolynomial, modulus: &BigInt) -> CrtPolynomial {
    let reduced_limbs: Vec<Polynomial> = crt_poly
        .limbs
        .iter()
        .map(|limb| {
            let reduced_coeffs = reduce_coefficients(limb.coefficients(), modulus);
            Polynomial::new(reduced_coeffs)
        })
        .collect();

    // Safe to unwrap because we're preserving the structure
    CrtPolynomial::new(reduced_limbs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::{BfvParamSet, BfvPreset};
    use e3_zk_helpers::utils::calculate_bit_width;
    use fhe::bfv::{Encoding, Plaintext, PublicKey, SecretKey};
    use fhe_traits::FheEncoder;
    use greco::bounds::GrecoBounds;
    use rand::thread_rng;

    fn test_bit_ct(params: &Arc<BfvParameters>) -> u32 {
        let (_, bounds) = GrecoBounds::compute(params, 0).unwrap();
        calculate_bit_width(&bounds.pk_bounds[0].to_string()).unwrap()
    }

    fn create_test_generator() -> (Arc<BfvParameters>, PublicKey, SecretKey) {
        let param_set: BfvParamSet = BfvPreset::InsecureThresholdBfv512.into();
        let bfv_params = param_set.build_arc();

        let mut rng = thread_rng();
        let sk = SecretKey::random(&bfv_params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        (bfv_params, pk, sk)
    }

    fn create_test_plaintext(params: &Arc<BfvParameters>, vote: u8) -> Plaintext {
        let mut message_data = vec![3u64; params.degree()];
        message_data[0] = if vote == 1 { 1 } else { 0 };
        Plaintext::try_encode(&message_data, Encoding::poly(), params).unwrap()
    }

    #[test]
    fn test_compute_basic_functionality() {
        let (bfv_params, pk, _sk) = create_test_generator();
        let mut rng = thread_rng();

        // Create test plaintexts.
        let pt1 = create_test_plaintext(&bfv_params, 0);
        let pt2 = create_test_plaintext(&bfv_params, 1);

        // Encrypt plaintexts.
        let (ct1, _u1, _e0_1, _e1_1) = pk.try_encrypt_extended(&pt1, &mut rng).unwrap();
        let (ct2, _u2, _e0_2, _e1_2) = pk.try_encrypt_extended(&pt2, &mut rng).unwrap();

        // Compute sum.
        let sum_ct = &ct1 + &ct2;

        // Compute ciphertext addition inputs.
        let bit_ct = test_bit_ct(&bfv_params);
        let result =
            CiphertextAdditionInputs::compute(&ct1, &ct2, &sum_ct, bfv_params.clone(), bit_ct);

        assert!(result.is_ok());
        let inputs = result.unwrap();

        let num_moduli = bfv_params.moduli().len();
        assert_eq!(inputs.prev_ct0is.limbs.len(), num_moduli);
        assert_eq!(inputs.prev_ct1is.limbs.len(), num_moduli);
        assert_eq!(inputs.sum_ct0is.limbs.len(), num_moduli);
        assert_eq!(inputs.sum_ct1is.limbs.len(), num_moduli);
        assert_eq!(inputs.r0is.limbs.len(), num_moduli);
        assert_eq!(inputs.r1is.limbs.len(), num_moduli);
    }

    #[test]
    fn test_standard_form_conversion() {
        let (bfv_params, pk, _sk) = create_test_generator();
        let mut rng = thread_rng();

        let pt = create_test_plaintext(&bfv_params, 1);
        let (ct1, _u1, _e0_1, _e1_1) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();
        let (ct2, _u2, _e0_2, _e1_2) = pk.try_encrypt_extended(&pt, &mut rng).unwrap();
        let sum_ct = &ct1 + &ct2;

        let bit_ct = test_bit_ct(&bfv_params);
        let inputs =
            CiphertextAdditionInputs::compute(&ct1, &ct2, &sum_ct, bfv_params.clone(), bit_ct)
                .unwrap();
        let standard_form = inputs.standard_form();

        // Verify structure is preserved.
        assert_eq!(
            standard_form.prev_ct0is.limbs.len(),
            inputs.prev_ct0is.limbs.len()
        );
    }
}
