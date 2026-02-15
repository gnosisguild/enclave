// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::compute_user_data_encryption_challenge_commitment;
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for Greco circuit.
pub struct Configs<let N: u32, let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Plaintext modulus: q mod t
    pub q_mod_t: Field,
    /// Scaling factors for each basis: [k0_0, k0_1, ..., k0_{L-1}]
    pub k0is: [Field; L],
    /// Bounds for public key polynomials for each CRT basis
    pub pk_bounds: [Field; L],
    /// Bounds for error polynomials (e0)
    pub e0_bound: Field,
    /// Bounds for error polynomials (e1)
    pub e1_bound: Field,
    /// Bound for secret polynomial u (ternary distribution)
    pub u_bound: Field,
    /// Lower bounds for r1 polynomials (modulus switching quotients)
    pub r1_low_bounds: [Field; L],
    /// Upper bounds for r1 polynomials (modulus switching quotients)
    pub r1_up_bounds: [Field; L],
    /// Bounds for r2 polynomials (cyclotomic reduction quotients)
    pub r2_bounds: [Field; L],
    /// Bounds for p1 polynomials (modulus switching quotients)
    pub p1_bounds: [Field; L],
    /// Bounds for p2 polynomials (cyclotomic reduction quotients)
    pub p2_bounds: [Field; L],
    /// Lower bound for k1 polynomial (scaled message)
    pub k1_low_bound: Field,
    /// Upper bound for k1 polynomial (scaled message)
    pub k1_up_bound: Field,
}

impl<let N: u32, let L: u32> Configs<N, L> {
    pub fn new(
        q_mod_t: Field,
        qis: [Field; L],
        k0is: [Field; L],
        pk_bounds: [Field; L],
        e0_bound: Field,
        e1_bound: Field,
        u_bound: Field,
        r1_low_bounds: [Field; L],
        r1_up_bounds: [Field; L],
        r2_bounds: [Field; L],
        p1_bounds: [Field; L],
        p2_bounds: [Field; L],
        k1_low_bound: Field,
        k1_up_bound: Field,
    ) -> Self {
        Configs {
            qis,
            q_mod_t,
            k0is,
            pk_bounds,
            e0_bound,
            e1_bound,
            u_bound,
            r1_low_bounds,
            r1_up_bounds,
            r2_bounds,
            p1_bounds,
            p2_bounds,
            k1_low_bound,
            k1_up_bound,
        }
    }
}

/// Correct User Data Encryption Circuit under Threshold public key
///
/// Verifies:
/// 1. Range checks on all polynomial coefficients
/// 2. Correct encryption: ct0i = pk0i * u + e0 + k1 * k0i + r1i * qi + r2i * cyclo
///    and ct1i = pk1i * u + e1 + p1i * qi + p2i * cyclo
///
/// DISCLAIMER: Ported from Halo2 circuit by Greco paper authors @ PSE.
/// Halo2 implementation: https://github.com/privacy-scaling-explorations/greco
pub struct UserDataEncryption<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E0: u32, let BIT_E1: u32, let BIT_K: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_P1: u32, let BIT_P2: u32> {
    configs: Configs<N, L>,
    pk_commitment: Field,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e0is: [Polynomial<N>; L],
    e0_quotients: [Polynomial<N>; L],
    e1: Polynomial<N>,
    k1: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E0: u32, let BIT_E1: u32, let BIT_K: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_P1: u32, let BIT_P2: u32> UserDataEncryption<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E0, BIT_E1, BIT_K, BIT_R1, BIT_R2, BIT_P1, BIT_P2> {
    pub fn new(
        configs: Configs<N, L>,
        pk_commitment: Field,
        pk0is: [Polynomial<N>; L],
        pk1is: [Polynomial<N>; L],
        ct0is: [Polynomial<N>; L],
        ct1is: [Polynomial<N>; L],
        u: Polynomial<N>,
        e0: Polynomial<N>,
        e0is: [Polynomial<N>; L],
        e0_quotients: [Polynomial<N>; L],
        e1: Polynomial<N>,
        k1: Polynomial<N>,
        r1is: [Polynomial<2 * N - 1>; L],
        r2is: [Polynomial<N - 1>; L],
        p1is: [Polynomial<2 * N - 1>; L],
        p2is: [Polynomial<N - 1>; L],
    ) -> UserDataEncryption<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E0, BIT_E1, BIT_K, BIT_R1, BIT_R2, BIT_P1, BIT_P2> {
        UserDataEncryption {
            configs,
            pk_commitment,
            pk0is,
            pk1is,
            ct0is,
            ct1is,
            u,
            e0,
            e0is,
            e0_quotients,
            e1,
            k1,
            r1is,
            r2is,
            p1is,
            p2is,
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
    fn gammas_payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();

        inputs.push(self.pk_commitment);

        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct0is);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct1is);

        // Flatten common polynomials (used across all CRT bases)
        inputs = flatten::<_, _, BIT_E0>(inputs, [self.e0]);
        inputs = flatten::<_, _, BIT_E1>(inputs, [self.e1]);
        inputs = flatten::<_, _, BIT_U>(inputs, [self.u]);
        inputs = flatten::<_, _, BIT_K>(inputs, [self.k1]);

        // Flatten randomness polynomials for each CRT basis
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1is);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2is);
        inputs = flatten::<_, _, BIT_P1>(inputs, self.p1is);
        inputs = flatten::<_, _, BIT_P2>(inputs, self.p2is);

        inputs
    }

    /// Performs coefficient-wise CRT consistency check for e0 polynomial.
    ///
    /// Verifies that for each CRT basis i and each coefficient j:
    /// e0.coefficients[j] = e0is[i].coefficients[j] + e0_quotients[i].coefficients[j] * qi
    ///
    /// This ensures that e0 == e0is[i] (mod qi) for all coefficients, which is
    /// much more secure than checking equality at a single evaluation point.
    ///
    /// # Security
    /// Coefficient-wise checking prevents attacks where a malicious prover could
    /// construct different polynomials that happen to evaluate to the same value
    /// at a single challenge point.
    fn check_e0_crt_consistency(self) {
        for i in 0..L {
            // Check each coefficient satisfies the CRT relationship
            for j in 0..N {
                // Verify: e0_coeff = e0i_coeff + quotient_coeff * qi
                assert(
                    self.e0.coefficients[j]
                        == self.e0is[i].coefficients[j]
                            + self.e0_quotients[i].coefficients[j] * self.configs.qis[i],
                );
            }
        }
    }

    /// Verifies the correct encryption constraints for the Greco circuit.
    ///
    /// This function implements the core zero-knowledge proof by checking:
    /// 1. Binary constraint on k1 polynomial
    /// 2. Range constraints on all polynomials coefficients
    /// 3. CRT consistency for e0 polynomial
    /// 4. Correct encryption equations
    ///
    /// The proof uses the Schwartz-Zippel lemma: if polynomial equations hold
    /// when evaluated at random points, then the polynomials are identical with
    /// high probability.
    ///
    /// # Encryption Equations
    /// For each CRT basis i:
    /// * ct0i(gamma) = pk0i(gamma) * u(gamma) + e0(gamma) + k1(gamma) * k0i + r1i(gamma) * qi + r2i(gamma) * cyclo
    /// * ct1i(gamma) = pk1i(gamma) * u(gamma) + e1(gamma) + p1i(gamma) * qi + p2i(gamma) * cyclo
    ///
    /// Where:
    /// * cyclo(gamma) = gamma^N + 1 (cyclotomic polynomial)
    /// * qi, k0i are constants from the cryptographic parameters
    /// * r1i, r2i, p1i, p2i are randomness polynomials for each i-th CRT basis.
    ///
    /// # Returns
    /// True if the encryption constraints are satisfied, false otherwise.
    pub fn execute(self) -> bool {
        // Step 1: Perform range checks on all polynomial coefficients
        self.check_range_bounds();

        // Step 2: Check CRT consistency for e0 polynomial
        self.check_e0_crt_consistency();

        // Step 3: Generate Fiat-Shamir challenges
        let gammas = self.generate_challenge();

        // Step 4: Verify encryption constraints using challenges
        self.verify_evaluations(gammas)
    }

    /// Performs range checks on all polynomial coefficients.
    ///
    /// Checks that all polynomial coefficients are within their expected bounds
    /// as specified in the `configs`. This prevents attacks where coefficients
    /// are outside the valid range, which could break the security properties
    /// of the encryption scheme.
    ///
    /// # Panics
    /// This function will cause the circuit to fail if any coefficient is outside
    /// its expected bounds. The bounds are defined per polynomial type in the `configs`.
    fn check_range_bounds(self) {
        self.u.range_check_2bounds::<BIT_U>(self.configs.u_bound, self.configs.u_bound);
        self.e0.range_check_2bounds::<BIT_E0>(self.configs.e0_bound, self.configs.e0_bound);
        self.e1.range_check_2bounds::<BIT_E1>(self.configs.e1_bound, self.configs.e1_bound);
        self.k1.range_check_2bounds::<BIT_K>(self.configs.k1_up_bound, self.configs.k1_low_bound);

        for i in 0..L {
            self.pk0is[i].range_check_2bounds::<BIT_PK>(
                self.configs.pk_bounds[i],
                self.configs.pk_bounds[i],
            );
            self.pk1is[i].range_check_2bounds::<BIT_PK>(
                self.configs.pk_bounds[i],
                self.configs.pk_bounds[i],
            );

            self.r1is[i].range_check_2bounds::<BIT_R1>(
                self.configs.r1_up_bounds[i],
                self.configs.r1_low_bounds[i],
            );
            self.r2is[i].range_check_2bounds::<BIT_R2>(
                self.configs.r2_bounds[i],
                self.configs.r2_bounds[i],
            );

            self.p1is[i].range_check_2bounds::<BIT_P1>(
                self.configs.p1_bounds[i],
                self.configs.p1_bounds[i],
            );
            self.p2is[i].range_check_2bounds::<BIT_P2>(
                self.configs.p2_bounds[i],
                self.configs.p2_bounds[i],
            );
        }
    }

    /// Generates Fiat-Shamir challenge values using the SAFE cryptographic sponge.
    ///
    /// This function implements the Fiat-Shamir transform for the Greco circuit:
    /// 1. First, it computes and verifies the public key commitment by absorbing
    ///    all public key polynomials and squeezing a single commitment value.
    /// 2. Then, it generates challenge values by absorbing all witness values
    ///    (ciphertexts, errors, randomness) and squeezing 2L challenge values.
    ///
    /// The sponge absorbs all witness values and squeezes out deterministic random
    /// field elements that will be used to evaluate polynomials for the Schwartz-Zippel lemma.
    ///
    /// # Returns
    /// Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}] where:
    /// - gamma_0 is used as the primary evaluation point
    /// - gamma_1, ..., gamma_{L-1} are used for linear combination of ct0 constraints
    /// - gamma_L, ..., gamma_{2L-1} are used for linear combination of ct1 constraints
    fn generate_challenge(self) -> Vec<Field> {
        compute_user_data_encryption_challenge_commitment::<N, L, BIT_PK>(
            self.pk0is,
            self.pk1is,
            self.gammas_payload(),
            self.pk_commitment,
        )
    }

    /// Verifies encryption constraints using Fiat-Shamir challenges.
    ///
    /// For each CRT basis i, this function verifies that the encryption equations hold
    /// when evaluated at the challenge points. It uses the Schwartz-Zippel lemma:
    /// if polynomial equations hold when evaluated at random points, then the polynomials
    /// are identical with high probability.
    ///
    /// The verification combines all CRT bases using a linear combination with the
    /// challenge values to reduce the number of constraints while maintaining security.
    ///
    /// # Arguments
    /// * `gammas` - Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}]
    ///              generated by `generate_challenge()`
    ///
    /// # Returns
    /// `true` if all encryption constraints are satisfied, `false` otherwise.
    fn verify_evaluations(self, gammas: Vec<Field>) -> bool {
        let gamma = gammas.get(0);
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;
        let u_at_gamma = self.u.eval(gamma);
        let e1_at_gamma = self.e1.eval(gamma);
        let k1_at_gamma = self.k1.eval(gamma);

        let mut sum = (0, 0);
        for i in 0..L {
            let pk0is_at_gamma = self.pk0is[i].eval(gamma);
            let r1i_at_gamma = self.r1is[i].eval(gamma);
            let r2i_at_gamma = self.r2is[i].eval(gamma);
            let e0is_at_gamma = self.e0is[i].eval(gamma);

            let pk0_u = (pk0is_at_gamma * u_at_gamma) + e0is_at_gamma;
            let mut ct0_rhs = pk0_u + (k1_at_gamma * self.configs.k0is[i]);
            ct0_rhs += r1i_at_gamma * self.configs.qis[i];
            ct0_rhs += r2i_at_gamma * cyclo_at_gamma;
            let ct0_lhs = self.ct0is[i].eval(gamma);
            let pk1is_at_gamma = self.pk1is[i].eval(gamma);
            let p1is_at_gamma = self.p1is[i].eval(gamma);
            let p2is_at_gamma = self.p2is[i].eval(gamma);
            let pk1_u = (pk1is_at_gamma * u_at_gamma) + e1_at_gamma;
            let mut ct1_rhs = pk1_u + p2is_at_gamma * cyclo_at_gamma;
            ct1_rhs += p1is_at_gamma * self.configs.qis[i];
            let ct1_lhs = self.ct1is[i].eval(gamma);
            let gamma_i = if i == 0 { 1 } else { gammas.get(i) };
            sum = (
                sum.0 + ct0_lhs * gamma_i + ct1_lhs * gammas.get(i + L),
                sum.1 + ct0_rhs * gamma_i + ct1_rhs * gammas.get(i + L),
            );
        }

        sum.0 == sum.1
    }
}
