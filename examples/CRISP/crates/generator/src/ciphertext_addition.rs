// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use bigint_poly::*;
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

    pub fn compute(
        pt: &Plaintext,
        old_ct: &Ciphertext,
        new_ct: &Ciphertext,
        sum_ct: &Ciphertext,
        params: &BfvParameters,
    ) -> Result<CiphertextAdditionInputs, String> {
        let ctx: &Arc<fhe_math::rq::Context> = params
            .ctx_at_level(pt.level())
            .map_err(|e| format!("Failed to get context at level: {}", e))?;
        let n: u64 = params.degree() as u64;

        // Extract and convert ciphertexts and public key polynomials.
        let mut old_ct0 = old_ct.c[0].clone();
        let mut old_ct1 = old_ct.c[1].clone();
        old_ct0.change_representation(Representation::PowerBasis);
        old_ct1.change_representation(Representation::PowerBasis);

        let mut new_ct0 = new_ct.c[0].clone();
        let mut new_ct1 = new_ct.c[1].clone();
        new_ct0.change_representation(Representation::PowerBasis);
        new_ct1.change_representation(Representation::PowerBasis);

        let mut sum_ct0 = sum_ct.c[0].clone();
        let mut sum_ct1 = sum_ct.c[1].clone();
        sum_ct0.change_representation(Representation::PowerBasis);
        sum_ct1.change_representation(Representation::PowerBasis);

        // Initialize matrices to store results.
        let mut res = CiphertextAdditionInputs::new(params.moduli().len(), n as usize);

        // For M=2 (adding two ciphertexts), each coefficient of the quotient polynomial
        // must be in {-1, 0, 1}, so the bound is 1 for all CRT moduli
        let r_bound = 1u64;

        let old_ct0_coeffs = old_ct0.coefficients();
        let old_ct1_coeffs = old_ct1.coefficients();
        let new_ct0_coeffs = new_ct0.coefficients();
        let new_ct1_coeffs = new_ct1.coefficients();
        let sum_ct0_coeffs = sum_ct0.coefficients();
        let sum_ct1_coeffs = sum_ct1.coefficients();

        let old_ct0_coeffs_rows = old_ct0_coeffs.rows();
        let old_ct1_coeffs_rows = old_ct1_coeffs.rows();
        let new_ct0_coeffs_rows = new_ct0_coeffs.rows();
        let new_ct1_coeffs_rows = new_ct1_coeffs.rows();
        let sum_ct0_coeffs_rows = sum_ct0_coeffs.rows();
        let sum_ct1_coeffs_rows = sum_ct1_coeffs.rows();

        // Perform the main computation logic
        let results: Vec<_> = izip!(
            ctx.moduli_operators(),
            old_ct0_coeffs_rows,
            old_ct1_coeffs_rows,
            new_ct0_coeffs_rows,
            new_ct1_coeffs_rows,
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
                    old_ct0_coeffs,
                    old_ct1_coeffs,
                    new_ct0_coeffs,
                    new_ct1_coeffs,
                    sum_ct0_coeffs,
                    sum_ct1_coeffs,
                ),
            )| {
                // Convert to vectors of bigint, center, and reverse order.
                let mut old_ct0i: Vec<BigInt> = old_ct0_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut old_ct1i: Vec<BigInt> = old_ct1_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut new_ct0i: Vec<BigInt> = new_ct0_coeffs
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect();
                let mut new_ct1i: Vec<BigInt> = new_ct1_coeffs
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

                reduce_and_center_coefficients_mut(&mut old_ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut old_ct1i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut new_ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut new_ct1i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut sum_ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut sum_ct1i, &qi_bigint);

                // Compute quotient polynomials: r = (sum_centered - (new_ct_centered + old_ct_centered)) / qi
                // For ciphertext addition: sum_centered = new_ct_centered + old_ct_centered + r * qi
                // So: r = (sum_centered - (new_ct_centered + old_ct_centered)) / qi
                let mut sum_r0i = Vec::new();
                let mut sum_r1i = Vec::new();

                sum_r0i.reserve_exact(n as usize);
                sum_r1i.reserve_exact(n as usize);
                for j in 0..n as usize {
                    let diff0 = &sum_ct0i[j] - (&new_ct0i[j] + &old_ct0i[j]);
                    let (q0, r0) = diff0.div_rem(&qi_bigint);
                    if !r0.is_zero() {
                        return Err(format!(
                            "Non-zero remainder in ct0 division at modulus index {}, coeff {}: remainder = {}", i, j, r0
                        ));
                    }
                    if q0 < (-1).into() || q0 > 1.into() {
                        return Err(format!(
                            "Quotient out of range [-1, 1] for ct0 at modulus index {}, coeff {}: quotient = {}", i, j, q0
                        ));
                    }
                    let diff1 = &sum_ct1i[j] - (&new_ct1i[j] + &old_ct1i[j]);
                    let (q1, r1) = diff1.div_rem(&qi_bigint);
                    if !r1.is_zero() {
                        return Err(format!(
                            "Non-zero remainder in ct1 division at modulus index {}, coeff {}: remainder = {}", i, j, r1
                        ));
                    }
                    if q1 < (-1).into() || q1 > 1.into() {
                        return Err(format!(
                            "Quotient out of range [-1, 1] for ct1 at modulus index {}, coeff {}: quotient = {}", i, j, q1
                        ));
                    }
                    sum_r0i.push(q0);
                    sum_r1i.push(q1);
                }

                Ok((i, old_ct0i, old_ct1i, sum_ct0i, sum_ct1i, sum_r0i, sum_r1i))
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

        // Merge results into the `res` structure after parallel execution.
        for (i, old_ct0i, old_ct1i, sum_ct0i, sum_ct1i, sum_r0i, sum_r1i) in results {
            res.prev_ct0is[i] = old_ct0i;
            res.prev_ct1is[i] = old_ct1i;
            res.sum_ct0is[i] = sum_ct0i;
            res.sum_ct1is[i] = sum_ct1i;
            res.r0is[i] = sum_r0i;
            res.r1is[i] = sum_r1i;
        }

        // Set the bound for the quotient polynomials
        res.r_bound = r_bound;

        Ok(res)
    }

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
