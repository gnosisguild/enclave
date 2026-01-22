//! Utility functions for polynomial operations.

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
    // Calculate the remainder ensuring it's non-negative.
    let mut r: BigInt = x % modulus;
    if r < BigInt::zero() {
        r += modulus;
    }

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
    coefficients
        .iter_mut()
        .for_each(|x| *x = reduce_and_center(x, modulus, &(modulus / 2)));
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
    coefficients
        .iter()
        .map(|x| reduce_and_center(x, modulus, &(modulus / 2)))
        .collect()
}

/// Reduces and centers a scalar value.
///
/// # Arguments
///
/// * `x` - The scalar value to reduce and center
/// * `modulus` - The modulus to reduce by
///
/// # Returns
///
/// The reduced and centered scalar value
pub fn reduce_and_center_scalar(x: &BigInt, modulus: &BigInt) -> BigInt {
    reduce_and_center(x, modulus, &(modulus / 2))
}

/// Reduces a scalar value modulo a modulus.
///
/// # Arguments
///
/// * `x` - The scalar value to reduce
/// * `modulus` - The modulus to reduce by
///
/// # Returns
///
/// The reduced scalar value in the range [0, modulus)
pub fn reduce_scalar(x: &BigInt, modulus: &BigInt) -> BigInt {
    (x + modulus) % modulus
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
pub fn reduce_in_ring(coefficients: &mut Vec<BigInt>, cyclo: &[BigInt], modulus: &BigInt) {
    let poly = Polynomial::new(coefficients.clone());
    let reduced = poly
        .reduce_by_cyclotomic(cyclo)
        .expect("Failed to reduce by cyclotomic");
    *coefficients = reduced.coefficients;
    reduce_and_center_coefficients_mut(coefficients, modulus);
}

/// Reduces each element in the given slice of `BigInt` by the modulus `p`.
///
/// This function takes a slice of `BigInt` coefficients and applies the modulus operation
/// on each element. It ensures the result is within the range `[0, p-1]` by adding `p`
/// before applying the modulus operation. The result is collected into a new `Vec<BigInt>`.
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
    coefficients.iter().map(|coeff| (coeff + p) % p).collect()
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
/// This function modifies the given mutable slice of `BigInt` coefficients in place. It adds `p`
/// to each element before applying the modulus operation, ensuring the results are within the range `[0, p-1]`.
///
/// # Arguments
///
/// * `coefficients` - A mutable slice of `BigInt` representing the coefficients to be reduced.
/// * `p` - A reference to a `BigInt` that represents the modulus value.
///
/// # Returns
///
/// A new `Vec<BigInt>` with reduced coefficients.
pub fn reduce_coefficients_mut(coefficients: &mut [BigInt], p: &BigInt) {
    for coeff in coefficients.iter_mut() {
        *coeff += p;
        *coeff %= p;
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
    fn test_reduce_scalar() {
        let x = BigInt::from(-3);
        let modulus = BigInt::from(7);
        let result = reduce_scalar(&x, &modulus);
        assert_eq!(result, BigInt::from(4));
    }

    #[test]
    fn test_reduce_and_center_scalar() {
        let x = BigInt::from(6);
        let modulus = BigInt::from(7);
        let result = reduce_and_center_scalar(&x, &modulus);
        assert_eq!(result, BigInt::from(-1));
    }
}
