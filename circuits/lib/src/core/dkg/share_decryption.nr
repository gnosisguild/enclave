// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_aggregated_shares_commitment, compute_share_encryption_commitment_from_message,
};
use crate::math::polynomial::Polynomial;

/// DKG share decryption and aggregation (C4a / C4b).
///
/// **Role:** Verify decrypted shares against C2 commitments and publish a commitment to the sum
/// over honest parties (run twice: sk path and e_sm path).
///
/// **Consumes:** Per-party share commitments from C2 (sk or e_sm branch).
///
/// **Produces:**
/// - C4a: `commit(agg_sk)` -> C6 (threshold share decryption).
/// - C4b: `commit(agg_e_sm)` -> C6 (threshold share decryption).
///
/// `BIT_MSG` hashes per-share plaintexts (aligned with C2). `BIT_AGG` hashes the CRT aggregate
/// (`compute_aggregated_shares_commitment`) and must match threshold C6 `BIT_SK` / `BIT_E_SM`.
pub struct ShareDecryption<let N: u32, let L: u32, let H: u32, let BIT_MSG: u32, let BIT_AGG: u32> {
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

impl<let N: u32, let L: u32, let H: u32, let BIT_MSG: u32, let BIT_AGG: u32> ShareDecryption<N, L, H, BIT_MSG, BIT_AGG> {
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
    ///   aggregated[mod_idx][coeff] = sum(decrypted_shares[party_idx][mod_idx][coeff]) mod q_l
    ///                                  for party_idx in 0..H
    ///
    /// Note: explicit modular reduction is omitted here because shares are verified to be in
    /// [0, q_l) by C2 range checks, and the sum fits in the circuit field. The modular
    /// reduction is enforced implicitly via the quotient polynomials in the downstream C6 circuit.
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
        // Step 1: Verify decrypted shares match expected C2 commitments.
        self.verify_commitments();

        // Step 2: Sum shares coefficient-wise per CRT limb.
        let aggregated = self.compute_aggregated_shares();

        // Step 3: Publish commitment to the aggregate (C6 input).
        compute_aggregated_shares_commitment::<N, L, BIT_AGG>(aggregated)
    }
}
