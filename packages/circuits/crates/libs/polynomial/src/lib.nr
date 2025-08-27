// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// Polynomial structure representing a polynomial of degree N-1.
///
/// A polynomial P(X) = a_{N-1} * X^{N-1} + a_{N-2} * X^{N-2} + ... + a_1 * X + a_0
/// is represented as an array of coefficients where coefficients[0] = a_{N-1} (highest degree)
/// and coefficients[N-1] = a_0 (constant term).
pub struct Polynomial<let N: u32> {
    /// Array of polynomial coefficients in descending degree order
    /// coefficients[0] = coefficient of X^{N-1} (highest degree term)
    /// coefficients[N-1] = constant term (degree 0)
    pub coefficients: [Field; N],
}

impl<let N: u32> Polynomial<N> {
    /// Creates a new polynomial from an array of coefficients.
    ///
    /// # Arguments
    /// * `coefficients` - Array of N coefficients in descending degree order
    ///                    coefficients[0] = coefficient of X^{N-1}
    ///                    coefficients[N-1] = constant term
    ///
    /// # Returns
    /// A new Polynomial instance with the specified coefficients
    pub fn new(coefficients: [Field; N]) -> Self {
        Polynomial { coefficients }
    }

    /// Evaluates the polynomial at a given point using Horner's method.
    ///
    /// Horner's method computes P(x) = a_{N-1} * x^{N-1} + ... + a_1 * x + a_0
    /// as ((...((a_{N-1} * x + a_{N-2}) * x + a_{N-3}) * x + ...) * x + a_0)
    /// This approach require n multiplications and n additions to evaluate the polynomial.
    ///
    /// # Arguments
    /// * `x` - The point at which to evaluate the polynomial.
    ///
    /// # Returns
    /// The value of the polynomial at point x: P(x).
    pub fn eval(self, x: Field) -> Field {
        let mut result = self.coefficients[0];

        for i in 1..self.coefficients.len() {
            result = result * x + self.coefficients[i];
        }

        result
    }

    /// Performs range checking on polynomial coefficients using a symmetric bound.
    ///
    /// This function constrains all polynomial coefficients to be in the range [-upper_bound, upper_bound].
    /// The implementation uses a clever shifting technique to avoid negative numbers in the range check:
    /// 1. Shifts each coefficient by adding upper_bound: c' = c + upper_bound
    /// 2. Checks that shifted coefficients are in [0, 2*upper_bound]
    /// 3. This ensures original coefficients are in [-upper_bound, upper_bound]
    ///
    /// # Arguments
    /// * `upper_bound` - The symmetric bound for coefficient range checking
    ///                   Coefficients must satisfy: -upper_bound <= c <= upper_bound
    pub fn range_check_1bound(self, upper_bound: u64) {
        let shifted_bound = (upper_bound as Field) + (upper_bound as Field);

        for i in 0..self.coefficients.len() {
            let shifted_coefficient = self.coefficients[i] + upper_bound as Field;

            shifted_coefficient.assert_max_bit_size::<66>();
            (shifted_bound - shifted_coefficient).assert_max_bit_size::<66>();
        }
    }

    /// Performs range checking on polynomial coefficients using asymmetric bounds
    ///
    /// This function constrains all polynomial coefficients to be in the range [lower_bound, upper_bound].
    /// Similar to range_check_1bound, it uses a shifting technique to avoid negative numbers:
    /// 1. Shifts each coefficient by adding -lower_bound: c' = c - lower_bound
    /// 2. Checks that shifted coefficients are in [0, upper_bound - lower_bound]
    /// 3. This ensures original coefficients are in [lower_bound, upper_bound]
    ///
    /// # Arguments
    /// * `upper_bound` - The upper bound for coefficient range checking
    /// * `lower_bound` - The lower bound for coefficient range checking (can be negative)
    ///                   Coefficients must satisfy: lower_bound <= c <= upper_bound
    pub fn range_check_2bounds(self, upper_bound: u64, lower_bound: i64) {
        let shifted_amount = (lower_bound * -1) as u64 as Field;
        let range_size = shifted_amount + upper_bound as Field;

        for i in 0..self.coefficients.len() {
            let shifted_coefficient = self.coefficients[i] + shifted_amount;

            shifted_coefficient.assert_max_bit_size::<66>();
            (range_size - shifted_coefficient).assert_max_bit_size::<66>();
        }
    }
}

/// Flattens an array of polynomials into a 1D array of Field elements.
///
/// This helper function serializes polynomial coefficients maintaining a deterministic
/// ordering of coefficients to ensure reproducible challenge values.
///
/// # Generic Parameters
/// * `A` - The degree of each polynomial in the array
/// * `L` - The number of polynomials in the array
/// * `SIZE` - The total size of the target array
///
/// # Arguments
/// * `inputs` - The target array to store flattened coefficients
/// * `poly` - Array of polynomials to flatten
/// * `offset` - Starting position in the inputs array
///
/// # Returns
/// A tuple containing:
/// * The updated inputs array with flattened coefficients
/// * The new offset position after flattening
pub fn flatten<let A: u32, let L: u32, let SIZE: u32>(
    mut inputs: [Field; SIZE],
    poly: [Polynomial<A>; L],
    mut offset: u32,
) -> ([Field; SIZE], u32) {
    for j in 0..L {
        for i in 0..A {
            inputs[i + offset] = poly[j].coefficients[i];
        }
        offset += A;
    }

    (inputs, offset)
}

#[test]
fn test_polynomial_eval() {
    let coeffs = [1, 2, 3]; // represents 1x^2 + 2x + 3
    let poly = Polynomial::new(coeffs);

    let x = 2; // evaluate at x = 2
    let result = poly.eval(x);

    // (1 * 2^2) + (2 * 2) + 3 = 4 + 4 + 3 = 11
    assert_eq(result, 11);
}

#[test]
fn test_polynomial_eval_zero() {
    let coeffs = [1, -2, 1]; // x^2 - 2x + 1 = (x-1)^2
    let poly = Polynomial::new(coeffs);

    let x = 1; // evaluate at x = 1, should be 0
    let result = poly.eval(x);

    assert_eq(result, 0);
}

#[test]
fn test_polynomial_bounds() {
    let coeffs = [-16, 240, 242];
    let poly = Polynomial::new(coeffs);

    // Test single bound check - constrains to [-242, 242]
    poly.range_check_1bound(242);

    // Test double bounds check - constrains to [-16, 242]
    poly.range_check_2bounds(242, -16);
}

#[test(should_fail_with = "assert_max_bit_size")]
fn test_polynomial_out_of_bounds_coefficients() {
    let coeffs = [-100];
    let poly = Polynomial::new(coeffs);

    // Test single bound check - constrains to [-99, 99]
    // Should fail because -100 is out of bounds.
    poly.range_check_1bound(99);

    // Test double bounds check - constrains to [-9, 99]
    // Should fail because -100 is out of bounds.
    poly.range_check_2bounds(99, -9);
}

#[test]
fn test_flatten() {
    // Create test polynomials
    let poly1 = Polynomial::new([1, 2, 3]); // degree 2
    let poly2 = Polynomial::new([4, 5, 6]); // degree 2
    let poly3 = Polynomial::new([7, 8, 9]); // degree 2

    let polynomials = [poly1, poly2, poly3];

    // Initialize target array with zeros
    let mut inputs = [0; 10]; // Size to accommodate 3 polynomials of degree 2 + offset
    let offset = 1; // Start at position 1

    // Flatten the polynomials
    let (result, new_offset): ([Field; 10], u32) = flatten(inputs, polynomials, offset);

    // Verify the flattened coefficients are in the correct positions
    // Expected: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    //           ^     ^^^^^^^^  ^^^^^^^^  ^^^^^^^^
    //           |     poly1     poly2     poly3
    //           offset

    assert_eq(result[0], 0); // Should remain unchanged (before offset)
    assert_eq(result[1], 1); // poly1[0]
    assert_eq(result[2], 2); // poly1[1]
    assert_eq(result[3], 3); // poly1[2]
    assert_eq(result[4], 4); // poly2[0]
    assert_eq(result[5], 5); // poly2[1]
    assert_eq(result[6], 6); // poly2[2]
    assert_eq(result[7], 7); // poly3[0]
    assert_eq(result[8], 8); // poly3[1]
    assert_eq(result[9], 9); // poly3[2]

    // Verify the new offset is correct (original offset + total coefficients)
    assert_eq(new_offset, offset + 3 * 3); // offset + num_polynomials * degree
}
