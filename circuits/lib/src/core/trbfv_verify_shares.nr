// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_secret_commitment, compute_shares_party_modulus_commitment,
    prepare_shares_party_modulus_commitment_payload, prepare_single_polynomial_commitment_payload,
};
use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for TRBFV secret share verification circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

/// TRBFV Secret Share Verification Circuit for secret key (Circuit 2a - VERIFY SHARES SK).
///
/// Verifies:
/// 1. secret commitment: verify secret hashes to expected_secret_commitment
/// 2. secret consistency: y[i][j][0] == secret_sk[i] for all i, j
/// 3. Range check: shares are in [0, q_j)
/// 4. Parity check: H[j] * y[i][j]^T == 0 mod q_j for all i, j
///
/// For SK: secret_sk is the trinary coefficients
pub struct VerifySharesSk<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> {
    configs: Configs<L>,
    /// Expected commitment to secret (from C1)
    expected_secret_commitment: Field,
    /// Secret key polynomial: Polynomial<N>
    /// trinary coefficients
    secret_sk: Polynomial<N>,
    /// Shares: y[coeff_idx][mod_idx][0..N_PARTIES+1]
    /// y[i][j][0] = secret_sk[i] = f(0), y[i][j][k] = f(k) for k = 1..N_PARTIES
    y: [[[Field; N_PARTIES + 1]; L]; N],
    /// Parity check matrices: H[mod_idx][row][col]
    /// Size per modulus: (N_PARTIES - T) * (N_PARTIES + 1)
    /// H * y^T = 0 mod q_j
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
}

/// TRBFV Secret Share Verification Circuit for smudging noise (Circuit 2b - VERIFY SHARES E_SM).
///
/// Verifies:
/// 1. secret commitment: verify secret hashes to expected_secret_commitment
/// 2. secret consistency: y[i][j][0] == e_sm[j][i] for all i, j
/// 3. Range check: shares are in [0, q_j)
/// 4. Parity check: H[j] * y[i][j]^T == 0 mod q_j for all i, j
///
/// For ESM: e_sm[j] is the RNS representation at modulus j
pub struct VerifySharesEsm<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> {
    configs: Configs<L>,
    /// Expected commitment to secret (from C1)
    /// This is computed from the first modulus's RNS representation.
    expected_secret_commitment: Field,
    /// Smudging noise polynomial per modulus: [Polynomial<N>; L]
    /// For ESM: each modulus has its own polynomial (RNS representation)
    secret_e_sm: [Polynomial<N>; L],
    /// Shares: y[coeff_idx][mod_idx][0..N_PARTIES+1]
    /// y[i][j][0] = e_sm[j][i] = f(0), y[i][j][k] = f(k) for k = 1..N_PARTIES
    y: [[[Field; N_PARTIES + 1]; L]; N],
    /// Parity check matrices: H[mod_idx][row][col]
    /// Size per modulus: (N_PARTIES - T) * (N_PARTIES + 1)
    /// H * y^T = 0 mod q_j
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> VerifySharesSk<N, L, N_PARTIES, T, BIT_SECRET, BIT_SHARE> {
    pub fn new(
        configs: Configs<L>,
        expected_secret_commitment: Field,
        secret_sk: Polynomial<N>,
        y: [[[Field; N_PARTIES + 1]; L]; N],
        h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    ) -> Self {
        VerifySharesSk { configs, expected_secret_commitment, secret_sk, y, h }
    }

    /// Main verification function
    pub fn verify(self) -> [[Field; L]; N_PARTIES] {
        // Step 1: Verify secret commitment matches expected
        self.verify_secret_commitment();

        // Step 2: Verify secret consistency
        self.verify_secret_consistency();

        // Step 3: Range checks
        check_range_bounds::<N, L, N_PARTIES, BIT_SHARE>(self.configs.qis, self.y);

        // Step 4: Verify parity check
        verify_parity_check::<N, L, N_PARTIES, T>(self.configs.qis, self.h, self.y);

        // Step 5: Commit to shares for each party and modulus
        commit_to_party_shares::<N, L, N_PARTIES>(self.y)
    }

    /// Verifies that secret hashes to expected_secret_commitment
    fn verify_secret_commitment(self) {
        let payload = prepare_single_polynomial_commitment_payload::<N, BIT_SECRET>(self.secret_sk);
        let computed_commitment = compute_secret_commitment(payload);
        assert(computed_commitment == self.expected_secret_commitment, "SK commitment mismatch");
    }

    /// Verifies secret consistency: `y[i][j][0] == secret_sk[i]` for all i, j.
    ///
    /// This function ensures that for each coefficient i and CRT basis j, the share
    /// at party ID 0 equals the corresponding secret coefficient for that modulus.
    /// This is a fundamental property of Shamir secret sharing where the secret is the
    /// evaluation of the sharing polynomial at point 0.
    ///
    /// secret_sk is the trinary coefficients, so y[i][j][0] is the same for all j.
    ///
    /// # Panics
    /// The circuit will fail if secret consistency doesn't hold for any
    /// coefficient or CRT basis.
    fn verify_secret_consistency(self) {
        for coeff_idx in 0..N {
            let secret_coeff = self.secret_sk.coefficients[coeff_idx];

            for mod_idx in 0..L {
                assert(self.y[coeff_idx][mod_idx][0] == secret_coeff);
            }
        }
    }
}

impl<let N: u32, let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SECRET: u32, let BIT_SHARE: u32> VerifySharesEsm<N, L, N_PARTIES, T, BIT_SECRET, BIT_SHARE> {
    pub fn new(
        configs: Configs<L>,
        expected_secret_commitment: Field,
        secret_e_sm: [Polynomial<N>; L],
        y: [[[Field; N_PARTIES + 1]; L]; N],
        h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    ) -> Self {
        VerifySharesEsm { configs, expected_secret_commitment, secret_e_sm, y, h }
    }

    /// Main verification function
    pub fn verify(self) -> [[Field; L]; N_PARTIES] {
        // Step 1: Verify secret commitment matches expected
        self.verify_secret_commitment();

        // Step 2: Verify secret consistency
        self.verify_secret_consistency();

        // Step 3: Range checks
        check_range_bounds::<N, L, N_PARTIES, BIT_SHARE>(self.configs.qis, self.y);

        // Step 4: Verify parity check
        verify_parity_check::<N, L, N_PARTIES, T>(self.configs.qis, self.h, self.y);

        // Step 5: Commit to shares for each party and modulus
        commit_to_party_shares::<N, L, N_PARTIES>(self.y)
    }

    /// Verifies that secret hashes to expected_secret_commitment
    /// For commitment verification, we use the first modulus's secret polynomial
    /// (matches C1's computation which uses the first modulus's RNS representation)
    fn verify_secret_commitment(self) {
        let payload =
            prepare_single_polynomial_commitment_payload::<N, BIT_SECRET>(self.secret_e_sm[0]);
        let computed_commitment = compute_secret_commitment(payload);
        assert(computed_commitment == self.expected_secret_commitment, "ESM commitment mismatch");
    }

    /// Verifies secret consistency: `y[i][j][0] == secret_e_sm[j][i]` for all i, j.
    ///
    /// This function ensures that for each coefficient i and CRT basis j, the share
    /// at party ID 0 equals the corresponding secret coefficient for that modulus.
    /// This is a fundamental property of Shamir secret sharing where the secret is the
    /// evaluation of the sharing polynomial at point 0.
    ///
    /// secret_e_sm[j] is the RNS representation at modulus j, so y[i][j][0] varies per modulus.
    ///
    /// # Panics
    /// The circuit will fail if secret consistency doesn't hold for any
    /// coefficient or CRT basis.
    fn verify_secret_consistency(self) {
        for coeff_idx in 0..N {
            for mod_idx in 0..L {
                let secret_coeff = self.secret_e_sm[mod_idx].coefficients[coeff_idx];
                assert(
                    self.y[coeff_idx][mod_idx][0] == secret_coeff,
                    "Secret consistency check failed",
                );
            }
        }
    }
}

/// Performs range checks on secret key and share values.
///
/// This function constrains all values to be within their expected bounds:
/// - Share values for parties k >= 1 must be in [0, q_j) for each CRT modulus q_j
///
/// These bounds are critical for security and correctness of the threshold BFV scheme.
///
/// # Panics
/// This function will cause the circuit to fail if any value is outside
/// its expected bounds.
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
/// This function verifies that for each coefficient i and CRT basis j, the share
/// vector `y[i][j]` forms a valid Reed-Solomon codeword by satisfying the parity
/// check equation with the parity check matrix `H[j]`.
///
/// The parity check matrix H[j] has dimensions `(N_PARTIES - T) * (N_PARTIES + 1)`,
/// and the share vector `y[i][j]` has length `N_PARTIES + 1`. The parity check
/// ensures that any T+1 shares can correctly reconstruct the secret key via
/// Lagrange interpolation.
///
/// # Panics
/// The circuit will fail if the parity check doesn't hold for any coefficient,
/// CRT basis, or parity check row.
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

/// Commits to shares for each party and modulus
/// Returns [[Field; L]; N_PARTIES] where commitments[party_idx][mod_idx]
pub fn commit_to_party_shares<let N: u32, let L: u32, let N_PARTIES: u32>(
    y: [[[Field; N_PARTIES + 1]; L]; N],
) -> [[Field; L]; N_PARTIES] {
    let mut commitments: [[Field; L]; N_PARTIES] = [[0; L]; N_PARTIES];

    for party_idx in 0..N_PARTIES {
        for mod_idx in 0..L {
            let payload = prepare_shares_party_modulus_commitment_payload::<N, L, N_PARTIES>(
                y,
                party_idx,
                mod_idx,
            );

            commitments[party_idx][mod_idx] = compute_shares_party_modulus_commitment(payload);
        }
    }

    commitments
}
