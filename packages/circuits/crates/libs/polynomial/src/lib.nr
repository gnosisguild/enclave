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
        let shifted_bound = (2 * upper_bound) as Field;

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
