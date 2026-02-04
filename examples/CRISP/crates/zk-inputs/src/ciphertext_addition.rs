// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_polynomial::{CrtPolynomial, Polynomial};
use e3_zk_helpers::commitments::compute_ciphertext_commitment;
use e3_zk_helpers::threshold::compute_pk_bit;
use e3_zk_helpers::utils::get_zkp_modulus;
use eyre::{Context, Result};
use fhe::bfv::BfvParameters;
use fhe::bfv::Ciphertext;
use num_bigint::BigInt;

/// Set of inputs for validation of a ciphertext addition.
///
/// This struct contains all the necessary data to prove that a ciphertext addition
/// was performed correctly in the zero-knowledge proof system.
#[derive(Clone, Debug)]
pub struct CiphertextAdditionWitness {
    pub prev_ct0is: CrtPolynomial,
    pub prev_ct1is: CrtPolynomial,
    pub sum_ct0is: CrtPolynomial,
    pub sum_ct1is: CrtPolynomial,
    pub r0is: CrtPolynomial,
    pub r1is: CrtPolynomial,
    pub prev_ct_commitment: BigInt,
}

impl CiphertextAdditionWitness {
    /// Computes the ciphertext addition inputs for zero-knowledge proof validation.
    ///
    /// # Arguments
    /// * `params` - BFV parameters
    /// * `prev_ct` - The existing ciphertext to add to
    /// * `ct` - The ciphertext being added
    /// * `sum_ct` - The result of the ciphertext addition
    ///
    /// # Returns
    /// CiphertextAdditionInputs containing all necessary proof data
    pub fn compute(
        params: &BfvParameters,
        prev_ct: &Ciphertext,
        ct: &Ciphertext,
        sum_ct: &Ciphertext,
    ) -> Result<CiphertextAdditionWitness> {
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
        //
        // We center so the quotient r = (sum − (prev + ct)) / q_i lies in {-1, 0, 1}.
        // BFV/fhe-math already gives coefficients in [0, q_i), so reduce is redundant. We need centering
        // into (-q/2, q/2]: then the difference per coefficient is small in absolute value, and for valid
        // ciphertext addition that difference is a multiple of q_i, so the quotient is in {-1, 0, 1},
        // which the circuit and compute_quotient expect.
        for c in &mut crt_polynomials {
            c.reverse();
            c.center(&moduli)?;
        }

        let [mut prev_ct0, mut prev_ct1, mut ct0, mut ct1, mut sum_ct0, mut sum_ct1] =
            crt_polynomials;

        // Compute quotient polynomials: r = (sum_centered - (ct_centered + prev_ct_centered)) / qi.
        // For ciphertext addition: sum_centered = ct_centered + prev_ct_centered + r * qi.
        // So: r = (sum_centered - (ct_centered + prev_ct_centered)) / qi.
        let mut r0 = Self::compute_quotient(&sum_ct0, &ct0, &prev_ct0, &moduli)
            .with_context(|| "Failed to compute r0 quotient")?;
        let mut r1 = Self::compute_quotient(&sum_ct1, &ct1, &prev_ct1, &moduli)
            .with_context(|| "Failed to compute r1 quotient")?;

        let zkp_modulus = &get_zkp_modulus();

        // Reduce all coefficients modulo the ZKP modulus so they lie in the proof system's
        // native field. The circuit expects witnesses in [0, zkp_modulus); unreduced values
        // would break constraint satisfaction or overflow the field representation.
        prev_ct0.reduce_uniform(zkp_modulus);
        prev_ct1.reduce_uniform(zkp_modulus);
        ct0.reduce_uniform(zkp_modulus);
        ct1.reduce_uniform(zkp_modulus);
        sum_ct0.reduce_uniform(zkp_modulus);
        sum_ct1.reduce_uniform(zkp_modulus);
        r0.reduce_uniform(zkp_modulus);
        r1.reduce_uniform(zkp_modulus);

        let pk_bit = compute_pk_bit(params)?;
        let prev_ct_commitment = compute_ciphertext_commitment(&prev_ct0, &prev_ct1, pk_bit);

        Ok(CiphertextAdditionWitness {
            prev_ct0is: prev_ct0,
            prev_ct1is: prev_ct1,
            sum_ct0is: sum_ct0,
            sum_ct1is: sum_ct1,
            r0is: r0,
            r1is: r1,
            prev_ct_commitment,
        })
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
            let (q_poly, remainder) = diff
                .div(&qi)
                .map_err(|e| eyre::eyre!("division by modulus q_i at index {}: {}", i, e))?;

            if !remainder.is_zero() {
                return Err(eyre::eyre!(
                    "Division by q_i at modulus index {} was not exact; non-zero remainder",
                    i
                ));
            }

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
