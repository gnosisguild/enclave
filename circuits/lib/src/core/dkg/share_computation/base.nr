// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Base circuit for C2 (SecretKeyShareComputation and SmudgingNoiseShareComputation).
// Verifies secret commitment and consistency, and outputs party commitments for
// downstream use in C3/C4. y is public to allow the wrapper to enforce consistency
// with chunk circuits without hashing overhead.

use crate::math::commitments::{
    compute_share_computation_e_sm_commitment, compute_share_computation_sk_commitment,
    compute_share_encryption_commitment_from_shares,
};
use crate::math::polynomial::Polynomial;

pub struct Configs<let L: u32> {
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

// ===== BASE CIRCUIT FOR SK =====

pub struct SecretKeyShareComputationBase<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> {
    configs: Configs<L>,
    /// Expected commitment to sk secret (from C1, public input)
    expected_secret_commitment: Field,
    /// Secret key polynomial
    sk_secret: Polynomial<N>,
    /// Full shares array y[coeff_idx][mod_idx][party_idx]
    /// Public so wrapper can enforce consistency with chunk circuits
    y: [[[Field; N_PARTIES + 1]; L]; N],
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> SecretKeyShareComputationBase<N, L, N_PARTIES, T, BIT_SECRET> {

    pub fn new(
        configs: Configs<L>,
        expected_secret_commitment: Field,
        sk_secret: Polynomial<N>,
        y: [[[Field; N_PARTIES + 1]; L]; N],
    ) -> Self {
        SecretKeyShareComputationBase { configs, expected_secret_commitment, sk_secret, y }
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

    /// Returns party_commitments[party_idx][mod_idx] for downstream use in C3/C4
    pub fn execute(self) -> [[Field; L]; N_PARTIES] {
        self.verify_secret_commitment();
        self.verify_secret_consistency();
        self.compute_party_commitments()
    }
}

// ===== BASE CIRCUIT FOR ESM =====

pub struct SmudgingNoiseShareComputationBase<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> {
    configs: Configs<L>,
    /// Expected commitment to e_sm secret (from C1, public input)
    expected_secret_commitment: Field,
    /// Smudging noise polynomial per modulus
    e_sm_secret: [Polynomial<N>; L],
    /// Full shares array y[coeff_idx][mod_idx][party_idx]
    /// Public so wrapper can enforce consistency with chunk circuits
    y: [[[Field; N_PARTIES + 1]; L]; N],
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32> SmudgingNoiseShareComputationBase<N, L, N_PARTIES, T, BIT_SECRET> {

    pub fn new(
        configs: Configs<L>,
        expected_secret_commitment: Field,
        e_sm_secret: [Polynomial<N>; L],
        y: [[[Field; N_PARTIES + 1]; L]; N],
    ) -> Self {
        SmudgingNoiseShareComputationBase { configs, expected_secret_commitment, e_sm_secret, y }
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

    /// Returns party_commitments[party_idx][mod_idx] for downstream use in C3/C4
    pub fn execute(self) -> [[Field; L]; N_PARTIES] {
        self.verify_secret_commitment();
        self.verify_secret_consistency();
        self.compute_party_commitments()
    }
}
