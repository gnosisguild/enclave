// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::errors::{MathError, ParityMatrixError, ParityMatrixResult};
use num_bigint::BigUint;
use num_traits::{One, Zero};

/// Compute modular exponentiation: base^exp mod modulus
pub fn mod_pow(base: &BigUint, exp: usize, modulus: &BigUint) -> BigUint {
    if modulus.is_one() {
        return BigUint::zero();
    }
    base.modpow(&BigUint::from(exp), modulus)
}

/// Compute modular inverse using extended Euclidean algorithm
/// Returns an error if inverse doesn't exist (gcd(a, modulus) != 1)
pub fn mod_inverse(a: &BigUint, modulus: &BigUint) -> ParityMatrixResult<BigUint> {
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
}
