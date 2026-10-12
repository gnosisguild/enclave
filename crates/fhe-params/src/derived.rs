// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Scaling, modular arithmetic, and coefficient widths derived from BFV parameters.

use fhe::bfv::BfvParameters;
use fhe_math::zq::Modulus;
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::ToPrimitive;
use thiserror::Error;

/// Errors from BFV parameter-derived arithmetic.
#[derive(Error, Debug)]
pub enum DerivedError {
    #[error("BFV error: {0}")]
    Fhe(#[from] fhe::Error),
    #[error("Sample error: {0}")]
    Sample(String),
    #[error("Unexpected error: {0}")]
    Other(String),
}

/// Return the product Q = q_0 * q_1 * ... * q_{L-1} of CRT moduli.
pub fn compute_q_product(moduli: &[u64]) -> BigUint {
    moduli.iter().map(|&m| BigUint::from(m)).product()
}

/// Return Q^{-1} mod t for BFV decoding. Fail if gcd(Q, t) != 1.
pub fn compute_q_inverse_mod_t(q: &BigUint, t: u64) -> Result<u64, DerivedError> {
    let modulus = BigUint::from(t);
    // Q=1, t=0 has division-by-zero semantics; other zero-modulus inputs return a gcd error.
    if t == 0 && q == &BigUint::from(1u32) {
        let _ = q % &modulus;
    }
    let inverse = if t == 0 { None } else { q.modinv(&modulus) };
    let inverse = inverse.ok_or_else(|| {
        DerivedError::Other(format!(
            "Q and t are not coprime, gcd = {}",
            q.gcd(&modulus)
        ))
    })?;
    Ok(inverse
        .to_u64()
        .expect("inverse below a u64 modulus must fit u64"))
}

/// Center an already-computed Q residue in [0, t) for a positive u64 plaintext modulus.
pub fn compute_q_mod_t_centered(q_mod_t: &BigUint, t: u64) -> BigInt {
    let t_bn = BigInt::from(t);
    // Preserve the raw helper's full u64 domain; zq::Modulus is limited to 62 bits.
    let half = &t_bn / 2u32;
    (BigInt::from(q_mod_t.clone()) + &half).mod_floor(&t_bn) - half
}

/// Return k0_i = (-t)^{-1} mod q_i for each CRT modulus.
pub fn compute_k0is(moduli: &[u64], plaintext_modulus: u64) -> Result<Vec<u64>, DerivedError> {
    let mut k0is = Vec::with_capacity(moduli.len());
    for &qi in moduli {
        let m = Modulus::new(qi).map_err(|e| {
            DerivedError::Sample(format!("Failed to create modulus for k0is: {:?}", e))
        })?;
        if plaintext_modulus >= qi {
            return Err(DerivedError::Other(format!(
                "plaintext modulus {plaintext_modulus} must be smaller than CRT modulus {qi}"
            )));
        }
        let neg_t = m.neg(plaintext_modulus);
        let k0qi = m.inv(neg_t).ok_or(DerivedError::Fhe(fhe::Error::MathError(
            fhe_math::Error::NonInvertible {
                value: neg_t,
                modulus: qi,
            },
        )))?;
        k0is.push(k0qi);
    }
    Ok(k0is)
}

/// Return the bit width of a bound, with a minimum of one bit.
pub fn calculate_bit_width(bound: &BigInt) -> u32 {
    if bound <= &BigInt::from(0) {
        return 1;
    }

    bound.bits() as u32
}

/// Return the bit width of centered ring coefficients.
pub fn compute_modulus_bit(params: &BfvParameters) -> u32 {
    let moduli = params.moduli();
    let bound = moduli.iter().copied().max().unwrap().saturating_sub(1) / 2;
    (u64::BITS - bound.leading_zeros()).max(1)
}

/// Return max_l bits(q_l - 1) for native CRT coefficients in [0, q_l).
pub fn compute_native_crt_coeff_bit(moduli: &[u64]) -> u32 {
    moduli
        .iter()
        .map(|&q| (u64::BITS - q.saturating_sub(1).leading_zeros()).max(1))
        .max()
        .unwrap_or(1)
}

/// Return the bit width of plaintext coefficients in [0, t).
pub fn compute_msg_bit(params: &BfvParameters) -> u32 {
    let bound = params.plaintext().saturating_sub(1);
    (u64::BITS - bound.leading_zeros()).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_product_preserves_full_precision_and_empty_product() {
        let moduli = [3u64, 5, 7];
        let q = compute_q_product(&moduli);
        assert_eq!(q, BigUint::from(105u64));
        assert_eq!(compute_q_product(&[]), BigUint::from(1u64));
        assert_eq!(
            compute_q_product(&[u64::MAX, u64::MAX]),
            BigUint::parse_bytes(b"340282366920938463426481119284349108225", 10).unwrap(),
        );
    }

    #[test]
    fn centered_q_residues_preserve_small_and_full_width_moduli() {
        for (residue, t, expected) in [
            (3, 5, BigInt::from(-2)),
            (2, 5, BigInt::from(2)),
            (3, 6, BigInt::from(-3)),
            (0, 1, BigInt::from(0)),
            (1 << 61, 1 << 62, -BigInt::from(1u64 << 61)),
            (u64::MAX / 2, u64::MAX, BigInt::from(u64::MAX / 2)),
            (u64::MAX / 2 + 1, u64::MAX, -BigInt::from(u64::MAX / 2)),
            (
                (u64::MAX - 1) / 2,
                u64::MAX - 1,
                -BigInt::from((u64::MAX - 1) / 2),
            ),
            (u64::MAX - 1, u64::MAX, BigInt::from(-1)),
        ] {
            assert_eq!(
                compute_q_mod_t_centered(&BigUint::from(residue), t),
                expected
            );
        }
        for t in [100, 0xffffee001, 17000000, 576460752317792257] {
            let modulus = Modulus::new(t).unwrap();
            for residue in [0, (t - 1) / 2, t / 2 + 1, t - 1] {
                assert_eq!(
                    compute_q_mod_t_centered(&BigUint::from(residue), t),
                    BigInt::from(modulus.center(residue)),
                );
            }
        }
    }

    #[test]
    fn compute_k0is_requires_plaintext_modulus_below_each_crt_modulus() {
        assert_eq!(compute_k0is(&[17], 5).unwrap(), vec![10]);
        assert!(compute_k0is(&[17], 17).is_err());
        assert!(compute_k0is(&[17], 18).is_err());
    }

    #[test]
    fn calculate_bit_width_handles_zero_and_positive_bounds() {
        for (bound, expected) in [(0, 1), (1, 1), (2, 2), (3, 2), (4, 3), (7, 3), (8, 4)] {
            assert_eq!(calculate_bit_width(&BigInt::from(bound)), expected);
        }
        assert_eq!(calculate_bit_width(&(BigInt::from(1) << 128usize)), 129);
    }

    #[test]
    fn calculate_bit_width_handles_negative_bounds() {
        for bound in [
            BigInt::from(-1),
            BigInt::from(-8),
            -BigInt::from(1) << 128usize,
        ] {
            assert_eq!(calculate_bit_width(&bound), 1);
        }
    }

    #[test]
    fn q_inverse_preserves_coprime_unit_and_failure_contracts() {
        for (q, t, expected) in [
            (BigUint::from(105u32), 17, 6),
            ((BigUint::from(1u32) << 128usize) + 1u32, 17, 9),
            (BigUint::from(0u32), 1, 0),
            (BigUint::from(105u32), 1, 0),
            (BigUint::from(u64::MAX - 1), u64::MAX, u64::MAX - 1),
        ] {
            assert_eq!(compute_q_inverse_mod_t(&q, t).unwrap(), expected);
        }
        for (q, t, gcd) in [(21u32, 7, 7), (0, 17, 17), (6, 0, 6), (0, 0, 0)] {
            let error = compute_q_inverse_mod_t(&BigUint::from(q), t).unwrap_err();
            assert!(matches!(error, DerivedError::Other(_)));
            assert_eq!(
                error.to_string(),
                format!("Unexpected error: Q and t are not coprime, gcd = {gcd}")
            );
        }
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn unit_q_with_zero_plaintext_modulus_keeps_division_by_zero_semantics() {
        let _ = compute_q_inverse_mod_t(&BigUint::from(1u32), 0);
    }

    #[test]
    fn coefficient_widths_preserve_power_of_two_and_raw_slice_boundaries() {
        for (t, expected_msg_width) in [(2, 1), (8, 3), (9, 4)] {
            let params = crate::build_bfv_params(8, t, &[17, 97], None);
            assert_eq!(compute_modulus_bit(&params), 6);
            assert_eq!(compute_native_crt_coeff_bit(params.moduli()), 7);
            assert_eq!(compute_msg_bit(&params), expected_msg_width);
        }
        for (moduli, expected) in [
            (vec![], 1),
            (vec![0], 1),
            (vec![1], 1),
            (vec![2], 1),
            (vec![17, 5], 5),
            (vec![u64::MAX], 64),
        ] {
            assert_eq!(compute_native_crt_coeff_bit(&moduli), expected);
        }
    }
}
