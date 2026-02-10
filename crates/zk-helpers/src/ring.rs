// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Polynomial ring helpers for ZK circuit computations (R_qi = Z_q_i[x] / (x^N + 1)).
//!
//! Cyclotomic polynomial and residue decomposition used by pk_generation, share_encryption,
//! and user_data_encryption when reducing by R_qi and decomposing residues.

use e3_polynomial::Polynomial;
use num_bigint::BigInt;
use num_traits::Zero;

/// Returns the coefficient vector for the cyclotomic polynomial x^N + 1 (degree N).
///
/// Used by share_encryption, user_data_encryption, and pk_generation when reducing by R_qi.
#[must_use]
pub fn cyclotomic_polynomial(n: u64) -> Vec<BigInt> {
    let mut cyclo = vec![BigInt::from(0u64); (n + 1) as usize];
    cyclo[0] = BigInt::from(1u64);
    cyclo[n as usize] = BigInt::from(1u64);
    cyclo
}

/// Decomposes the residue `xi - xi_hat` into `r1 * qi + r2 * cyclo` mod R_qi.
///
/// Verifies that `xi == xi_hat (mod R_qi)`, then computes quotient polynomials
/// such that `xi - xi_hat = r1 * qi + r2 * cyclo` (with cyclo = x^N + 1).
/// Returns `(r1, r2)` as polynomials. Panics on assertion failures (exact division,
/// degree checks, reconstruction).
pub fn decompose_residue(
    xi: &Polynomial,
    xi_hat: &Polynomial,
    qi_bigint: &BigInt,
    cyclo: &[BigInt],
    n: u64,
) -> (Polynomial, Polynomial) {
    let cyclo_poly = Polynomial::new(cyclo.to_vec());
    let qi_poly = Polynomial::new(vec![qi_bigint.clone()]);

    let mut xi_hat_mod_rqi = xi_hat.clone();
    xi_hat_mod_rqi = xi_hat_mod_rqi.reduce_by_cyclotomic(cyclo).unwrap();
    xi_hat_mod_rqi.reduce(qi_bigint);
    xi_hat_mod_rqi.center(qi_bigint);
    assert_eq!(xi, &xi_hat_mod_rqi);

    let num_coeffs = xi.sub(xi_hat).coefficients().to_vec();
    assert_eq!((num_coeffs.len() as u64) - 1, 2 * (n - 1));

    let mut num_mod_zqi = Polynomial::new(num_coeffs.clone());
    num_mod_zqi.reduce(qi_bigint);
    num_mod_zqi.center(qi_bigint);

    let (r2_poly, r2_rem_poly) = num_mod_zqi.clone().div(&cyclo_poly).unwrap();
    assert!(r2_rem_poly.coefficients().iter().all(|c| c.is_zero()));
    assert_eq!((r2_poly.coefficients().len() as u64) - 1, n - 2);

    let r2_times_cyclo = r2_poly.mul(&cyclo_poly);
    let mut r2_times_cyclo_mod = r2_times_cyclo.clone();
    r2_times_cyclo_mod.reduce(qi_bigint);
    r2_times_cyclo_mod.center(qi_bigint);
    assert_eq!(&num_mod_zqi, &r2_times_cyclo_mod);
    assert_eq!(
        (r2_times_cyclo.coefficients().len() as u64) - 1,
        2 * (n - 1)
    );

    let num_poly = Polynomial::new(num_coeffs);
    let r1_num = num_poly.sub(&r2_times_cyclo);
    assert_eq!((r1_num.coefficients().len() as u64) - 1, 2 * (n - 1));

    let (r1_poly, r1_rem_poly) = r1_num.div(&qi_poly).unwrap();
    assert!(r1_rem_poly.coefficients().iter().all(|c| c.is_zero()));
    assert_eq!((r1_poly.coefficients().len() as u64) - 1, 2 * (n - 1));
    assert_eq!(&r1_num, &r1_poly.mul(&qi_poly));

    let r1_times_qi = r1_poly.clone().scalar_mul(qi_bigint);
    let xi_calculated = xi_hat
        .clone()
        .add(&r1_times_qi)
        .add(&r2_times_cyclo)
        .trim_leading_zeros();
    assert_eq!(xi, &xi_calculated);

    (r1_poly, r2_poly)
}
