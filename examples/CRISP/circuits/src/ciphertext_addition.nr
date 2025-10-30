// SPDX-License-Identifier: LGPL-3.0-only
//
// This file contains ciphertext addition verification for BFV ciphertexts.

use greco::CryptographicParams;
use polynomial::{flatten, Polynomial};
use safe::SafeSponge;

/// Parameters for ciphertext addition verification.
///
/// # Arguments
/// * `N` - Polynomial degree.
/// * `L` - Number of CRT bases.
/// * `BIT_ZERO_CT` - Bit length of the zero ciphertext polynomials.
/// * `BIT_PREV_CT` - Bit length of the previous ciphertext polynomials.
/// * `BIT_SUM_CT` - Bit length of the sum ciphertext polynomials.
pub struct CiphertextAddition<let N: u32, let L: u32, let BIT_ZERO_CT: u32, let BIT_PREV_CT: u32, let BIT_SUM_CT: u32> {
    crypto_params: CryptographicParams<L>,
    zero_ct0is: [Polynomial<N>; L],
    zero_ct1is: [Polynomial<N>; L],
    prev_ct0is: [Polynomial<N>; L],
    prev_ct1is: [Polynomial<N>; L],
    sum_ct0is: [Polynomial<N>; L],
    sum_ct1is: [Polynomial<N>; L],
    r0is: [Polynomial<N>; L],
    r1is: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_ZERO_CT: u32, let BIT_PREV_CT: u32, let BIT_SUM_CT: u32> CiphertextAddition<N, L, BIT_ZERO_CT, BIT_PREV_CT, BIT_SUM_CT> {
    /// Creates a new CiphertextAddition instance.
    ///
    /// # Arguments
    /// * `crypto_params` - Cryptographic parameters for the ciphertext addition circuit.
    /// * `zero_ct0is`, `zero_ct1is` - Zero ciphertext polynomials.
    /// * `prev_ct0is`, `prev_ct1is` - Previous ciphertext polynomials.
    /// * `sum_ct0is`, `sum_ct1is` - Ciphertext addition polynomials.
    /// * `r0is`, `r1is` - Randomness polynomials.
    pub fn new(
        crypto_params: CryptographicParams<L>,
        zero_ct0is: [Polynomial<N>; L],
        zero_ct1is: [Polynomial<N>; L],
        prev_ct0is: [Polynomial<N>; L],
        prev_ct1is: [Polynomial<N>; L],
        sum_ct0is: [Polynomial<N>; L],
        sum_ct1is: [Polynomial<N>; L],
        r0is: [Polynomial<N>; L],
        r1is: [Polynomial<N>; L],
    ) -> Self {
        Self {
            crypto_params,
            zero_ct0is,
            zero_ct1is,
            prev_ct0is,
            prev_ct1is,
            sum_ct0is,
            sum_ct1is,
            r0is,
            r1is,
        }
    }

    /// Flattens all polynomials coefficients into a single array for challenge generation.
    ///
    /// This function serializes all polynomial coefficients into a 1D array to enable
    /// the generation of random challenge values using the Fiat-Shamir transform.
    /// The coefficients are arranged in a specific order to ensure deterministic
    /// challenge generation.
    ///
    /// # Returns
    /// An array containing all polynomials coefficients in flattened form
    fn payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Flatten zero ciphertext polynomials
        inputs = flatten::<_, _, BIT_ZERO_CT>(inputs, self.zero_ct0is);
        inputs = flatten::<_, _, BIT_ZERO_CT>(inputs, self.zero_ct1is);

        // Flatten previous ciphertext polynomials
        inputs = flatten::<_, _, BIT_PREV_CT>(inputs, self.prev_ct0is);
        inputs = flatten::<_, _, BIT_PREV_CT>(inputs, self.prev_ct1is);

        // Flatten sum ciphertext polynomials
        inputs = flatten::<_, _, BIT_SUM_CT>(inputs, self.sum_ct0is);
        inputs = flatten::<_, _, BIT_SUM_CT>(inputs, self.sum_ct1is);

        // Flatten randomness polynomials
        inputs = flatten::<_, _, 1>(inputs, self.r0is);
        inputs = flatten::<_, _, 1>(inputs, self.r1is);

        inputs
    }

    /// Verifies the correct addition constraints for the ciphertext addition circuit.
    ///
    /// This function implements the core zero-knowledge proof by checking:
    /// 1. Range constraints on all polynomials coefficients
    /// 2. Correct addition equations
    ///
    /// The proof uses the Schwartz-Zippel lemma: if polynomial equations hold
    /// when evaluated at random points, then the polynomials are identical with
    /// high probability.
    ///
    /// # Addition Equations
    /// For each CRT basis i:
    /// * sum0i(gamma) = zero_ct0i(gamma) + prev_ct0i(gamma) + r0i(gamma) * qi
    /// * sum1i(gamma) = zero_ct1i(gamma) + prev_ct1i(gamma) + r1i(gamma) * qi
    ///
    /// Where:
    /// * qi are constants from the cryptographic parameters
    /// * r0i, r1i are the correction terms for each i-th CRT basis.
    ///
    /// # Returns
    /// True if the ciphertext addition constraints are satisfied, false otherwise.
    pub fn verify(self) -> bool {
        // Step 1: Perform range checks on all polynomial coefficients
        self.check_range_bounds();

        // Step 2: Generate Fiat-Shamir challenges
        let gammas = self.generate_challenge();

        // Step 3: Verify ciphertext addition constraints using challenges
        self.verify_evaluations(gammas)
    }

    /// Verifies range constraints on r0is and r1is.
    fn check_range_bounds(self) {
        for i in 0..L {
            // For M=2 (adding two ciphertexts), each coefficient of the quotient polynomial.
            // must be in {-1, 0, 1}, so the bound will always be 1 for all CRT moduli.
            self.r0is[i].range_check_1bound::<1>(1);
            self.r1is[i].range_check_1bound::<1>(1);
        }
    }

    /// Generates Fiat-Shamir challenge values using a cryptographic sponge
    ///
    /// The sponge absorbs all witness values and squeezes out deterministic random field elements
    /// that will be used to evaluate polynomials for the Schwartz-Zippel lemma.
    ///
    /// # Returns
    /// Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}]
    fn generate_challenge(self) -> Vec<Field> {
        let inputs = self.payload();

        // Domain separator for ciphertext addition circuit - "CiphertextAddition" in hex
        let domain_separator = [
            0x43, 0x69, 0x70, 0x68, 0x65, 0x72, 0x74, 0x65, 0x78, 0x74, 0x41, 0x64, 0x64, 0x69,
            0x74, 0x69, 0x6f, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        // IO Pattern: ABSORB(input_size), SQUEEZE(2*L)
        let input_size = inputs.len();
        let io_pattern = [0x80000000 | input_size, 0x00000000 | (2 * L)];

        let mut sponge = SafeSponge::start(io_pattern, domain_separator);
        sponge.absorb(inputs);
        let gammas = sponge.squeeze();
        sponge.finish();
        gammas
    }

    /// Verifies ciphertext addition constraints using Fiat-Shamir challenges.
    ///
    /// # Arguments
    /// * `gammas` - Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}]
    ///
    /// # Returns
    /// True if the ciphertext addition constraints are satisfied, false otherwise.
    fn verify_evaluations(self, gammas: Vec<Field>) -> bool {
        let mut result = true;

        for i in 0..L {
            let gamma_i = gammas.get(i);

            let sum0 = self.sum_ct0is[i].eval(gamma_i);
            let sum1 = self.sum_ct1is[i].eval(gamma_i);

            let ct0 = self.zero_ct0is[i].eval(gamma_i);
            let ct1 = self.zero_ct1is[i].eval(gamma_i);

            let old0 = self.prev_ct0is[i].eval(gamma_i);
            let old1 = self.prev_ct1is[i].eval(gamma_i);

            let q_i = self.crypto_params.qis[i];

            let radd0 = self.r0is[i].eval(gamma_i);
            let radd1 = self.r1is[i].eval(gamma_i);

            result = result & (sum0 == ct0 + old0 + radd0 * q_i);
            result = result & (sum1 == ct1 + old1 + radd1 * q_i);
        }

        result
    }
}
