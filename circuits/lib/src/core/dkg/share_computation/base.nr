// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C2 base circuits - SK and e_sm share-array verification (`SecretKeyShareComputationBase`,
//! `SmudgingNoiseShareComputationBase`).

use crate::math::commitments::{
    compute_share_computation_e_sm_commitment, compute_share_computation_sk_commitment,
    compute_share_encryption_commitment_from_shares,
};
use crate::math::polynomial::Polynomial;

// --- C2a: SK secret shares ---

/// C2a base - bind `sk_secret` and public share array `y` to C1; emit per-party commitments for C3.
///
/// **Role:** `y[coeff][mod][0]` matches `sk_secret`; party columns hash to C3-facing commitments.
///
/// **Consumes:** `expected_secret_commitment` from C1 (sk branch).
///
/// **Produces:** `[[Field; L]; N_PARTIES]` party commitments for C3.
pub struct SecretKeyShareComputationBase<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> {
    /// Expected commitment to sk secret (from C1, public input)
    expected_secret_commitment: Field,
    /// Secret key polynomial
    sk_secret: Polynomial<N>,
    /// Public share array `y[coeff][mod][party]`; wrappers/chunks enforce equality.
    y: [[[Field; N_PARTIES + 1]; L]; N],
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> SecretKeyShareComputationBase<N, L, N_PARTIES, T, BIT_SECRET> {

    pub fn new(
        expected_secret_commitment: Field,
        sk_secret: Polynomial<N>,
        y: [[[Field; N_PARTIES + 1]; L]; N],
    ) -> Self {
        SecretKeyShareComputationBase { expected_secret_commitment, sk_secret, y }
    }

    fn verify_secret_commitment(self) {
        assert(
            compute_share_computation_sk_commitment::<N, BIT_SECRET>(self.sk_secret)
                == self.expected_secret_commitment,
            "SK commitment mismatch",
        );
    }

    /// Verifies y[i][j][0] == sk_secret[i] for all i, j
    fn verify_secret_consistency(self) {
        for coeff_idx in 0..N {
            let secret_coeff = self.sk_secret.coefficients[coeff_idx];
            for mod_idx in 0..L {
                assert(
                    self.y[coeff_idx][mod_idx][0] == secret_coeff,
                    "SK secret consistency check failed",
                );
            }
        }
    }

    fn compute_party_commitments(self) -> [[Field; L]; N_PARTIES] {
        let mut party_commitments: [[Field; L]; N_PARTIES] = [[0; L]; N_PARTIES];
        for party_idx in 0..N_PARTIES {
            for mod_idx in 0..L {
                party_commitments[party_idx][mod_idx] = compute_share_encryption_commitment_from_shares::<N, L, N_PARTIES>(
                    self.y,
                    party_idx,
                    mod_idx,
                );
            }
        }
        party_commitments
    }

    /// Returns per-party commitments for C3.
    pub fn execute(self) -> [[Field; L]; N_PARTIES] {
        // Step 1: Bind `sk_secret` to C1.
        self.verify_secret_commitment();
        // Step 2: Align `y[..][..][0]` with `sk_secret`.
        self.verify_secret_consistency();
        // Step 3: Hash party columns for C3.
        self.compute_party_commitments()
    }
}

// --- C2b: e_sm shares ---

/// C2b base - bind `e_sm_secret` and public share array `y` to C1; emit per-party commitments for C3.
///
/// **Role:** Same layout as C2a, for the smudging-noise branch.
///
/// **Consumes:** `expected_secret_commitment` from C1 (e_sm branch).
///
/// **Produces:** `[[Field; L]; N_PARTIES]` party commitments for C3.
pub struct SmudgingNoiseShareComputationBase<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> {
    /// Expected commitment to e_sm secret (from C1, public input)
    expected_secret_commitment: Field,
    /// Smudging noise polynomial per modulus
    e_sm_secret: [Polynomial<N>; L],
    /// Public share array `y[coeff][mod][party]`; wrappers/chunks enforce equality.
    y: [[[Field; N_PARTIES + 1]; L]; N],
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> SmudgingNoiseShareComputationBase<N, L, N_PARTIES, T, BIT_SECRET> {

    pub fn new(
        expected_secret_commitment: Field,
        e_sm_secret: [Polynomial<N>; L],
        y: [[[Field; N_PARTIES + 1]; L]; N],
    ) -> Self {
        SmudgingNoiseShareComputationBase { expected_secret_commitment, e_sm_secret, y }
    }

    fn verify_secret_commitment(self) {
        assert(
            compute_share_computation_e_sm_commitment::<N, L, BIT_SECRET>(self.e_sm_secret)
                == self.expected_secret_commitment,
            "ESM commitment mismatch",
        );
    }

    /// Verifies y[i][j][0] == e_sm_secret[j][i] for all i, j
    fn verify_secret_consistency(self) {
        for coeff_idx in 0..N {
            for mod_idx in 0..L {
                let secret_coeff = self.e_sm_secret[mod_idx].coefficients[coeff_idx];
                assert(
                    self.y[coeff_idx][mod_idx][0] == secret_coeff,
                    "ESM secret consistency check failed",
                );
            }
        }
    }

    fn compute_party_commitments(self) -> [[Field; L]; N_PARTIES] {
        let mut party_commitments: [[Field; L]; N_PARTIES] = [[0; L]; N_PARTIES];
        for party_idx in 0..N_PARTIES {
            for mod_idx in 0..L {
                party_commitments[party_idx][mod_idx] = compute_share_encryption_commitment_from_shares::<N, L, N_PARTIES>(
                    self.y,
                    party_idx,
                    mod_idx,
                );
            }
        }
        party_commitments
    }

    /// Returns per-party commitments for C3.
    pub fn execute(self) -> [[Field; L]; N_PARTIES] {
        // Step 1: Bind `e_sm_secret` to C1.
        self.verify_secret_commitment();
        // Step 2: Align `y[..][..][0]` with `e_sm_secret` per modulus.
        self.verify_secret_consistency();
        // Step 3: Hash party columns for C3.
        self.compute_party_commitments()
    }
}
