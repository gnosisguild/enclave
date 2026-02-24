// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_aggregated_shares_commitment, compute_threshold_share_decryption_challenge,
};
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for Threshold decryption share circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Coefficient bounds for r1 polynomials (modulus switching quotients, per CRT basis)
    pub r1_bounds: [Field; L],
    /// Coefficient bounds for r2 polynomials (cyclotomic reduction quotients, per CRT basis)
    pub r2_bounds: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L], r1_bounds: [Field; L], r2_bounds: [Field; L]) -> Self {
        Configs { qis, r1_bounds, r2_bounds }
    }
}

/// Decryption Share Circuit (C6).
///
/// Proves that a ciphernode correctly computed its decryption share from the homomorphic
/// result ciphertext, using the aggregated secret key share and smudging noise committed
/// during P1 (DKG). Runs once per ciphernode; T+1 valid proofs are required before
/// C7 can reconstruct the plaintext.
///
/// # Decryption Share Formula
///
/// For each CRT basis l:
///   d[l] = ct0[l] + ct1[l] * sk[l] + e_sm[l] + r2[l] * (X^N + 1) + r1[l] * q_l
///
/// where:
///   - sk      is the ciphernode's aggregated secret key share (sum of received shares, from C4a)
///   - e_sm    is the ciphernode's aggregated smudging noise share (from C4b)
///   - r1, r2  are quotient polynomials for modular reduction (secret witnesses)
///
/// Smudging:
///
/// The decryption share d is public but reveals nothing about sk. The smudging noise
/// e_sm is chosen large enough to statistically mask the ct1 * sk term, so an adversary
/// observing all T+1 decryption shares cannot extract information about any party's
/// secret key share.
///
/// Outputs:
///
/// d[l] -> public decryption share per CRT basis, consumed by C7 (dec-result-trbfv).
pub struct ShareDecryption<let N: u32, let L: u32, let BIT_CT: u32, let BIT_SK: u32, let BIT_E_SM: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_D: u32> {
    /// Circuit parameters including bounds and cryptographic constants
    configs: Configs<L>,

    /// Expected commitment to the aggregated secret key share, from C4a.
    /// Binds this proof to the sk committed during DKG.
    /// (public witness)
    expected_sk_commitment: Field,

    /// Expected commitment to the aggregated smudging noise share, from C4b.
    /// Binds this proof to the e_sm committed during DKG.
    /// (public witness)
    expected_e_sm_commitment: Field,

    /// First ciphertext component per CRT basis.
    /// (public witnesses)
    ct0: [Polynomial<N>; L],

    /// Second ciphertext component per CRT basis.
    /// (public witnesses)
    ct1: [Polynomial<N>; L],

    /// Aggregated secret key share: sum of all received sk shares (from C4a).
    /// Verified against expected_sk_commitment; range checks handled by DKG circuits.
    /// (secret witness)
    sk: [Polynomial<N>; L],

    /// Aggregated smudging noise share: sum of all received e_sm shares (from C4b).
    /// Verified against expected_e_sm_commitment; range checks handled by DKG circuits.
    /// (secret witness)
    e_sm: [Polynomial<N>; L],

    /// Modulus switching quotient polynomials, per CRT basis.
    /// (secret witnesses)
    r1: [Polynomial<2 * N - 1>; L],

    /// Cyclotomic reduction quotient polynomials, per CRT basis.
    /// (secret witnesses)
    r2: [Polynomial<N - 1>; L],

    /// Claimed decryption share polynomials, per CRT basis.
    /// Made public after proof generation; consumed by C7 for Lagrange reconstruction.
    /// (public witnesses)
    d: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_CT: u32, let BIT_SK: u32, let BIT_E_SM: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_D: u32> ShareDecryption<N, L, BIT_CT, BIT_SK, BIT_E_SM, BIT_R1, BIT_R2, BIT_D> {
    pub fn new(
        configs: Configs<L>,
        expected_sk_commitment: Field,
        expected_e_sm_commitment: Field,
        ct0: [Polynomial<N>; L],
        ct1: [Polynomial<N>; L],
        sk: [Polynomial<N>; L],
        e_sm: [Polynomial<N>; L],
        r1: [Polynomial<2 * N - 1>; L],
        r2: [Polynomial<N - 1>; L],
        d: [Polynomial<N>; L],
    ) -> Self {
        ShareDecryption {
            configs,
            expected_sk_commitment,
            expected_e_sm_commitment,
            ct0,
            ct1,
            sk,
            e_sm,
            r1,
            r2,
            d,
        }
    }

    /// Verifies that the aggregated secret key share hashes to its expected commitment from C4a.
    ///
    /// Ensures the ciphernode is using the exact sk committed during DKG.
    /// Any substitution of a different key will cause this assertion to fail.
    fn verify_agg_sk_commitment(self) {
        assert(
            compute_aggregated_shares_commitment::<N, L, BIT_SK>(self.sk)
                == self.expected_sk_commitment,
            "S commitment mismatch",
        );
    }

    /// Verifies that the aggregated smudging noise share hashes to its expected commitment from C4b.
    ///
    /// Ensures the ciphernode is using the exact e_sm committed during DKG.
    /// Any substitution of different noise will cause this assertion to fail.
    fn verify_agg_e_sm_commitment(self) {
        assert(
            compute_aggregated_shares_commitment::<N, L, BIT_E_SM>(self.e_sm)
                == self.expected_e_sm_commitment,
            "E commitment mismatch",
        );
    }

    /// Serializes all inputs into a flat array for Fiat-Shamir challenge generation.
    ///
    /// sk and e_sm are included via their commitments rather than their full coefficient
    /// arrays, this saves constraints while still binding them to the transcript.
    /// The serialization order is load-bearing: any change produces different challenge
    /// values and breaks existing proofs.
    ///
    /// Order: expected_sk_commitment, expected_e_sm_commitment, ct0, ct1, r1, r2, d.
    fn payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Use commitments instead of full polynomials (saves constraints)
        inputs.push(self.expected_sk_commitment);
        inputs.push(self.expected_e_sm_commitment);

        // Flatten ciphertext components (public inputs)
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct0);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct1);

        // Flatten quotient polynomials (secret witnesses)
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2);

        // Flatten decryption shares (public outputs)
        inputs = flatten::<_, _, BIT_D>(inputs, self.d);

        inputs
    }

    /// Executes the decryption share circuit.
    ///
    /// Verifies commitment integrity for sk and e_sm, range checks the quotient
    /// polynomials, derives the Fiat-Shamir challenge, then checks the decryption
    /// equation at the challenge point for each CRT basis.
    pub fn execute(self) {
        // Step 1: Verify sk commitment matches expected
        self.verify_agg_sk_commitment();

        // Step 2: Verify e_sm commitment matches expected
        self.verify_agg_e_sm_commitment();

        // Step 3: Perform range checks on quotient polynomials
        // Note: sk and e_sm range checks are handled by commitment verification
        // (the DKG decryption circuit already performed range checks on these values)
        self.check_range_bounds();

        // Step 4: Generate Fiat-Shamir challenge from the transcript
        let gamma = self.generate_challenge();

        // Step 5: Verify decryption share computation for each CRT basis
        for i in 0..L {
            self.verify_decryption_share_computation(i, gamma);
        }
    }

    /// Verifies that quotient polynomial coefficients are within their expected bounds.
    ///
    /// Range checks on sk and e_sm are intentionally omitted: the DKG circuits already
    /// performed them, and commitment binding guarantees the values are unchanged.
    fn check_range_bounds(self) {
        // Check quotient polynomials are within bounds
        for basis_idx in 0..L {
            // r_1 quotients can be negative (modulus quotients)
            self.r1[basis_idx].range_check_2bounds::<BIT_R1>(
                self.configs.r1_bounds[basis_idx],
                self.configs.r1_bounds[basis_idx],
            );
            // r_2 quotients (cyclotomic quotients)
            self.r2[basis_idx].range_check_2bounds::<BIT_R2>(
                self.configs.r2_bounds[basis_idx],
                self.configs.r2_bounds[basis_idx],
            );
        }
    }

    /// Derives the Fiat-Shamir challenge gamma via the SAFE cryptographic sponge.
    ///
    /// Absorbs all inputs (via payload()) and squeezes a single challenge field element
    /// used as the evaluation point for Schwartz-Zippel verification.
    fn generate_challenge(self) -> Field {
        let inputs = self.payload();

        compute_threshold_share_decryption_challenge::<L>(inputs)
    }

    /// Verifies the decryption share equation at the Fiat-Shamir challenge point for one CRT basis.
    ///
    /// Checks that the claimed d[basis_idx] satisfies:
    ///   d(gamma) = ct0(gamma) + ct1(gamma) * sk(gamma) + e_sm(gamma) + r2(gamma) * (gamma^N + 1) + r1(gamma) * q_l
    ///
    /// By the Schwartz-Zippel lemma, if this holds at a random gamma, the underlying
    /// polynomial equation holds with high probability.
    fn verify_decryption_share_computation(self, basis_idx: u32, gamma: Field) {
        // Evaluate ciphertext components at gamma
        let c_0_at_gamma = self.ct0[basis_idx].eval(gamma);
        let c_1_at_gamma = self.ct1[basis_idx].eval(gamma);

        // Evaluate aggregated sums at gamma
        let sk_at_gamma = self.sk[basis_idx].eval(gamma);
        let e_sm_at_gamma = self.e_sm[basis_idx].eval(gamma);

        // Evaluate quotient polynomials at gamma
        let r_1_at_gamma = self.r1[basis_idx].eval(gamma);
        let r_2_at_gamma = self.r2[basis_idx].eval(gamma);

        // Evaluate cyclotomic polynomial X^N + 1 at gamma
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;

        // Compute expected decryption share using the lifted formula:
        // d_i = c_0i + c_1i * sk_i + e_sm_i + r_2i * (X^N + 1) + r_1i * q_i
        let expected_decryption_share = c_0_at_gamma
            + c_1_at_gamma * sk_at_gamma
            + e_sm_at_gamma
            + r_2_at_gamma * cyclo_at_gamma
            + r_1_at_gamma * self.configs.qis[basis_idx];

        // Evaluate the party's claimed decryption share at gamma
        let computed_decryption_share = self.d[basis_idx].eval(gamma);

        // Enforce equality: computed decryption share must match expected value
        assert(
            computed_decryption_share == expected_decryption_share,
            "Decryption share computation failed",
        );
    }
}
