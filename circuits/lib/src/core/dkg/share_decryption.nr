// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_aggregated_shares_commitment, compute_share_encryption_commitment_from_message,
};
use crate::math::polynomial::Polynomial;

/// DKG Share Decryption & Aggregation Circuit (C4a / C4b).
///
/// Verifies that each ciphernode correctly decrypted the shares addressed to them
/// and honestly aggregated all honest parties' contributions into a single combined value.
///
/// This circuit runs twice in parallel:
/// - C4a: verifies and aggregates secret key (sk) shares, consuming commit(sk_share) from C2a
/// - C4b: verifies and aggregates smudging noise (e_sm) shares, consuming commit(e_sm_share) from C2b
///
/// Both variants use identical verification logic. The source of `expected_commitments`
/// is the only difference between the two instantiations.
///
/// Outputs:
/// - C4a: `commit(agg_sk)` -> C6 ([`threshold/share_decryption`](../../threshold/share_decryption))
/// - C4b: `commit(agg_e_sm)` -> C6 ([`threshold/share_decryption`](../../threshold/share_decryption))
pub struct ShareDecryption<let N: u32, let L: u32, let H: u32, let BIT_MSG: u32> {
    /// Expected commitments to the share polynomials, produced in C2a (for C4a) or C2b (for C4b)
    /// via commit_to_party_shares. Organised as [party_idx][mod_idx], covering all H honest
    /// parties and L CRT moduli.
    /// (public witnesses)
    expected_commitments: [[Field; L]; H],

    /// Decrypted share polynomials from H honest parties.
    /// Organised as [party_idx][mod_idx].
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

    /// Verifies that each decrypted share matches its expected commitment from C2a/C2b.
    ///
    /// For each honest party and each CRT modulus, hashes the decrypted share polynomial
    /// and confirms it equals the commitment produced in C2 via commit_to_party_shares.
    /// Since commitments are binding, any substitution or tampering, claiming a different
    /// decrypted value or using the wrong secret key, will cause this assertion to fail.
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

    /// Aggregates all verified shares by summing coefficient-wise across honest parties.
    ///
    /// For each CRT modulus and each coefficient position, computes:
    ///   aggregated[mod_idx][coeff] = sum(decrypted_shares[party_idx][mod_idx][coeff])
    ///                                  for party_idx in 0..H
    ///
    /// This reflects the threshold key construction: sk = sum(sk_i) for i in H.
    /// Each ciphernode now holds a share of this sum, used during threshold decryption in P4.
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

    /// Executes the share decryption and aggregation circuit.
    ///
    /// Returns a commitment to the aggregated shares:
    /// - C4a: commit(agg_sk), consumed by C6 in P4
    /// - C4b: commit(agg_e_sm), consumed by C6 in P4
    pub fn execute(self) -> Field {
        // Step 1: Verify all commitments match
        self.verify_commitments();

        // Step 2: Compute aggregated shares
        let aggregated = self.compute_aggregated_shares();

        // Step 3: Return commitment to aggregated shares
        compute_aggregated_shares_commitment::<N, L, BIT_MSG>(aggregated)
    }
}
