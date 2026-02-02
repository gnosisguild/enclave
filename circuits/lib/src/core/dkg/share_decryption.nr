// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_aggregated_shares_commitment, compute_share_encryption_commitment_from_message,
};
use crate::math::polynomial::Polynomial;

/// Share Decryption Commitment Verification (Circuit 4).
///
/// Verifies:
/// 1. Each decrypted share from H honest parties matches its commitment from Circuit 3
/// 2. Computes sum of all shares
/// 3. Returns commitment to aggregated shares
pub struct ShareDecryption<let N: u32, let L: u32, let H: u32, let BIT_MSG: u32> {
    /// Expected commitments from Circuit 3 for H honest parties: [party_idx][mod_idx]
    /// (public witness)
    expected_commitments: [[Field; L]; H],

    /// Decrypted shares from H honest parties: [party_idx][mod_idx]
    /// (secret witnesses)
    decrypted_shares: [[Polynomial<N>; L]; H],
}

impl<let N: u32, let L: u32, let H: u32, let BIT_MSG: u32> ShareDecryption<N, L, H, BIT_MSG> {
    pub fn new(
        expected_commitments: [[Field; L]; H],
        decrypted_shares: [[Polynomial<N>; L]; H],
    ) -> Self {
        ShareDecryption { expected_commitments, decrypted_shares }
    }

    /// Verifies all decrypted shares match their expected commitments
    fn verify_commitments(self) {
        for party_idx in 0..H {
            for mod_idx in 0..L {
                assert(
                    compute_share_encryption_commitment_from_message::<N, BIT_MSG>(
                        self.decrypted_shares[party_idx][mod_idx],
                    )
                        == self.expected_commitments[party_idx][mod_idx],
                    "Commitment mismatch",
                );
            }
        }
    }

    /// Computes sum of all decrypted shares
    fn compute_aggregated_shares(self) -> [Polynomial<N>; L] {
        let mut sum: [Polynomial<N>; L] = [Polynomial::new([0; N]); L];

        for mod_idx in 0..L {
            let mut coeffs = [0 as Field; N];
            for coeff_idx in 0..N {
                let mut total = 0 as Field;
                for party_idx in 0..H {
                    total =
                        total + self.decrypted_shares[party_idx][mod_idx].coefficients[coeff_idx];
                }
                coeffs[coeff_idx] = total;
            }
            sum[mod_idx] = Polynomial::new(coeffs);
        }

        sum
    }

    /// Main verification function
    /// Returns commitment to aggregated shares
    pub fn execute(self) -> Field {
        // Step 1: Verify all commitments match
        self.verify_commitments();

        // Step 2: Compute aggregated shares
        let aggregated = self.compute_aggregated_shares();

        // Step 3: Return commitment to aggregated shares
        compute_aggregated_shares_commitment::<N, L, BIT_MSG>(aggregated)
    }
}
