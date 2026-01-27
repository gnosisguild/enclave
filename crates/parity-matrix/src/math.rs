// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Modular arithmetic operations for finite field computations.
//!
//! This module provides efficient implementations of modular exponentiation,
//! polynomial evaluation, and modular inverses over `Z_q`.

use crate::errors::{MathError, ParityMatrixError, ParityMatrixResult};
use num_bigint::BigUint;
use num_traits::{One, Zero};

/// Compute modular exponentiation: `base^exp mod modulus`.
///
/// # Example
///
/// ```
/// use parity_matrix::math::mod_pow;
/// use num_bigint::BigUint;
///
/// let result = mod_pow(&BigUint::from(3u32), 4, &BigUint::from(11u32));
/// assert_eq!(result, BigUint::from(4u32)); // 3^4 = 81 ≡ 4 (mod 11)
/// ```
pub fn mod_pow(base: &BigUint, exp: usize, modulus: &BigUint) -> BigUint {
    if modulus.is_one() {
        return BigUint::zero();
    }
    base.modpow(&BigUint::from(exp), modulus)
}

/// Evaluate a polynomial with given coefficients at points `0, 1, ..., n`.
///
/// For a polynomial `f(x) = a₀ + a₁x + ... + aₖxᵏ` with coefficients `coeffs = [a₀, ..., aₖ]`,
/// returns the vector `[f(0), f(1), ..., f(n)]` where all arithmetic is performed modulo `q`.
///
/// # Example
///
/// ```
/// use parity_matrix::math::evaluate_polynomial;
/// use num_bigint::BigUint;
///
/// // Evaluate f(x) = 2x + 1 at points 0, 1, 2, 3
/// let coeffs = vec![BigUint::from(1u32), BigUint::from(2u32)];
/// let result = evaluate_polynomial(&coeffs, 3, &BigUint::from(7u32));
/// assert_eq!(result[0], BigUint::from(1u32)); // f(0) = 1
/// assert_eq!(result[1], BigUint::from(3u32)); // f(1) = 3
/// ```
pub fn evaluate_polynomial(coeffs: &[BigUint], n: usize, q: &BigUint) -> Vec<BigUint> {
    let mut eval_vec = vec![BigUint::zero(); n + 1];
    #[allow(clippy::needless_range_loop)]
    for j in 0..=n {
        let x = BigUint::from(j);
        let mut val = BigUint::zero();
        for (i, coeff) in coeffs.iter().enumerate() {
            val = (val + coeff * mod_pow(&x, i, q)) % q;
        }
        eval_vec[j] = val;
    }
    eval_vec
}

/// Compute modular inverse using extended Euclidean algorithm.
///
/// Returns `a^{-1} mod modulus` such that `a · a^{-1} ≡ 1 (mod modulus)`.
///
/// # Errors
///
/// Returns an error if:
/// - `modulus = 0` or `modulus = 1`
/// - `gcd(a, modulus) ≠ 1` (inverse doesn't exist)
///
/// # Example
///
/// ```
/// use parity_matrix::math::mod_inverse;
/// use num_bigint::BigUint;
///
/// let inv = mod_inverse(&BigUint::from(3u32), &BigUint::from(11u32))?;
/// assert_eq!((BigUint::from(3u32) * &inv) % BigUint::from(11u32), BigUint::from(1u32));
/// # Ok::<(), parity_matrix::errors::ParityMatrixError>(())
/// ```
pub fn mod_inverse(a: &BigUint, modulus: &BigUint) -> ParityMatrixResult<BigUint> {
    if modulus.is_zero() {
        return Err(ParityMatrixError::from(MathError::InvalidModulus {
            modulus: modulus.to_string(),
            reason: "modulus cannot be zero".to_string(),
        }));
    }
    if modulus.is_one() {
        return Err(ParityMatrixError::from(MathError::InvalidModulus {
            modulus: modulus.to_string(),
            reason: "modular inverse is undefined for modulus = 1".to_string(),
        }));
    }
    let a_reduced = a % modulus;
    if a_reduced.is_zero() {
        return Err(ParityMatrixError::from(MathError::NoModularInverse {
            a: a.to_string(),
            modulus: modulus.to_string(),
        }));
    }

    // Extended Euclidean algorithm using signed arithmetic
    // We work with (coefficient, value) pairs
    let m = modulus.clone();

    let mut old_r = a_reduced;
    let mut r = m.clone();
    let mut old_s: (bool, BigUint) = (true, BigUint::one()); // (is_positive, abs_value)
    let mut s: (bool, BigUint) = (true, BigUint::zero());

    while !r.is_zero() {
        let quotient = &old_r / &r;

        let temp_r = r.clone();
        r = old_r - &quotient * &r;
        old_r = temp_r;

        // s = old_s - quotient * s
        // Handle signed arithmetic manually
        let prod = &quotient * &s.1;
        let new_s = if old_s.0 == s.0 {
            // Same sign: old_s - quotient * s
            if old_s.1 >= prod {
                (old_s.0, &old_s.1 - &prod)
            } else {
                (!old_s.0, &prod - &old_s.1)
            }
        } else {
            // Different signs: old_s - quotient * s = old_s + |quotient * s| or old_s - |quotient * s|
            if s.0 {
                // s is positive, so -quotient * s is negative
                // old_s (negative) - positive = old_s + |prod|
                (old_s.0, &old_s.1 + &prod)
            } else {
                // s is negative, so -quotient * s is positive
                // old_s (positive) + prod
                (old_s.0, &old_s.1 + &prod)
            }
        };

        old_s = s;
        s = new_s;
    }

    if !old_r.is_one() {
        return Err(ParityMatrixError::from(MathError::NoModularInverse {
            a: a.to_string(),
            modulus: modulus.to_string(),
        }));
    }

    // Convert signed result to positive mod m
    if old_s.0 {
        Ok(old_s.1 % &m)
    } else {
        // Negative: m - abs_value
        let abs_mod = &old_s.1 % &m;
        if abs_mod.is_zero() {
            Ok(BigUint::zero())
        } else {
            Ok(&m - abs_mod)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // ==================== Verify mod_pow and mod_inverse ====================

    #[test]
    fn test_mod_pow() {
        let q7 = BigUint::from(7u32);
        let q11 = BigUint::from(11u32);
        let q1024 = BigUint::from(1024u32);

        assert_eq!(mod_pow(&BigUint::from(2u32), 3, &q7), BigUint::from(1u32)); // 8 mod 7 = 1
        assert_eq!(mod_pow(&BigUint::from(3u32), 4, &q11), BigUint::from(4u32)); // 81 mod 11 = 4
        assert_eq!(
            mod_pow(&BigUint::from(5u32), 0, &BigUint::from(13u32)),
            BigUint::one()
        ); // x^0 = 1
        assert_eq!(mod_pow(&BigUint::zero(), 5, &q7), BigUint::zero()); // 0^n = 0
        assert_eq!(mod_pow(&BigUint::from(2u32), 10, &q1024), BigUint::zero()); // 1024 mod 1024 = 0
    }

    #[test]
    fn test_mod_inverse() {
        // Test: a * a^{-1} = 1 mod q
        for q_val in [3u32, 5, 7, 11, 13, 17, 23] {
            let q = BigUint::from(q_val);
            for a_val in 1..q_val {
                let a = BigUint::from(a_val);
                let inv = mod_inverse(&a, &q).expect("Inverse should exist for prime q");
                assert_eq!(
                    (&a * &inv) % &q,
                    BigUint::one(),
                    "a * a^-1 should be 1 mod q"
                );
            }
        }
    }

    #[test]
    fn test_mod_inverse_large() {
        // Test with a large prime
        let q = BigUint::from_str("170141183460469231731687303715884105727").unwrap();
        let a = BigUint::from(12345678901234567890u64);
        let inv = mod_inverse(&a, &q).expect("Inverse should exist");
        assert_eq!((&a * &inv) % &q, BigUint::one());
    }

    #[test]
    fn test_mod_inverse_zero_modulus() {
        // Test that modulus = 0 returns an error
        let a = BigUint::from(5u32);
        let q = BigUint::zero();
        let result = mod_inverse(&a, &q);
        assert!(result.is_err());
        match result {
            Err(ParityMatrixError::Math { message }) => {
                assert!(message.contains("modulus cannot be zero"));
            }
            _ => panic!("Expected Math error for zero modulus"),
        }
    }

    #[test]
    fn test_mod_inverse_one_modulus() {
        // Test that modulus = 1 returns an error
        let a = BigUint::from(5u32);
        let q = BigUint::one();
        let result = mod_inverse(&a, &q);
        assert!(result.is_err());
        match result {
            Err(ParityMatrixError::Math { message }) => {
                assert!(message.contains("modular inverse is undefined for modulus = 1"));
            }
            _ => panic!("Expected Math error for modulus = 1"),
        }
    }

    #[test]
    fn test_mod_inverse_zero_element() {
        // Zero has no modular inverse
        let q = BigUint::from(7u32);
        let a = BigUint::zero();
        let result = mod_inverse(&a, &q);
        assert!(result.is_err());
        match result {
            Err(ParityMatrixError::Math { message }) => {
                assert!(message.contains("Modular inverse does not exist"));
            }
            _ => panic!("Expected Math error for zero"),
        }
    }

    #[test]
    fn test_mod_inverse_non_invertible_composite_modulus() {
        // Test with composite modulus where gcd(a, q) != 1
        let q = BigUint::from(6u32); // composite: 2 * 3
        let a = BigUint::from(2u32); // gcd(2, 6) = 2 != 1
        let result = mod_inverse(&a, &q);
        assert!(result.is_err());

        let a2 = BigUint::from(3u32); // gcd(3, 6) = 3 != 1
        let result2 = mod_inverse(&a2, &q);
        assert!(result2.is_err());

        // But gcd(5, 6) = 1, so inverse should exist
        let a3 = BigUint::from(5u32);
        let result3 = mod_inverse(&a3, &q);
        assert!(result3.is_ok());
        let inv3 = result3.unwrap();
        assert_eq!((&a3 * &inv3) % &q, BigUint::one());
    }

    #[test]
    fn test_mod_inverse_multiple_of_modulus() {
        // a is a multiple of q, so gcd(a, q) = q != 1
        let q = BigUint::from(7u32);
        let a = BigUint::from(14u32); // 14 = 2 * 7
        let result = mod_inverse(&a, &q);
        assert!(result.is_err());

        let a2 = BigUint::from(21u32); // 21 = 3 * 7
        let result2 = mod_inverse(&a2, &q);
        assert!(result2.is_err());
    }

    #[test]
    fn test_mod_inverse_large_composite_modulus() {
        // Test with larger composite modulus
        let q = BigUint::from(15u32); // composite: 3 * 5
        // Elements that share factors with q should fail
        let non_invertible = vec![3u32, 5, 6, 9, 10, 12];
        for val in non_invertible {
            let a = BigUint::from(val);
            let result = mod_inverse(&a, &q);
            assert!(result.is_err(), "{} should not have inverse mod {}", val, q);
        }

        // Elements coprime with q should succeed
        let invertible = vec![1u32, 2, 4, 7, 8, 11, 13, 14];
        for val in invertible {
            let a = BigUint::from(val);
            let result = mod_inverse(&a, &q);
            assert!(result.is_ok(), "{} should have inverse mod {}", val, q);
            let inv = result.unwrap();
            assert_eq!((&a * &inv) % &q, BigUint::one());
        }
    }

    #[test]
    fn test_mod_inverse_reduced_mod_q() {
        // Test that a mod q is used (not the full value of a)
        let q = BigUint::from(7u32);
        let a = BigUint::from(9u32); // 9 mod 7 = 2
        let inv = mod_inverse(&a, &q).unwrap();
        // Should be same as inverse of 2
        let inv2 = mod_inverse(&BigUint::from(2u32), &q).unwrap();
        assert_eq!(inv, inv2);
        assert_eq!((&a * &inv) % &q, BigUint::one());
    }

    #[test]
    fn test_mod_inverse_all_elements_prime_modulus() {
        // For prime modulus, all non-zero elements should have inverses
        let q = BigUint::from(11u32);
        for a_val in 1u32..11 {
            let a = BigUint::from(a_val);
            let inv = mod_inverse(&a, &q).unwrap();
            assert_eq!((&a * &inv) % &q, BigUint::one());
        }
    }

    #[test]
    fn test_evaluate_polynomial() {
        let q = BigUint::from(7u32);
        
        // Test constant polynomial: f(x) = 3
        let coeffs = vec![BigUint::from(3u32)];
        let result = evaluate_polynomial(&coeffs, 5, &q);
        assert_eq!(result.len(), 6); // n+1 = 6
        assert!(result.iter().all(|x| *x == BigUint::from(3u32)));
        
        // Test linear polynomial: f(x) = 2x + 1
        let coeffs = vec![BigUint::from(1u32), BigUint::from(2u32)];
        let result = evaluate_polynomial(&coeffs, 3, &q);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], BigUint::from(1u32)); // f(0) = 1
        assert_eq!(result[1], BigUint::from(3u32)); // f(1) = 2*1 + 1 = 3
        assert_eq!(result[2], BigUint::from(5u32)); // f(2) = 2*2 + 1 = 5
        assert_eq!(result[3], BigUint::from(0u32)); // f(3) = 2*3 + 1 = 7 mod 7 = 0
        
        // Test quadratic polynomial: f(x) = x^2 + x + 1
        let coeffs = vec![BigUint::from(1u32), BigUint::from(1u32), BigUint::from(1u32)];
        let result = evaluate_polynomial(&coeffs, 2, &q);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], BigUint::from(1u32)); // f(0) = 1
        assert_eq!(result[1], BigUint::from(3u32)); // f(1) = 1 + 1 + 1 = 3
        assert_eq!(result[2], BigUint::from(7u32) % &q); // f(2) = 4 + 2 + 1 = 7 mod 7 = 0
        
        // Test with larger modulus
        let q = BigUint::from(101u32);
        let coeffs = vec![BigUint::from(5u32), BigUint::from(3u32), BigUint::from(2u32)];
        let result = evaluate_polynomial(&coeffs, 10, &q);
        assert_eq!(result.len(), 11);
        // Verify f(0) = 5
        assert_eq!(result[0], BigUint::from(5u32));
        // Verify f(1) = 5 + 3 + 2 = 10
        assert_eq!(result[1], BigUint::from(10u32));
    }
}
