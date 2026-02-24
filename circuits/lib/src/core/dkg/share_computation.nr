// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_share_computation_e_sm_commitment, compute_share_computation_sk_commitment,
    compute_share_encryption_commitment_from_shares,
};
use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for Threshold secret share verification circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

/// Secret Key Share Computation Circuit (Circuit C2a).
///
/// Verifies that secret key shares were correctly computed using Shamir Secret Sharing.
/// This circuit proves that shares are consistent with the committed secret key from C1,
/// satisfy range bounds, and form a valid Reed-Solomon codeword (ensuring proper threshold
/// reconstruction properties).
///
/// Outputs:
/// - commit(shares[party_idx][mod_idx]) -> C3a (share encryption)
pub struct SecretKeyShareComputation<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> {
    configs: Configs<L>,
    /// Expected commitment to secret (from C1)
    /// (public witness)
    expected_secret_commitment: Field,
    /// Secret key polynomial: Polynomial<N>
    /// trinary coefficients
    /// (secret witness)
    sk_secret: Polynomial<N>,
    /// Share matrix: y[coeff_idx][mod_idx][party_idx].
    /// - y[i][j][0] = secret (evaluation at point 0)
    /// - y[i][j][k] = share for party k (evaluation at point k, k=1..N_PARTIES)
    /// (secret witnesses)
    y: [[[Field; N_PARTIES + 1]; L]; N],
    /// Parity check matrices for Reed-Solomon verification.
    /// H[mod_idx] has dimensions (N_PARTIES - T) x (N_PARTIES + 1).
    /// (public constants, verified in P0)
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
}

/// Smudging Noise Share Computation Circuit (Circuit C2b).
///
/// Verifies that smudging noise shares were correctly computed using Shamir Secret Sharing.
/// Identical verification logic to C2a, but operates on smudging noise `e_sm` which has
/// different structure (CRT representation, one polynomial per modulus).
///
/// Outputs:
/// - commit(shares[party_idx][mod_idx]) -> C3b (share encryption)
pub struct SmudgingNoiseShareComputation<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> {
    configs: Configs<L>,
    /// Expected commitment to secret (from C1)
    /// This is computed from all L RNS polynomials (matching
    /// multiple_polynomial_payload's behavior which hashes all L modulus polynomials)
    expected_secret_commitment: Field,
    /// Smudging noise polynomial per modulus: [Polynomial<N>; L]
    /// For ESM: each modulus has its own polynomial (RNS representation)
    e_sm_secret: [Polynomial<N>; L],
    /// Share matrix: y[coeff_idx][mod_idx][party_idx].
    /// - y[i][j][0] = e_sm_secret[j][i] (varies per modulus due to CRT)
    /// - y[i][j][k] = share for party k
    /// (secret witnesses)
    y: [[[Field; N_PARTIES + 1]; L]; N],
    /// Parity check matrices for Reed-Solomon verification.
    /// H[mod_idx] has dimensions (N_PARTIES - T) x (N_PARTIES + 1).
    /// (public constants, verified in P0)
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> SecretKeyShareComputation<N, L, N_PARTIES, T, BIT_SECRET, BIT_SHARE> {
    pub fn new(
        configs: Configs<L>,
        expected_secret_commitment: Field,
        sk_secret: Polynomial<N>,
        y: [[[Field; N_PARTIES + 1]; L]; N],
        h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    ) -> Self {
        SecretKeyShareComputation { configs, expected_secret_commitment, sk_secret, y, h }
    }

    /// Executes the secret key share computation circuit.
    ///
    /// Returns commitments to shares for each party and modulus:
    /// `[[Field; L]; N_PARTIES]` where `commitments[party_idx][mod_idx]`
    /// is the commitment to party `party_idx`'s share for modulus `mod_idx`.
    ///
    /// These commitments are used in C3a to verify that shares are encrypted correctly.
    pub fn execute(self) -> [[Field; L]; N_PARTIES] {
        // Step 1: Verify secret commitment matches expected
        self.verify_secret_commitment();

        // Step 2: Verify secret consistency
        self.verify_secret_consistency();

        // Step 3: Range checks
        check_range_bounds::<N, L, N_PARTIES, BIT_SHARE>(self.configs.qis, self.y);

        // Step 4: Verify Reed-Solomon parity check
        verify_parity_check::<N, L, N_PARTIES, T>(self.configs.qis, self.h, self.y);

        // Step 5: Commit to shares for each party and modulus
        commit_to_party_shares::<N, L, N_PARTIES>(self.y)
    }

    /// Verifies that the secret key hashes to the expected commitment from C1.
    ///
    /// This links C2a to C1, ensuring we're verifying shares of the correct secret.
    fn verify_secret_commitment(self) {
        assert(
            compute_share_computation_sk_commitment::<N, BIT_SECRET>(self.sk_secret)
                == self.expected_secret_commitment,
            "SK commitment mismatch",
        );
    }

    /// Verifies secret consistency: `y[i][j][0] == sk_secret[i]` for all i, j.
    ///
    /// This enforces the fundamental Shamir Secret Sharing property: the secret is
    /// the evaluation of the sharing polynomial at point 0. For each coefficient i
    /// and CRT basis j, the share at party ID 0 must equal the secret coefficient.
    ///
    /// Since `sk_secret` is trinary (uniform across moduli), `y[i][j][0]` is the
    /// same for all j.
    fn verify_secret_consistency(self) {
        for coeff_idx in 0..N {
            let secret_coeff = self.sk_secret.coefficients[coeff_idx];

            for mod_idx in 0..L {
                assert(self.y[coeff_idx][mod_idx][0] == secret_coeff);
            }
        }
    }
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> SmudgingNoiseShareComputation<N, L, N_PARTIES, T, BIT_SECRET, BIT_SHARE> {
    pub fn new(
        configs: Configs<L>,
        expected_secret_commitment: Field,
        e_sm_secret: [Polynomial<N>; L],
        y: [[[Field; N_PARTIES + 1]; L]; N],
        h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    ) -> Self {
        SmudgingNoiseShareComputation { configs, expected_secret_commitment, e_sm_secret, y, h }
    }

    /// Executes the smudging noise share computation circuit.
    ///
    /// Returns commitments to shares for each party and modulus:
    /// `[[Field; L]; N_PARTIES]` where `commitments[party_idx][mod_idx]`
    /// is the commitment to party `party_idx`'s noise share for modulus `mod_idx`.
    ///
    /// These commitments are used in C3b to verify that noise shares are encrypted correctly.
    pub fn execute(self) -> [[Field; L]; N_PARTIES] {
        // Step 1: Verify secret commitment matches expected
        self.verify_secret_commitment();

        // Step 2: Verify secret consistency
        self.verify_secret_consistency();

        // Step 3: Range checks
        check_range_bounds::<N, L, N_PARTIES, BIT_SHARE>(self.configs.qis, self.y);

        // Step 4: Verify Reed-Solomon parity check
        verify_parity_check::<N, L, N_PARTIES, T>(self.configs.qis, self.h, self.y);

        // Step 5: Commit to shares for each party and modulus
        commit_to_party_shares::<N, L, N_PARTIES>(self.y)
    }

    /// Verifies that the smudging noise hashes to the expected commitment from C1.
    ///
    /// The commitment is computed over all L RNS polynomials, linking C2b to C1.
    fn verify_secret_commitment(self) {
        assert(
            compute_share_computation_e_sm_commitment::<N, L, BIT_SECRET>(self.e_sm_secret)
                == self.expected_secret_commitment,
            "ESM commitment mismatch",
        );
    }

    /// Verifies secret consistency: `y[i][j][0] == e_sm_secret[j][i]` for all i, j.
    ///
    /// This enforces the Shamir Secret Sharing property for CRT-represented secrets.
    /// Since `e_sm_secret[j]` is the RNS representation at modulus j, the value
    /// `y[i][j][0]` varies per modulus (unlike C2a where it's uniform).
    fn verify_secret_consistency(self) {
        for coeff_idx in 0..N {
            for mod_idx in 0..L {
                let secret_coeff = self.e_sm_secret[mod_idx].coefficients[coeff_idx];
                assert(
                    self.y[coeff_idx][mod_idx][0] == secret_coeff,
                    "Secret consistency check failed",
                );
            }
        }
    }
}

/// Performs range checks on share values.
///
/// Verifies that all shares `y[i][j][k]` for `k >= 1` (party shares, not the secret at `k=0`)
/// are in the valid range [0, q_j) for each CRT modulus q_j.
///
/// These bounds are critical for security-shares outside this range could:
/// - Leak information about the secret
/// - Break the reconstruction process via Lagrange interpolation
/// - Violate the information-theoretic security of Shamir Secret Sharing
pub fn check_range_bounds<let N: u32, let L: u32, let N_PARTIES: u32, let BIT_SHARE: u32>(
    qis: [Field; L],
    y: [[[Field; N_PARTIES + 1]; L]; N],
) {
    // Shares y[i][j][k] for k >= 1 should be in [0, q_j)
    for mod_idx in 0..L {
        let q_j = qis[mod_idx];

        for coeff_idx in 0..N {
            for party_idx in 1..(N_PARTIES + 1) {
                // Use range_check_standard from Polynomial by creating a single-coefficient polynomial
                Polynomial::new([y[coeff_idx][mod_idx][party_idx]])
                    .range_check_standard::<BIT_SHARE>(q_j);
            }
        }
    }
}

/// Verifies Reed-Solomon parity check: `H[j] * y[i][j]^T == 0 mod q_j` for all i, j.
///
/// This is the cryptographic core of share verification. The parity check matrix H[j]
/// is constructed (and verified in P0) such that valid Shamir shares must satisfy this
/// linear equation. This guarantees two essential properties:
///
/// 1. **Reconstruction**: Any T+1 shares can correctly recover the secret via Lagrange interpolation
/// 2. **Security**: Any T or fewer shares reveal no information about the secret (information-theoretic)
///
/// The parity check matrix H[j] has dimensions (N_PARTIES - T) x (N_PARTIES + 1), and
/// the share vector y[i][j] has length N_PARTIES + 1. The verification is performed
/// coefficient-wise and modulus-wise.
///
/// For Shamir Secret Sharing with threshold T, shares are evaluations of a degree-T
/// polynomial. The Reed-Solomon parity check verifies that the shares form a valid
/// codeword-equivalently, that they lie on a degree-T polynomial-via the parity check matrix H[j].
pub fn verify_parity_check<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32>(
    qis: [Field; L],
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    y: [[[Field; N_PARTIES + 1]; L]; N],
) {
    for coeff_idx in 0..N {
        for mod_idx in 0..L {
            let q_j = qis[mod_idx];

            // For each row of H, compute dot product with y and verify == 0
            for row in 0..(N_PARTIES - T) {
                let mut sum: Field = 0;

                for col in 0..(N_PARTIES + 1) {
                    sum = sum + h[mod_idx][row][col] * y[coeff_idx][mod_idx][col];
                }

                // Reduce mod q_j and verify == 0
                let m = ModU128::new(q_j);
                let result = m.reduce_mod(sum);
                assert(result == 0, "Parity check failed");
            }
        }
    }
}

/// Commits to shares for each party and modulus.
///
/// For each party and each CRT modulus, computes a commitment to that party's shares
/// across all polynomial coefficients. These commitments link C2a/C2b to C3a/C3b,
/// ensuring that the shares encrypted in the next phase are the same shares verified here.
pub fn commit_to_party_shares<let N: u32, let L: u32, let N_PARTIES: u32>(
    y: [[[Field; N_PARTIES + 1]; L]; N],
) -> [[Field; L]; N_PARTIES] {
    let mut commitments: [[Field; L]; N_PARTIES] = [[0; L]; N_PARTIES];

    for party_idx in 0..N_PARTIES {
        for mod_idx in 0..L {
            commitments[party_idx][mod_idx] = compute_share_encryption_commitment_from_shares::<N, L, N_PARTIES>(
                y,
                party_idx,
                mod_idx,
            );
        }
    }

    commitments
}
