// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_pk_trbfv_commitment, compute_secret_commitment, compute_trbfv_pk_challenge_commitment,
    prepare_pk_commitment_payload, prepare_single_polynomial_commitment_payload,
};
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for TRBFV public key generation circuit.
pub struct Configs<let N: u32, let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Bound for error polynomial (eek) coefficients
    pub eek_bound: Field,
    /// Bound for secret key polynomial (sk) coefficients
    pub sk_bound: Field,
    /// Bound for smudging noise polynomial (e_sm) coefficients
    pub e_sm_bound: Field,
    /// Bounds for r1 polynomials (modulus switching quotients) for each CRT basis
    pub r1_bounds: [Field; L],
    /// Bounds for r2 polynomials (cyclotomic reduction quotients) for each CRT basis
    pub r2_bounds: [Field; L],
}

impl<let N: u32, let L: u32> Configs<N, L> {
    pub fn new(
        qis: [Field; L],
        eek_bound: Field,
        sk_bound: Field,
        e_sm_bound: Field,
        r1_bounds: [Field; L],
        r2_bounds: [Field; L],
    ) -> Self {
        Configs { qis, eek_bound, sk_bound, e_sm_bound, r1_bounds, r2_bounds }
    }
}

/// TRBFV Public Key Verification (Circuit 1).
///
/// Verifies:
/// 1. Range checks on all secret witnesses (secret key, error, smudging noise, quotients)
/// 2. Correct public key generation: pk0_i = -a_i * sk + eek + r2_i * (X^N + 1) + r1_i * q_i
///    and pk1_i = a_i
///
/// Outputs:
/// - commit(sk_trbfv)
/// - commit(pk_trbfv)
/// - commit(e_sm)
pub struct TrbfvPublicKey<let N: u32, let L: u32, let BIT_EEK: u32, let BIT_SK: u32, let BIT_E_SM: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_PK: u32> {
    /// Cryptographic parameters including bounds, moduli, and constants.
    configs: Configs<N, L>,

    /// Common Reference String polynomials (public)
    /// One polynomial per modulus i
    a: [Polynomial<N>; L],

    /// Error polynomial (secret witness)
    /// Small coefficients sampled from error distribution
    eek: Polynomial<N>,

    /// Secret key polynomial (secret witness)
    /// Small coefficients sampled from CBD (Centered Binomial Distribution)
    sk: Polynomial<N>,

    /// Smudging noise polynomial (secret witness)
    /// Used for threshold decryption security
    e_sm: Polynomial<N>,

    /// Quotients from polynomial operations (secret witness)
    /// r1[i] are quotients from modulus switching for modulus i (can be negative, degree 2*N-1)
    r1: [Polynomial<2 * N - 1>; L],
    /// r2[i] are quotients from cyclotomic reduction for modulus i (typically positive, degree N-1)
    r2: [Polynomial<N - 1>; L],

    /// TRBFV public key components (public output)
    /// pk0[i] is the first component of the public key for modulus i
    pk0: [Polynomial<N>; L],
    /// pk1[i] is the second component of the public key for modulus i (should equal a[i])
    pk1: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_EEK: u32, let BIT_SK: u32, let BIT_E_SM: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_PK: u32> TrbfvPublicKey<N, L, BIT_EEK, BIT_SK, BIT_E_SM, BIT_R1, BIT_R2, BIT_PK> {
    pub fn new(
        configs: Configs<N, L>,
        a: [Polynomial<N>; L],
        eek: Polynomial<N>,
        sk: Polynomial<N>,
        e_sm: Polynomial<N>,
        r1: [Polynomial<2 * N - 1>; L],
        r2: [Polynomial<N - 1>; L],
        pk0: [Polynomial<N>; L],
        pk1: [Polynomial<N>; L],
    ) -> Self {
        TrbfvPublicKey { configs, a, eek, sk, e_sm, r1, r2, pk0, pk1 }
    }

    /// Flattens all witness data into a single array for Fiat-Shamir challenge generation
    fn payload(
        self,
        sk_commitment: Field,
        pk_commitment: Field,
        e_sm_commitment: Field,
    ) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Flatten CRS polynomials a (L polynomials of degree N)
        inputs = flatten::<_, _, BIT_EEK>(inputs, self.a);

        // Flatten error polynomial eek (1 polynomial of degree N)
        inputs = flatten::<_, _, BIT_EEK>(inputs, [self.eek]);

        // Use commitments instead of full polynomials
        inputs.push(sk_commitment);
        inputs.push(pk_commitment);
        inputs.push(e_sm_commitment);

        // Flatten quotient polynomials (L polynomials each)
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2);

        inputs
    }

    /// Main verification function
    /// Returns (commit(sk), commit(pk), commit(e_sm))
    pub fn verify(self) -> (Field, Field, Field) {
        // Step 1: Perform range checks on all secret witness values
        self.perform_range_checks();

        // Step 2: Compute commitments
        let sk_payload = prepare_single_polynomial_commitment_payload::<N, BIT_SK>(self.sk);
        let e_sm_payload = prepare_single_polynomial_commitment_payload::<N, BIT_E_SM>(self.e_sm);
        let pk_payload = prepare_pk_commitment_payload::<N, L, BIT_PK>(self.pk0, self.pk1);

        let sk_commitment = compute_secret_commitment(sk_payload);
        let e_sm_commitment = compute_secret_commitment(e_sm_payload);
        let pk_commitment = compute_pk_trbfv_commitment(pk_payload);

        // Step 3: Generate Fiat-Shamir challenges using commitments
        let gammas = self.generate_challenge(sk_commitment, pk_commitment, e_sm_commitment);

        // Step 4: Verify public key equations for each modulus
        for i in 0..L {
            let gamma = gammas.get(i);
            self.verify_public_key_for_modulus(i, gamma);
        }

        // Step 5: Return all commitments
        (sk_commitment, pk_commitment, e_sm_commitment)
    }

    /// Generates Fiat-Shamir challenge values using the SAFE cryptographic sponge
    fn generate_challenge(
        self,
        sk_commitment: Field,
        pk_commitment: Field,
        e_sm_commitment: Field,
    ) -> Vec<Field> {
        let inputs = self.payload(sk_commitment, pk_commitment, e_sm_commitment);

        compute_trbfv_pk_challenge_commitment::<L>(inputs)
    }

    /// Performs range checks on all secret witness values
    fn perform_range_checks(self) {
        // Check that error polynomial has small coefficients
        self.eek.range_check_2bounds::<BIT_EEK>(self.configs.eek_bound, self.configs.eek_bound);

        // Check that secret key polynomial has small coefficients
        self.sk.range_check_2bounds::<BIT_SK>(self.configs.sk_bound, self.configs.sk_bound);

        // Check that smudging noise polynomial has small coefficients
        self.e_sm.range_check_2bounds::<BIT_E_SM>(self.configs.e_sm_bound, self.configs.e_sm_bound);

        // Check quotient terms are within expected bounds (per modulus)
        for i in 0..L {
            self.r1[i].range_check_2bounds::<BIT_R1>(
                self.configs.r1_bounds[i],
                self.configs.r1_bounds[i],
            );

            self.r2[i].range_check_2bounds::<BIT_R2>(
                self.configs.r2_bounds[i],
                self.configs.r2_bounds[i],
            );
        }
    }

    /// Verifies the TRBFV public key generation equations for a specific CRT basis
    fn verify_public_key_for_modulus(self, i: u32, gamma: Field) {
        // Evaluate all polynomials at the random challenge point gamma
        let a_at_gamma = self.a.map(|a_poly| a_poly.eval(gamma));

        let eek_at_gamma = self.eek.eval(gamma);
        let sk_at_gamma = self.sk.eval(gamma);

        let r1_at_gamma = self.r1.map(|r1_poly| r1_poly.eval(gamma));
        let r2_at_gamma = self.r2.map(|r2_poly| r2_poly.eval(gamma));

        let pk0_at_gamma = self.pk0.map(|pk0_poly| pk0_poly.eval(gamma));
        let pk1_at_gamma = self.pk1.map(|pk1_poly| pk1_poly.eval(gamma));

        // Evaluate the cyclotomic polynomial X^N + 1 at gamma
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;

        // pk0_i = -a_i * sk + eek + r2_i * (X^N + 1) + r1_i * q_i
        let expected_pk0 = -a_at_gamma[i] * sk_at_gamma
            + eek_at_gamma
            + r2_at_gamma[i] * cyclo_at_gamma
            + r1_at_gamma[i] * self.configs.qis[i];

        assert(pk0_at_gamma[i] == expected_pk0, "Public key equation 1 failed");

        // Equation 2: pk1_i = a_i
        assert(pk1_at_gamma[i] == a_at_gamma[i], "Public key equation 2 failed");
    }
}
