// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_share_computation_e_sm_commitment, compute_share_computation_sk_commitment,
    compute_threshold_pk_challenge, compute_threshold_pk_commitment,
};
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for threshold public key generation circuit.
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

/// Correct Threshold Public Key Contribution Generation Circuit (C1).
///
/// This circuit proves that a contribution to the threshold BFV public key was generated correctly
/// from secret values without revealing those secrets. Each ciphernode generates
/// their own key contribution, and this circuit ensures the generation followed
/// the BFV key generation equations.
///
/// Verifies:
/// 1. Range checks on all secret witnesses (sk, eek, e_sm, quotients)
/// 2. Compute commitments to sk, pk, and e_sm
/// 3. Generate Fiat-Shamir challenges (one per CRT modulus)
/// 4. Verify BFV key generation equations at challenge points (Schwartz-Zippel)
///
/// Outputs:
/// - commit(sk) -> C2a (secret key share verification)
/// - commit(e_sm) -> C2b (smudging noise share verification)
/// - commit(pk_trbfv) -> C5 (public key aggregation)
pub struct PkGeneration<let N: u32, let L: u32, let BIT_EEK: u32, let BIT_SK: u32, let BIT_E_SM: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_PK: u32> {
    /// Cryptographic parameters including bounds, moduli, and constants.
    configs: Configs<N, L>,

    /// Common Reference String polynomials (public witnesses)
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
    e_sm: [Polynomial<N>; L],

    /// Quotients from polynomial operations (secret witnesses)
    /// r1[i] are quotients from modulus switching for modulus i (can be negative, degree 2*N-1)
    r1: [Polynomial<2 * N - 1>; L],
    /// r2[i] are quotients from cyclotomic reduction for modulus i (typically positive, degree N-1)
    r2: [Polynomial<N - 1>; L],

    /// Threshold public key components (committed witnesses)
    /// pk0[i] is the first component of the public key for modulus i
    pk0: [Polynomial<N>; L],
    /// pk1[i] is the second component of the public key for modulus i (should equal a[i])
    pk1: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_EEK: u32, let BIT_SK: u32, let BIT_E_SM: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_PK: u32> PkGeneration<N, L, BIT_EEK, BIT_SK, BIT_E_SM, BIT_R1, BIT_R2, BIT_PK> {
    pub fn new(
        configs: Configs<N, L>,
        a: [Polynomial<N>; L],
        eek: Polynomial<N>,
        sk: Polynomial<N>,
        e_sm: [Polynomial<N>; L],
        r1: [Polynomial<2 * N - 1>; L],
        r2: [Polynomial<N - 1>; L],
        pk0: [Polynomial<N>; L],
        pk1: [Polynomial<N>; L],
    ) -> Self {
        PkGeneration { configs, a, eek, sk, e_sm, r1, r2, pk0, pk1 }
    }

    /// Flattens all witness data into a single array for Fiat-Shamir challenge generation.
    ///
    /// Uses commitments for sk, pk, and e_sm rather than full polynomials to reduce
    /// constraint count while maintaining binding properties.
    fn payload(self, sk_commitment: Field, pk_commitment: Field) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Flatten CRS polynomials a (L polynomials of degree N)
        inputs = flatten::<_, _, BIT_PK>(inputs, self.a);

        // Flatten error polynomial eek (1 polynomial of degree N)
        inputs = flatten::<_, _, BIT_EEK>(inputs, [self.eek]);

        // Use commitments instead of full polynomials
        inputs.push(sk_commitment);
        inputs.push(pk_commitment);

        // Flatten quotient polynomials (L polynomials each)
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2);

        inputs
    }

    /// Executes the threshold public key generation circuit.
    ///
    /// Returns a tuple of three commitments:
    /// - commit(sk) -> C2a (secret key share verification)
    /// - commit(e_sm) -> C2b (smudging noise share verification)
    /// - commit(pk_trbfv) -> C5 (public key aggregation)
    pub fn execute(self) -> (Field, Field, Field) {
        // 1. Perform range checks on all secret witness values
        self.perform_range_checks();

        // 2. Compute commitments
        let sk_commitment = compute_share_computation_sk_commitment::<N, BIT_SK>(self.sk);
        let e_sm_commitment =
            compute_share_computation_e_sm_commitment::<N, L, BIT_E_SM>(self.e_sm);
        let pk_commitment = compute_threshold_pk_commitment::<N, L, BIT_PK>(self.pk0, self.pk1);

        // 3. Generate Fiat-Shamir challenges (one per CRT modulus)
        let gamma = self.generate_challenge(sk_commitment, pk_commitment);
        self.verify_evaluations(gamma);

        // 4. Return all commitments
        (sk_commitment, pk_commitment, e_sm_commitment)
    }

    /// Generates Fiat-Shamir challenge values using the SAFE cryptographic sponge.
    ///
    /// Returns a single challenge point `gamma` used to verify key generation
    /// equations for all CRT moduli via the Schwartz-Zippel lemma.
    fn generate_challenge(self, sk_commitment: Field, pk_commitment: Field) -> Field {
        let inputs = self.payload(sk_commitment, pk_commitment);

        compute_threshold_pk_challenge(inputs)
    }

    /// Performs range checks on all secret witness values.
    ///
    /// These bounds are critical for BFV security-large coefficients would
    /// break the scheme's hardness assumptions (Ring-LWE).
    fn perform_range_checks(self) {
        // Check that error polynomial has small coefficients
        self.eek.range_check_2bounds::<BIT_EEK>(self.configs.eek_bound, self.configs.eek_bound);

        // Check that secret key polynomial has small coefficients
        self.sk.range_check_2bounds::<BIT_SK>(self.configs.sk_bound, self.configs.sk_bound);

        // Check quotient terms are within expected bounds (per modulus)
        for i in 0..L {
            self.e_sm[i].range_check_2bounds::<BIT_E_SM>(
                self.configs.e_sm_bound,
                self.configs.e_sm_bound,
            );

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

    /// Verifies BFV key generation equations for all CRT moduli using Schwartz-Zippel.
    ///
    /// For each CRT modulus i, verifies at the challenge point gamma:
    /// - `pk0[i](gamma) = -a[i](gamma) * sk(gamma) + eek(gamma) + r2[i](gamma) * (gamma^N + 1) + r1[i](gamma) * q_i`
    /// - `pk1[i](gamma) = a[i](gamma)`
    ///
    /// The Schwartz-Zippel lemma states that if these polynomial equations hold at a
    /// random point, they hold for all coefficients with overwhelming probability.
    fn verify_evaluations(self, gamma: Field) {
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;
        let sk_at_gamma = self.sk.eval(gamma);
        let eek_at_gamma = self.eek.eval(gamma);

        for i in 0..L {
            let expected_pk0 = -self.a[i].eval(gamma) * sk_at_gamma
                + eek_at_gamma
                + self.r2[i].eval(gamma) * cyclo_at_gamma
                + self.r1[i].eval(gamma) * self.configs.qis[i];

            assert(self.pk0[i].eval(gamma) == expected_pk0, "Public key equation 1 failed");
            assert(
                self.pk1[i].eval(gamma) == self.a[i].eval(gamma),
                "Public key equation 2 failed",
            );
        }
    }
}
