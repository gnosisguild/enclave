// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bigint_poly::*;
use eyre::{Context, Result};
use fhe::bfv::BfvParameters;
use fhe::bfv::Ciphertext;
use fhe::bfv::Plaintext;
use fhe_math::rq::Representation;
use itertools::izip;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::Zero;
use rayon::iter::{ParallelBridge, ParallelIterator};
use shared::constants::get_zkp_modulus;
use std::sync::Arc;

/// Set of inputs for validation of a ciphertext addition.
///
/// This struct contains all the necessary data to prove that a ciphertext addition
/// was performed correctly in the zero-knowledge proof system.
#[derive(Clone, Debug)]
pub struct CiphertextAdditionInputs {
    pub prev_ct0is: Vec<Vec<BigInt>>,
    pub prev_ct1is: Vec<Vec<BigInt>>,
    pub sum_ct0is: Vec<Vec<BigInt>>,
    pub sum_ct1is: Vec<Vec<BigInt>>,
    pub r0is: Vec<Vec<BigInt>>,
    pub r1is: Vec<Vec<BigInt>>,
    pub r_bound: u64,
}

impl CiphertextAdditionInputs {
    /// Creates a new CiphertextAdditionInputs with zero-initialized vectors.
    ///
    /// # Arguments
    /// * `num_moduli` - Number of CRT moduli
    /// * `degree` - Polynomial degree
    pub fn new(num_moduli: usize, degree: usize) -> Self {
        CiphertextAdditionInputs {
            prev_ct0is: vec![vec![BigInt::zero(); degree]; num_moduli],
            prev_ct1is: vec![vec![BigInt::zero(); degree]; num_moduli],
            sum_ct0is: vec![vec![BigInt::zero(); degree]; num_moduli],
            sum_ct1is: vec![vec![BigInt::zero(); degree]; num_moduli],
            r0is: vec![vec![BigInt::zero(); degree]; num_moduli],
            r1is: vec![vec![BigInt::zero(); degree]; num_moduli],
            r_bound: 0,
        }
    }

    /// Computes the ciphertext addition inputs for zero-knowledge proof validation.
    ///
    /// # Arguments
    /// * `pt` - The plaintext being encrypted
    /// * `prev_ct` - The existing ciphertext to add to
    /// * `ct` - The ciphertext being added (from Greco)
    /// * `sum_ct` - The result of the ciphertext addition
    /// * `params` - BFV parameters
    ///
    /// # Returns
    /// CiphertextAdditionInputs containing all necessary proof data
    pub fn compute(
        pt: &Plaintext,
        prev_ct: &Ciphertext,
        ct: &Ciphertext,
        sum_ct: &Ciphertext,
        params: &BfvParameters,
    ) -> Result<CiphertextAdditionInputs> {
        let ctx: &Arc<fhe_math::rq::Context> = params
            .ctx_at_level(pt.level())
            .with_context(|| "Failed to get context at level")?;
        let n: u64 = params.degree() as u64;

        // Extract and convert ciphertexts to power basis representation.
        let mut prev_ct0 = prev_ct.c[0].clone();
        let mut prev_ct1 = prev_ct.c[1].clone();
        prev_ct0.change_representation(Representation::PowerBasis);
        prev_ct1.change_representation(Representation::PowerBasis);

        let mut ct0 = ct.c[0].clone();
        let mut ct1 = ct.c[1].clone();
        ct0.change_representation(Representation::PowerBasis);
        ct1.change_representation(Representation::PowerBasis);

        let mut sum_ct0 = sum_ct.c[0].clone();
        let mut sum_ct1 = sum_ct.c[1].clone();
        sum_ct0.change_representation(Representation::PowerBasis);
        sum_ct1.change_representation(Representation::PowerBasis);

        // Initialize matrices to store results.
        let mut res = CiphertextAdditionInputs::new(params.moduli().len(), n as usize);

        // For M=2 (adding two ciphertexts), each coefficient of the quotient polynomial
        // must be in {-1, 0, 1}, so the bound is 1 for all CRT moduli.
        let r_bound = 1u64;

        let prev_ct0_coeffs = prev_ct0.coefficients();
        let prev_ct1_coeffs = prev_ct1.coefficients();
        let ct0_coeffs = ct0.coefficients();
        let ct1_coeffs = ct1.coefficients();
        let sum_ct0_coeffs = sum_ct0.coefficients();
        let sum_ct1_coeffs = sum_ct1.coefficients();

        let prev_ct0_coeffs_rows = prev_ct0_coeffs.rows();
        let prev_ct1_coeffs_rows = prev_ct1_coeffs.rows();
        let ct0_coeffs_rows = ct0_coeffs.rows();
        let ct1_coeffs_rows = ct1_coeffs.rows();
        let sum_ct0_coeffs_rows = sum_ct0_coeffs.rows();
        let sum_ct1_coeffs_rows = sum_ct1_coeffs.rows();

        // Perform the main computation logic in parallel across moduli.
        let results: Vec<_> = izip!(
            ctx.moduli_operators(),
            prev_ct0_coeffs_rows,
            prev_ct1_coeffs_rows,
            ct0_coeffs_rows,
            ct1_coeffs_rows,
            sum_ct0_coeffs_rows,
            sum_ct1_coeffs_rows,
        )
        .enumerate()
        .par_bridge()
        .map(
            |(
                i,
                (
                    qi,
                    prev_ct0_coeffs,
                    prev_ct1_coeffs,
                    ct0_coeffs,
                    ct1_coeffs,
                    sum_ct0_coeffs,
                    sum_ct1_coeffs,
                ),
            )| {
                // Convert to vectors of BigInt, center, and reverse order.
                let mut prev_ct0i: Vec<BigInt> = prev_ct0_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut prev_ct1i: Vec<BigInt> = prev_ct1_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut ct0i: Vec<BigInt> = ct0_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut ct1i: Vec<BigInt> = ct1_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut sum_ct0i: Vec<BigInt> = sum_ct0_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut sum_ct1i: Vec<BigInt> = sum_ct1_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();

                let qi_bigint = BigInt::from(qi.modulus());

                // Center coefficients around zero for proper modular arithmetic.
                reduce_and_center_coefficients_mut(&mut prev_ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut prev_ct1i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut ct1i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut sum_ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut sum_ct1i, &qi_bigint);

                // Compute quotient polynomials: r = (sum_centered - (ct_centered + prev_ct_centered)) / qi.
                // For ciphertext addition: sum_centered = ct_centered + prev_ct_centered + r * qi.
                // So: r = (sum_centered - (ct_centered + prev_ct_centered)) / qi.
                let mut r0i = Vec::new();
                let mut r1i = Vec::new();

                // Reserve space for the quotient polynomials.
                r0i.reserve_exact(n as usize);
                r1i.reserve_exact(n as usize);

                for j in 0..n as usize {
                    let diff0 = &sum_ct0i[j] - (&ct0i[j] + &prev_ct0i[j]);
                    let (q0, r0) = diff0.div_rem(&qi_bigint);
                    if !r0.is_zero() {
                        return Err(eyre::eyre!(
                            "Non-zero remainder in ct0 division at modulus index {}, coeff {}: remainder = {}", i, j, r0
                        ));
                    }
                    if q0 < (-1).into() || q0 > 1.into() {
                        return Err(eyre::eyre!(
                            "Quotient out of range [-1, 1] for ct0 at modulus index {}, coeff {}: quotient = {}", i, j, q0
                        ));
                    }
                    let diff1 = &sum_ct1i[j] - (&ct1i[j] + &prev_ct1i[j]);
                    let (q1, r1) = diff1.div_rem(&qi_bigint);
                    if !r1.is_zero() {
                        return Err(eyre::eyre!(
                            "Non-zero remainder in ct1 division at modulus index {}, coeff {}: remainder = {}", i, j, r1
                        ));
                    }
                    if q1 < (-1).into() || q1 > 1.into() {
                        return Err(eyre::eyre!(
                            "Quotient out of range [-1, 1] for ct1 at modulus index {}, coeff {}: quotient = {}", i, j, q1
                        ));
                    }
                    r0i.push(q0);
                    r1i.push(q1);
                }

                Ok((i, prev_ct0i, prev_ct1i, sum_ct0i, sum_ct1i, r0i, r1i))
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

        // Merge results into the `res` structure after parallel execution.
        for (i, prev_ct0i, prev_ct1i, sum_ct0i, sum_ct1i, r0i, r1i) in results {
            res.prev_ct0is[i] = prev_ct0i;
            res.prev_ct1is[i] = prev_ct1i;
            res.sum_ct0is[i] = sum_ct0i;
            res.sum_ct1is[i] = sum_ct1i;
            res.r0is[i] = r0i;
            res.r1is[i] = r1i;
        }

        // Set the bound for the quotient polynomials.
        res.r_bound = r_bound;

        Ok(res)
    }

    /// Converts the inputs to standard form by reducing coefficients modulo the ZKP modulus.
    ///
    /// # Returns
    /// A new CiphertextAdditionInputs with coefficients reduced to the ZKP modulus
    pub fn standard_form(&self) -> Self {
        let zkp_modulus = &get_zkp_modulus();
        CiphertextAdditionInputs {
            prev_ct0is: reduce_coefficients_2d(&self.prev_ct0is, zkp_modulus),
            prev_ct1is: reduce_coefficients_2d(&self.prev_ct1is, zkp_modulus),
            sum_ct0is: reduce_coefficients_2d(&self.sum_ct0is, zkp_modulus),
            sum_ct1is: reduce_coefficients_2d(&self.sum_ct1is, zkp_modulus),
            r0is: reduce_coefficients_2d(&self.r0is, zkp_modulus),
            r1is: reduce_coefficients_2d(&self.r1is, zkp_modulus),
            r_bound: self.r_bound,
        }
    }
}
