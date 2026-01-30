// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Utility functions for polynomial operations.

use crate::polynomial::PolynomialError;
use crate::Polynomial;
use num_bigint::BigInt;
use num_traits::Zero;

/// Reduces a number modulo a prime modulus and centers it.
///
/// This function takes an arbitrary number and reduces it modulo the specified prime modulus.
/// After reduction, the number is adjusted to be within the symmetric range
/// [−(modulus−1)/2, (modulus−1)/2]. If the number is already within this range, it remains unchanged.
///
/// # Arguments
///
/// * `x` - A reference to a `BigInt` representing the number to be reduced and centered.
/// * `modulus` - A reference to the prime modulus `BigInt` used for reduction.
/// * `half_modulus` - A reference to a `BigInt` representing half of the modulus used to center the coefficient.
///
/// # Returns
///
/// A `BigInt` representing the reduced and centered number.
pub fn reduce_and_center(x: &BigInt, modulus: &BigInt, half_modulus: &BigInt) -> BigInt {
    let mut r = reduce(x, modulus);

    // Adjust the remainder if it is greater than half_modulus.
    if (modulus % BigInt::from(2)) == BigInt::from(1) {
        if r > *half_modulus {
            r -= modulus;
        }
    } else if r >= *half_modulus {
        r -= modulus;
    }

    r
}

/// Reduces a number modulo a modulus.
///
/// # Arguments
///
/// * `x` - The number to reduce
/// * `modulus` - The modulus to reduce by
///
/// # Returns
///
/// The reduced number in the range [0, modulus)
pub fn reduce(x: &BigInt, modulus: &BigInt) -> BigInt {
    let mut r = x % modulus;
    if r < BigInt::zero() {
        r += modulus;
    }
    r
}

/// Reduces and centers polynomial coefficients modulo a prime modulus.
///
/// This function iterates over a mutable slice of polynomial coefficients, reducing each coefficient
/// modulo a given prime modulus and adjusting the result to be within the symmetric range
/// [−(modulus−1)/2, (modulus−1)/2].
///
/// # Arguments
///
/// * `coefficients` - A mutable slice of `BigInt` coefficients to be reduced and centered
/// * `modulus` - A prime modulus `BigInt` used for reduction and centering
///
/// # Panics
///
/// Panics if `modulus` is zero due to division by zero
pub fn reduce_and_center_coefficients_mut(coefficients: &mut [BigInt], modulus: &BigInt) {
    let half_modulus = modulus / 2;
    coefficients
        .iter_mut()
        .for_each(|x| *x = reduce_and_center(x, modulus, &half_modulus));
}

/// Reduces and centers polynomial coefficients modulo a prime modulus.
///
/// This function creates a new vector with coefficients reduced and centered modulo the given modulus.
///
/// # Arguments
///
/// * `coefficients` - A slice of `BigInt` coefficients to be reduced and centered
/// * `modulus` - A prime modulus `BigInt` used for reduction and centering
///
/// # Returns
///
/// A new `Vec<BigInt>` with reduced and centered coefficients
pub fn reduce_and_center_coefficients(coefficients: &[BigInt], modulus: &BigInt) -> Vec<BigInt> {
    let half_modulus = modulus / 2;
    coefficients
        .iter()
        .map(|x| reduce_and_center(x, modulus, &half_modulus))
        .collect()
}

/// Reduces a polynomial's coefficients within a polynomial ring defined by a cyclotomic polynomial and a modulus.
///
/// This function performs two reductions on the polynomial represented by `coefficients`:
/// 1. **Cyclotomic Reduction**: Reduces the polynomial by the cyclotomic polynomial, replacing
///    the original coefficients with the remainder after polynomial division.
/// 2. **Modulus Reduction**: Reduces the coefficients of the polynomial modulo a given modulus,
///    centering the coefficients within the range [-modulus/2, modulus/2).
///
/// # Arguments
///
/// * `coefficients` - A mutable reference to a `Vec<BigInt>` representing the coefficients of the polynomial
///   to be reduced. The coefficients should be in descending order of degree.
/// * `cyclo` - A slice of `BigInt` representing the coefficients of the cyclotomic polynomial (typically x^N + 1).
/// * `modulus` - A reference to a `BigInt` representing the modulus for the coefficient reduction. The coefficients
///   will be reduced and centered modulo this value.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `PolynomialError` if the cyclotomic reduction fails.
pub fn reduce_in_ring(
    coefficients: &mut Vec<BigInt>,
    cyclo: &[BigInt],
    modulus: &BigInt,
) -> Result<(), PolynomialError> {
    let coeffs = coefficients.clone();
    let poly = Polynomial::new(coeffs);
    let reduced = poly.reduce_by_cyclotomic(cyclo)?;
    *coefficients = reduced.coefficients;
    reduce_and_center_coefficients_mut(coefficients, modulus);
    Ok(())
}

/// Reduces each element in the given slice of `BigInt` by the modulus `p`.
///
/// This function takes a slice of `BigInt` coefficients and applies the modulus operation
/// on each element. It ensures the result is within the range `[0, p-1]` by computing
/// `r = coeff % p` and adding `p` if `r` is negative. The result is collected into a new `Vec<BigInt>`.
///
/// # Arguments
///
/// * `coefficients` - A slice of `BigInt` representing the coefficients to be reduced.
/// * `p` - A reference to a `BigInt` that represents the modulus value.
///
/// # Returns
///
/// A `Vec<BigInt>` where each element is reduced modulo `p`.
pub fn reduce_coefficients(coefficients: &[BigInt], p: &BigInt) -> Vec<BigInt> {
    coefficients.iter().map(|coeff| reduce(coeff, p)).collect()
}

/// Reduces coefficients in a 2D matrix.
///
/// # Arguments
///
/// * `coefficient_matrix` - A 2D matrix of coefficients to reduce.
/// * `p` - The modulus to reduce by.
///
/// # Returns
///
/// A new 2D matrix with reduced coefficients.
pub fn reduce_coefficients_2d(coefficient_matrix: &[Vec<BigInt>], p: &BigInt) -> Vec<Vec<BigInt>> {
    coefficient_matrix
        .iter()
        .map(|coeffs| reduce_coefficients(coeffs, p))
        .collect()
}

/// Reduces coefficients in a 3D matrix.
///
/// # Arguments
///
/// * `coefficient_matrix` - A 3D matrix of coefficients to reduce.
/// * `p` - The modulus to reduce by.
///
/// # Returns
///
/// A new 3D matrix with reduced coefficients.
pub fn reduce_coefficients_3d(
    coefficient_matrix: &[Vec<Vec<BigInt>>],
    p: &BigInt,
) -> Vec<Vec<Vec<BigInt>>> {
    coefficient_matrix
        .iter()
        .map(|coeffs| reduce_coefficients_2d(coeffs, p))
        .collect()
}

/// Mutably reduces each element in the given slice of `BigInt` by the modulus `p`.
///
/// This function modifies the given mutable slice of `BigInt` coefficients in place. It computes
/// `r = coeff % p` for each element, then adds `p` if `r` is negative, ensuring the results are
/// within the range `[0, p-1]`.
///
/// # Arguments
///
/// * `coefficients` - A mutable slice of `BigInt` representing the coefficients to be reduced.
/// * `p` - A reference to a `BigInt` that represents the modulus value.
pub fn reduce_coefficients_mut(coefficients: &mut [BigInt], p: &BigInt) {
    for coeff in coefficients.iter_mut() {
        *coeff = reduce(coeff, p);
    }
}

/// Checks if all coefficients in a vector are within a centered range.
///
/// This function verifies that every coefficient in the input vector falls within
/// the inclusive range [lower_bound, upper_bound]. This is typically used for
/// coefficients that have been centered around zero.
///
/// # Arguments
///
/// * `vec` - A slice of `BigInt` coefficients to check.
/// * `lower_bound` - The minimum allowed value (inclusive).
/// * `upper_bound` - The maximum allowed value (inclusive).
///
/// # Returns
///
/// `true` if all coefficients are within bounds, `false` otherwise.
pub fn range_check_centered(vec: &[BigInt], lower_bound: &BigInt, upper_bound: &BigInt) -> bool {
    vec.iter()
        .all(|coeff| coeff >= lower_bound && coeff <= upper_bound)
}

/// Checks if all coefficients satisfy standard range constraints with separate upper and lower bounds.
///
/// This function verifies that each coefficient falls within one of two ranges:
/// 1. [0, up_bound] (positive range)
/// 2. [modulus + low_bound, modulus) (negative range wrapped around modulus)
///
/// This is commonly used in cryptographic applications where coefficients can be
/// represented in both positive and negative forms modulo a prime.
///
/// # Mathematical Background
///
/// In modular arithmetic, negative values are often represented as their positive
/// equivalents: `-x ≡ modulus - x (mod modulus)`. This function checks both
/// the direct positive representation and the wrapped negative representation.
///
/// # Arguments
///
/// * `vec` - A slice of `BigInt` coefficients to check
/// * `low_bound` - The lower bound for the negative range (typically negative)
/// * `up_bound` - The upper bound for the positive range
/// * `modulus` - The modulus used for wraparound calculations
///
/// # Returns
///
/// `true` if all coefficients satisfy the range constraints, `false` otherwise
pub fn range_check_standard_2bounds(
    vec: &[BigInt],
    low_bound: &BigInt,
    up_bound: &BigInt,
    modulus: &BigInt,
) -> bool {
    vec.iter().all(|coeff| {
        (coeff >= &BigInt::from(0) && coeff <= up_bound)
            || (coeff >= &(modulus + low_bound) && coeff < modulus)
    })
}

/// Checks if all coefficients satisfy symmetric standard range constraints.
///
/// This function verifies that each coefficient falls within one of two symmetric ranges:
/// 1. [0, bound] (positive range)
/// 2. [modulus - bound, modulus) (negative range wrapped around modulus)
///
/// This is a special case of `range_check_standard_2bounds` where the bounds are
/// symmetric around zero. Commonly used for error distributions in cryptography.
///
/// # Mathematical Background
///
/// For a coefficient `c` and bound `b`, this function accepts:
/// - `c ∈ [0, b]` (small positive values).
/// - `c ∈ [modulus - b, modulus)` (small negative values as positive representatives).
///
/// # Arguments
///
/// * `vec` - A slice of `BigInt` coefficients to check
/// * `bound` - The symmetric bound (both positive and negative)
/// * `modulus` - The modulus used for wraparound calculations
///
/// # Returns
///
/// `true` if all coefficients satisfy the symmetric range constraints, `false` otherwise
pub fn range_check_standard(vec: &[BigInt], bound: &BigInt, modulus: &BigInt) -> bool {
    vec.iter().all(|coeff| {
        (coeff >= &BigInt::from(0) && coeff <= bound)
            || (coeff >= &(modulus - bound) && coeff < modulus)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    #[test]
    fn test_reduce_and_center() {
        let modulus = BigInt::from(7);
        let half_modulus = &modulus / 2;

        // Test positive number
        assert_eq!(
            reduce_and_center(&BigInt::from(10), &modulus, &half_modulus),
            BigInt::from(3)
        );

        // Test negative number
        assert_eq!(
            reduce_and_center(&BigInt::from(-3), &modulus, &half_modulus),
            BigInt::from(-3)
        );

        // Test number greater than half modulus
        assert_eq!(
            reduce_and_center(&BigInt::from(6), &modulus, &half_modulus),
            BigInt::from(-1)
        );
    }

    #[test]
    fn test_reduce_coefficients() {
        let coeffs = vec![BigInt::from(10), BigInt::from(-3), BigInt::from(15)];
        let modulus = BigInt::from(7);
        let result = reduce_coefficients(&coeffs, &modulus);
        assert_eq!(
            result,
            vec![BigInt::from(3), BigInt::from(4), BigInt::from(1)]
        );
    }

    #[test]
    fn test_reduce_coefficients_less_than_neg_modulus() {
        let modulus = BigInt::from(7);

        // Test with values < -p (the bug fix case)
        let coeffs = vec![
            BigInt::from(-10), // -10 % 7 = -3, -3 + 7 = 4
            BigInt::from(-14), // -14 % 7 = 0
            BigInt::from(-15), // -15 % 7 = -1, -1 + 7 = 6
            BigInt::from(-21), // -21 % 7 = 0
        ];
        let result = reduce_coefficients(&coeffs, &modulus);
        assert_eq!(
            result,
            vec![
                BigInt::from(4),
                BigInt::from(0),
                BigInt::from(6),
                BigInt::from(0)
            ]
        );

        // Test mixed positive and negative values
        let coeffs2 = vec![
            BigInt::from(-50),
            BigInt::from(-7),
            BigInt::from(-1),
            BigInt::from(0),
            BigInt::from(1),
            BigInt::from(7),
            BigInt::from(50),
        ];
        let result2 = reduce_coefficients(&coeffs2, &modulus);
        assert_eq!(
            result2,
            vec![
                BigInt::from(6), // -50 % 7 = -1, -1 + 7 = 6
                BigInt::from(0), // -7 % 7 = 0
                BigInt::from(6), // -1 % 7 = -1, -1 + 7 = 6
                BigInt::from(0),
                BigInt::from(1),
                BigInt::from(0), // 7 % 7 = 0
                BigInt::from(1), // 50 % 7 = 1
            ]
        );

        // Verify all results are in [0, modulus)
        for r in &result2 {
            assert!(*r >= BigInt::from(0), "Result {} should be >= 0", r);
            assert!(*r < modulus, "Result {} should be < {}", r, modulus);
        }
    }

    #[test]
    fn test_range_check_centered() {
        let vec = vec![BigInt::from(-2), BigInt::from(0), BigInt::from(2)];
        let lower = BigInt::from(-3);
        let upper = BigInt::from(3);
        assert!(range_check_centered(&vec, &lower, &upper));

        let vec_out_of_range = vec![BigInt::from(-5), BigInt::from(0), BigInt::from(2)];
        assert!(!range_check_centered(&vec_out_of_range, &lower, &upper));
    }

    #[test]
    fn test_range_check_standard() {
        let vec = vec![BigInt::from(1), BigInt::from(2), BigInt::from(3)];
        let bound = BigInt::from(5);
        let modulus = BigInt::from(7);
        assert!(range_check_standard(&vec, &bound, &modulus));
    }

    #[test]
    fn test_reduce_and_center_coefficients() {
        let coeffs = vec![BigInt::from(10), BigInt::from(15), BigInt::from(20)];
        let modulus = BigInt::from(7);
        let result = reduce_and_center_coefficients(&coeffs, &modulus);
        assert_eq!(
            result,
            vec![BigInt::from(3), BigInt::from(1), BigInt::from(-1)]
        );
    }

    #[test]
    fn test_reduce() {
        let x = BigInt::from(-3);
        let modulus = BigInt::from(7);
        let result = reduce(&x, &modulus);
        assert_eq!(result, BigInt::from(4));
    }

    #[test]
    fn test_reduce_less_than_neg_modulus() {
        let modulus = BigInt::from(7);

        // Test value < -p (the bug fix case)
        assert_eq!(reduce(&BigInt::from(-10), &modulus), BigInt::from(4)); // -10 % 7 = -3, -3 + 7 = 4
        assert_eq!(reduce(&BigInt::from(-14), &modulus), BigInt::from(0)); // -14 % 7 = 0
        assert_eq!(reduce(&BigInt::from(-15), &modulus), BigInt::from(6)); // -15 % 7 = -1, -1 + 7 = 6
        assert_eq!(reduce(&BigInt::from(-21), &modulus), BigInt::from(0)); // -21 % 7 = 0

        // Test exactly -p
        assert_eq!(reduce(&BigInt::from(-7), &modulus), BigInt::from(0));

        // Test values in [-p, 0)
        assert_eq!(reduce(&BigInt::from(-6), &modulus), BigInt::from(1)); // -6 % 7 = -6, -6 + 7 = 1
        assert_eq!(reduce(&BigInt::from(-1), &modulus), BigInt::from(6)); // -1 % 7 = -1, -1 + 7 = 6

        // Test positive values
        assert_eq!(reduce(&BigInt::from(0), &modulus), BigInt::from(0));
        assert_eq!(reduce(&BigInt::from(3), &modulus), BigInt::from(3));
        assert_eq!(reduce(&BigInt::from(7), &modulus), BigInt::from(0));
        assert_eq!(reduce(&BigInt::from(10), &modulus), BigInt::from(3));

        // Verify all results are in [0, modulus)
        let test_values = vec![
            BigInt::from(-100),
            BigInt::from(-50),
            BigInt::from(-7),
            BigInt::from(-1),
            BigInt::from(0),
            BigInt::from(1),
            BigInt::from(7),
            BigInt::from(50),
            BigInt::from(100),
        ];
        for val in test_values {
            let result = reduce(&val, &modulus);
            assert!(
                result >= BigInt::from(0),
                "Result {} should be >= 0",
                result
            );
            assert!(
                result < modulus,
                "Result {} should be < {}",
                result,
                modulus
            );
        }
    }

    #[test]
    fn test_reduce_in_ring() {
        // Test successful reduction
        // cyclo = [1, 0, 1] represents x^2 + 1, so n = cyclo.len() - 1 = 2
        let cyclo = vec![BigInt::from(1), BigInt::from(0), BigInt::from(1)];
        let modulus = BigInt::from(7);

        // Create coefficients: [1, 2, 3] represents x^2 + 2x + 3
        let mut coeffs = vec![BigInt::from(1), BigInt::from(2), BigInt::from(3)];

        // Reduce in ring: first reduce by cyclotomic, then reduce coefficients modulo
        let result = reduce_in_ring(&mut coeffs, &cyclo, &modulus);
        assert!(result.is_ok());

        // Verify coefficients were modified in place
        // The result should be the remainder after dividing by x^2 + 1, then reduced mod 7
        // x^2 + 2x + 3 divided by x^2 + 1 gives remainder 2x + 2
        // After right-aligning to n=2 (cyclo.len()-1 = 2): [2, 2]
        // After mod 7 and centering: [2, 2]
        assert_eq!(coeffs.len(), 2);
        // The remainder 2x + 2 = [2, 2], when right-aligned to length 2, gives [2, 2]
        assert_eq!(coeffs[0], BigInt::from(2));
        assert_eq!(coeffs[1], BigInt::from(2));
    }

    #[test]
    fn test_reduce_in_ring_error_cases() {
        // Test with zero cyclotomic polynomial
        let cyclo_zero = vec![BigInt::from(0), BigInt::from(0)];
        let modulus = BigInt::from(7);
        let mut coeffs = vec![BigInt::from(1), BigInt::from(2)];

        let result = reduce_in_ring(&mut coeffs, &cyclo_zero, &modulus);
        assert!(matches!(result, Err(PolynomialError::DivisionByZero)));

        // Test with invalid cyclotomic (zero leading coefficient)
        let cyclo_invalid = vec![BigInt::from(0), BigInt::from(1)];
        let mut coeffs2 = vec![BigInt::from(1), BigInt::from(2)];

        let result2 = reduce_in_ring(&mut coeffs2, &cyclo_invalid, &modulus);
        assert!(matches!(
            result2,
            Err(PolynomialError::InvalidPolynomial { .. })
        ));
    }

    #[test]
    fn test_reduce_in_ring_modulus_reduction() {
        // Test that coefficients are properly reduced and centered modulo
        let cyclo = vec![BigInt::from(1), BigInt::from(0), BigInt::from(1)];
        let modulus = BigInt::from(7);

        // Create coefficients with large values
        let mut coeffs = vec![BigInt::from(10), BigInt::from(15), BigInt::from(20)];

        let result = reduce_in_ring(&mut coeffs, &cyclo, &modulus);
        assert!(result.is_ok());

        // Verify coefficients are reduced and centered (within [-3, 3] for modulus 7)
        for coeff in &coeffs {
            assert!(*coeff >= BigInt::from(-3));
            assert!(*coeff <= BigInt::from(3));
        }
    }

    #[test]
    fn test_reduce_coefficients_mut() {
        let modulus = BigInt::from(7);

        // Test with values < -p (the bug fix case)
        let mut coeffs = vec![
            BigInt::from(-10), // -10 % 7 = -3, -3 + 7 = 4
            BigInt::from(-14), // -14 % 7 = 0
            BigInt::from(-15), // -15 % 7 = -1, -1 + 7 = 6
            BigInt::from(-21), // -21 % 7 = 0
        ];
        reduce_coefficients_mut(&mut coeffs, &modulus);
        assert_eq!(
            coeffs,
            vec![
                BigInt::from(4),
                BigInt::from(0),
                BigInt::from(6),
                BigInt::from(0)
            ]
        );

        // Test mixed positive and negative values
        let mut coeffs2 = vec![
            BigInt::from(-50),
            BigInt::from(-7),
            BigInt::from(-1),
            BigInt::from(0),
            BigInt::from(1),
            BigInt::from(7),
            BigInt::from(50),
        ];
        reduce_coefficients_mut(&mut coeffs2, &modulus);
        assert_eq!(
            coeffs2,
            vec![
                BigInt::from(6), // -50 % 7 = -1, -1 + 7 = 6
                BigInt::from(0), // -7 % 7 = 0
                BigInt::from(6), // -1 % 7 = -1, -1 + 7 = 6
                BigInt::from(0),
                BigInt::from(1),
                BigInt::from(0), // 7 % 7 = 0
                BigInt::from(1), // 50 % 7 = 1
            ]
        );

        // Verify all results are in [0, modulus)
        for r in &coeffs2 {
            assert!(*r >= BigInt::from(0), "Result {} should be >= 0", r);
            assert!(*r < modulus, "Result {} should be < {}", r, modulus);
        }

        // Test that it modifies in place
        let mut coeffs3 = vec![BigInt::from(-3)];
        let original_ptr = coeffs3.as_ptr();
        reduce_coefficients_mut(&mut coeffs3, &modulus);
        assert_eq!(coeffs3[0], BigInt::from(4));
        assert_eq!(coeffs3.as_ptr(), original_ptr); // Same memory location
    }
}
