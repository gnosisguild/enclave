/// Provides helper methods that perform modular poynomial arithmetic over polynomials encoded in vectors
/// of coefficients from largest degree to lowest.
use num_bigint::BigInt;
use num_traits::*;

/// Adds two polynomials represented as vectors of `BigInt` coefficients in descending order of powers.
///
/// This function aligns two polynomials of potentially different lengths and adds their coefficients.
/// It assumes that polynomials are represented from leading degree to degree zero, even if the
/// coefficient at degree zero is zero. Leading zeros are not removed to keep the order of the
/// polynomial correct, which in Greco's case is necessary so that the order can be checked.
///
/// # Arguments
///
/// * `poly1` - Coefficients of the first polynomial, from highest to lowest degree.
/// * `poly2` - Coefficients of the second polynomial, from highest to lowest degree.
///
/// # Returns
///
/// A vector of `BigInt` coefficients representing the sum of the two polynomials.
pub fn poly_add(poly1: &[BigInt], poly2: &[BigInt]) -> Vec<BigInt> {
    // Determine the new length and create extended polynomials
    let max_length = std::cmp::max(poly1.len(), poly2.len());
    let mut extended_poly1 = vec![BigInt::zero(); max_length];
    let mut extended_poly2 = vec![BigInt::zero(); max_length];

    // Copy original coefficients into extended vectors
    extended_poly1[max_length - poly1.len()..].clone_from_slice(poly1);
    extended_poly2[max_length - poly2.len()..].clone_from_slice(poly2);

    // Add the coefficients
    let mut result = vec![BigInt::zero(); max_length];
    for i in 0..max_length {
        result[i] = &extended_poly1[i] + &extended_poly2[i];
    }

    result
}

/// Negates the coefficients of a polynomial represented as a slice of `BigInt` coefficients.
///
/// This function creates a new polynomial where each coefficient is the negation of the corresponding
/// coefficient in the input polynomial.
///
/// # Arguments
///
/// * `poly` - A slice of `BigInt` representing the coefficients of the polynomial, with the highest
///   degree term at index 0 and the constant term at the end.
///
/// # Returns
///
/// A vector of `BigInt` representing the polynomial with negated coefficients, with the same degree
/// order as the input polynomial.
pub fn poly_neg(poly: &[BigInt]) -> Vec<BigInt> {
    poly.iter().map(|x| -x).collect()
}

/// Subtracts one polynomial from another, both represented as slices of `BigInt` coefficients in descending order.
///
/// This function subtracts the second polynomial (`poly2`) from the first polynomial (`poly1`). It does so
/// by first negating the coefficients of `poly2` and then adding the result to `poly1`.
///
/// # Arguments
///
/// * `poly1` - A slice of `BigInt` representing the coefficients of the first polynomial (minuend), with
///   the highest degree term at index 0 and the constant term at the end.
/// * `poly2` - A slice of `BigInt` representing the coefficients of the second polynomial (subtrahend), with
///   the highest degree term at index 0 and the constant term at the end.
///
/// # Returns
///
/// A vector of `BigInt` representing the coefficients of the resulting polynomial after subtraction.
pub fn poly_sub(poly1: &[BigInt], poly2: &[BigInt]) -> Vec<BigInt> {
    poly_add(poly1, &poly_neg(poly2))
}

/// Multiplies two polynomials represented as slices of `BigInt` coefficients naively.
///
/// Given two polynomials `poly1` and `poly2`, where each polynomial is represented by a slice of
/// coefficients, this function computes their product. The order of coefficients (ascending or
/// descending powers) should be the same for both input polynomials. The resulting polynomial is
/// returned as a vector of `BigInt` coefficients in the same order as the inputs.
///
/// # Arguments
///
/// * `poly1` - A slice of `BigInt` representing the coefficients of the first polynomial.
/// * `poly2` - A slice of `BigInt` representing the coefficients of the second polynomial.
///
/// # Returns
///
/// A vector of `BigInt` representing the coefficients of the resulting polynomial after multiplication,
/// in the same order as the input polynomials.
pub fn poly_mul(poly1: &[BigInt], poly2: &[BigInt]) -> Vec<BigInt> {
    let product_len = poly1.len() + poly2.len() - 1;
    let mut product = vec![BigInt::zero(); product_len];

    for i in 0..poly1.len() {
        for j in 0..poly2.len() {
            product[i + j] += &poly1[i] * &poly2[j];
        }
    }

    product
}

/// Divides one polynomial by another, returning the quotient and remainder, with both polynomials
/// represented by vectors of `BigInt` coefficients in descending order of powers.
///
/// Given two polynomials `dividend` and `divisor`, where each polynomial is represented by a vector
/// of coefficients in descending order of powers (i.e., the coefficient at index `i` corresponds
/// to the term of degree `n - i`, where `n` is the degree of the polynomial), this function computes
/// their quotient and remainder. The quotient and remainder are also represented in descending order
/// of powers.
///
/// # Arguments
///
/// * `dividend` - A slice of `BigInt` representing the coefficients of the dividend polynomial.
/// * `divisor` - A slice of `BigInt` representing the coefficients of the divisor polynomial. The leading
///   coefficient (highest degree term) must be non-zero.
///
/// # Returns
///
/// A tuple containing two vectors of `BigInt`:
/// * The first vector represents the quotient polynomial, with coefficients in descending order of powers.
/// * The second vector represents the remainder polynomial, also in descending order of powers.
///
/// # Panics
///
/// This function will panic if the divisor is empty or if the leading coefficient of the divisor is zero.
pub fn poly_div(dividend: &[BigInt], divisor: &[BigInt]) -> (Vec<BigInt>, Vec<BigInt>) {
    assert!(
        !divisor.is_empty() && !divisor[0].is_zero(),
        "Leading coefficient of divisor cannot be zero"
    );

    let mut quotient = vec![BigInt::zero(); dividend.len() - divisor.len() + 1];
    let mut remainder = dividend.to_vec();

    for i in 0..quotient.len() {
        let coeff = &remainder[i] / &divisor[0];
        quotient[i] = coeff.clone();

        for j in 0..divisor.len() {
            remainder[i + j] = &remainder[i + j] - &divisor[j] * &coeff;
        }
    }

    while remainder.len() > 0 && remainder[0].is_zero() {
        remainder.remove(0);
    }

    (quotient, remainder)
}

/// Multiplies each coefficient of a polynomial by a scalar.
///
/// This function takes a polynomial represented as a vector of `BigInt` coefficients and multiplies each
/// coefficient by a given scalar.
///
/// # Arguments
///
/// * `poly` - A slice of `BigInt` representing the coefficients of the polynomial, with the highest degree term
///   at index 0 and the constant term at the end.
/// * `scalar` - A `BigInt` representing the scalar by which each coefficient of the polynomial will be multiplied.
///
/// # Returns
///
/// A vector of `BigInt` representing the polynomial with each coefficient multiplied by the scalar, maintaining
/// the same order of coefficients as the input polynomial.
pub fn poly_scalar_mul(poly: &[BigInt], scalar: &BigInt) -> Vec<BigInt> {
    poly.iter().map(|coeff| coeff * scalar).collect()
}

/// Reduces the coefficients of a polynomial by dividing it with a cyclotomic polynomial
/// and updating the coefficients with the remainder.
///
/// This function performs a polynomial long division of the input polynomial (represented by
/// `coefficients`) by the given cyclotomic polynomial (represented by `cyclo`). It replaces
/// the original coefficients with the coefficients of the remainder from this division.
///
/// # Arguments
///
/// * `coefficients` - A mutable reference to a `Vec<BigInt>` containing the coefficients of
///   the polynomial to be reduced. The coefficients are in descending order of degree,
///   i.e., the first element is the coefficient of the highest degree term.
/// * `cyclo` - A slice of `BigInt` representing the coefficients of the cyclotomic polynomial.
///   The coefficients are also in descending order of degree.
///
/// # Panics
///
/// This function will panic if the remainder length exceeds the degree of the cyclotomic polynomial,
/// which would indicate an issue with the division operation.
pub fn reduce_coefficients_by_cyclo(coefficients: &mut Vec<BigInt>, cyclo: &[BigInt]) {
    // Perform polynomial long division, assuming poly_div returns (quotient, remainder)
    let (_, remainder) = poly_div(&coefficients, cyclo);

    let N = cyclo.len() - 1;
    let mut out: Vec<BigInt> = vec![BigInt::zero(); N];

    // Calculate the starting index in `out` where the remainder should be copied
    let start_idx = N - remainder.len();

    // Copy the remainder into the `out` vector starting from `start_idx`
    out[start_idx..].clone_from_slice(&remainder);

    // Resize the original `coefficients` vector to fit the result and copy the values
    coefficients.clear();
    coefficients.extend(out);
}

/// Reduces a number modulo a prime modulus and centers it.
///
/// This function takes an arbitrary number and reduces it modulo the specified prime modulus.
/// After reduction, the number is adjusted to be within the symmetric range
/// [(−(modulus−1))/2, (modulus−1)/2]. If the number is already within this range, it remains unchanged.
///
/// # Parameters
///
/// - `x`: A reference to a `BigInt` representing the number to be reduced and centered.
/// - `modulus`: A reference to the prime modulus `BigInt` used for reduction.
/// - `half_modulus`: A reference to a `BigInt` representing half of the modulus used to center the coefficient.
///
/// # Returns
///
/// - A `BigInt` representing the reduced and centered number.
pub fn reduce_and_center(x: &BigInt, modulus: &BigInt, half_modulus: &BigInt) -> BigInt {
    // Calculate the remainder ensuring it's non-negative
    let mut r = x % modulus;
    if r < BigInt::zero() {
        r += modulus;
    }

    // Adjust the remainder if it is greater than half_modulus
    if (modulus % BigInt::from(2)) == BigInt::from(1) {
        if r > *half_modulus {
            r -= modulus;
        }
    }
    else if r >= *half_modulus {                                                                                                                                 r -= modulus; 
    }

    r
}

/// Reduces and centers polynomial coefficients modulo a prime modulus.
///
/// This function iterates over a mutable slice of polynomial coefficients, reducing each coefficient
/// modulo a given prime modulus and adjusting the result to be within the symmetric range
/// [−(modulus−1)/2, (modulus−1)/2].
///
/// # Parameters
///
/// - `coefficients`: A mutable slice of `BigInt` coefficients to be reduced and centered.
/// - `modulus`: A prime modulus `BigInt` used for reduction and centering.
///
/// # Panics
///
/// - Panics if `modulus` is zero due to division by zero.
pub fn reduce_and_center_coefficients_mut(coefficients: &mut [BigInt], modulus: &BigInt) {
    let half_modulus = modulus / BigInt::from(2);
    coefficients
        .iter_mut()
        .for_each(|x| *x = reduce_and_center(x, modulus, &half_modulus));
}

pub fn reduce_and_center_coefficients(
    coefficients: &mut [BigInt],
    modulus: &BigInt,
) -> Vec<BigInt> {
    let half_modulus = modulus / BigInt::from(2);
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
pub fn reduce_in_ring(coefficients: &mut Vec<BigInt>, cyclo: &[BigInt], modulus: &BigInt) {
    reduce_coefficients_by_cyclo(coefficients, cyclo);
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

pub fn reduce_coefficients_2d(coefficient_matrix: &[Vec<BigInt>], p: &BigInt) -> Vec<Vec<BigInt>> {
    coefficient_matrix
        .iter()
        .map(|coeffs| reduce_coefficients(coeffs, p))
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
pub fn reduce_coefficients_mut(coefficients: &mut [BigInt], p: &BigInt) {
    for coeff in coefficients.iter_mut() {
        *coeff += p;
        *coeff %= p;
    }
}

pub fn range_check_centered(vec: &[BigInt], lower_bound: &BigInt, upper_bound: &BigInt) -> bool {
    vec.iter()
        .all(|coeff| coeff >= lower_bound && coeff <= upper_bound)
}

pub fn range_check_standard_2bounds(vec: &[BigInt], low_bound: &BigInt, up_bound: &BigInt, modulus: &BigInt) -> bool {
    vec.iter().all(|coeff| {
        (coeff >= &BigInt::from(0) && coeff <= up_bound)
            || (coeff >= &(modulus + low_bound) && coeff < modulus)
    })
}
pub fn range_check_standard(vec: &[BigInt], bound: &BigInt, modulus: &BigInt) -> bool {
    vec.iter().all(|coeff| {
        (coeff >= &BigInt::from(0) && coeff <= bound)
            || (coeff >= &(modulus - bound) && coeff < modulus)
    })
}
