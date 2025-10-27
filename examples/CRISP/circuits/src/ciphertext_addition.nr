// SPDX-License-Identifier: LGPL-3.0-only
//
// This file contains ciphertext addition verification for BFV ciphertexts.

use polynomial::Polynomial;

/// Parameters for ciphertext addition verification.
///
/// # Arguments
/// * `N` - Polynomial degree.
/// * `L` - Number of CRT bases.
pub struct CiphertextAddition<let N: u32, let L: u32> {
    pub prev_ct0is: [Polynomial<N>; L],
    pub prev_ct1is: [Polynomial<N>; L],
    sum_ct0is: [Polynomial<N>; L],
    sum_ct1is: [Polynomial<N>; L],
    r0is: [Polynomial<N>; L],
    r1is: [Polynomial<N>; L],
    pub r_bound: u64,
}

impl<let N: u32, let L: u32> CiphertextAddition<N, L> {
    /// Creates a new CiphertextAddition instance.
    ///
    /// # Arguments
    /// * `prev_ct0is`, `prev_ct1is` - Previous ciphertext polynomials.
    /// * `sum_ct0is`, `sum_ct1is` - Ciphertext addition polynomials.
    /// * `r0is`, `r1is` - Randomness polynomials.
    /// * `r_bound` - Range bound for the randomness polynomials.
    pub fn new(
        prev_ct0is: [Polynomial<N>; L],
        prev_ct1is: [Polynomial<N>; L],
        sum_ct0is: [Polynomial<N>; L],
        sum_ct1is: [Polynomial<N>; L],
        r0is: [Polynomial<N>; L],
        r1is: [Polynomial<N>; L],
        r_bound: u64,
    ) -> Self {
        Self { prev_ct0is, prev_ct1is, sum_ct0is, sum_ct1is, r0is, r1is, r_bound }
    }

    /// Verifies ciphertext addition constraints using Fiat-Shamir challenges.
    ///
    /// # Arguments
    /// * `ct0is`, `ct1is` - Ciphertext pairs to be added.
    /// * `qis` - CRT moduli
    /// * `gammas` - Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}]
    pub fn check_ciphertext_addition_constraints(
        self: CiphertextAddition<N, L>,
        ct0is: [Polynomial<N>; L],
        ct1is: [Polynomial<N>; L],
        qis: [Field; L],
        gammas: Vec<Field>,
    ) {
        for i in 0..L {
            let gamma_i = gammas.get(i);

            let sum0 = self.sum_ct0is[i].eval(gamma_i);
            let sum1 = self.sum_ct1is[i].eval(gamma_i);

            let ct0 = ct0is[i].eval(gamma_i);
            let ct1 = ct1is[i].eval(gamma_i);

            let old0 = self.prev_ct0is[i].eval(gamma_i);
            let old1 = self.prev_ct1is[i].eval(gamma_i);

            let q_i = qis[i];

            let radd0 = self.r0is[i].eval(gamma_i);
            let radd1 = self.r1is[i].eval(gamma_i);

            assert(sum0 == ct0 + old0 + radd0 * q_i);
            assert(sum1 == ct1 + old1 + radd1 * q_i);
        }
    }

    /// Verifies range constraints on r0is and r1is.
    /// # Arguments
    /// * `BIT_R` - Bit-length of the range to check.
    pub fn check_range_bounds<let BIT_R: u32>(self: CiphertextAddition<N, L>) {
        for i in 0..L {
            self.r0is[i].range_check_1bound::<BIT_R>(self.r_bound);
            self.r1is[i].range_check_1bound::<BIT_R>(self.r_bound);
        }
    }
}

#[test]
fn test_check_ciphertext_addition_constraints() {
    let ct_add_params = CiphertextAddition {
        prev_ct0is: [Polynomial::new([2, 3, 4]), Polynomial::new([1, 1, 1])],
        prev_ct1is: [Polynomial::new([3, 4, 5]), Polynomial::new([1, 3, 2])],
        sum_ct0is: [Polynomial::new([8, 10, 12]), Polynomial::new([9, 11, 13])],
        sum_ct1is: [Polynomial::new([6, 7, 9]), Polynomial::new([8, 9, 7])],
        r0is: [Polynomial::new([5, 6, 7]), Polynomial::new([3, 4, 5])],
        r1is: [Polynomial::new([1, 2, 1]), Polynomial::new([2, 2, 2])],
        r_bound: 1,
    };

    let ct0is = [Polynomial::new([1, 1, 1]), Polynomial::new([2, 2, 2])];
    let ct1is = [Polynomial::new([2, 1, 3]), Polynomial::new([3, 2, 1])];

    let mut gammas = Vec::new();

    gammas.push(Field::from(2));
    gammas.push(Field::from(3));

    let qis = [Field::from(1), Field::from(2)];

    ct_add_params.check_ciphertext_addition_constraints(ct0is, ct1is, qis, gammas);
}

#[test]
fn test_check_range_bounds() {
    let ct_add_params = CiphertextAddition {
        prev_ct0is: [Polynomial::new([2, 3, 4]), Polynomial::new([1, 1, 1])],
        prev_ct1is: [Polynomial::new([3, 4, 5]), Polynomial::new([1, 3, 2])],
        sum_ct0is: [Polynomial::new([8, 10, 12]), Polynomial::new([9, 11, 13])],
        sum_ct1is: [Polynomial::new([6, 7, 9]), Polynomial::new([8, 9, 7])],
        r0is: [Polynomial::new([-1, 1, 0]), Polynomial::new([0, 1, 1])],
        r1is: [Polynomial::new([0, 1, 1]), Polynomial::new([0, 1, 1])],
        r_bound: 1,
    };

    ct_add_params.check_range_bounds::<4>();
}
