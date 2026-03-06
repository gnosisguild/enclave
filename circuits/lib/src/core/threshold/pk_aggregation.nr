// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{compute_pk_aggregation_commitment, compute_threshold_pk_commitment};
use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for Threshold public key aggregation circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli for each basis: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

/// Public Key Aggregation Circuit (C5).
///
/// Combines the threshold BFV public key shares from all H honest ciphernodes into
/// a single aggregated public key. This is the output of P2 (Aggregation) and the
/// input to P3 (User Encryption).
///
/// The additive structure of threshold BFV means the aggregated public key is simply
/// the component-wise sum of individual shares modulo each CRT modulus. The
/// corresponding threshold secret key is the sum of individual secret key contributions,
/// enabling threshold decryption via Lagrange interpolation in P4 without ever
/// reconstructing the full secret key.
///
/// Outputs:
/// - commit(pk_agg) -> consumed by the user-data-encryption circuit in P3.
pub struct PkAggregation<let N: u32, let H: u32, let L: u32, let BIT_PK: u32> {
    /// Circuit parameters including CRT moduli
    configs: Configs<L>,

    /// Expected commitments to each honest party's threshold public key share.
    /// commit(pk_trbfv[h]) produced by C1 for each h in H.
    /// (public witnesses)
    expected_threshold_pk_commitments: [Field; H],

    /// Individual threshold public key first components from H honest parties.
    /// pk0[party_idx][basis_idx] for each party and CRT basis.
    /// (committed witnesses)
    pk0: [[Polynomial<N>; L]; H],
    /// Individual threshold public key second components from H honest parties.
    /// pk1[party_idx][basis_idx] for each party and CRT basis.
    /// (committed witnesses)
    pk1: [[Polynomial<N>; L]; H],

    /// Claimed aggregated public key first component for each CRT basis.
    /// Must equal sum(pk0[h][l]) mod q_l for each basis l.
    /// (committed witness)
    pk0_agg: [Polynomial<N>; L],
    /// Claimed aggregated public key second component for each CRT basis.
    /// Must equal sum(pk1[h][l]) mod q_l for each basis l.
    /// (committed witness)
    pk1_agg: [Polynomial<N>; L],
}

impl<let N: u32, let H: u32, let L: u32, let BIT_PK: u32> PkAggregation<N, H, L, BIT_PK> {
    pub fn new(
        configs: Configs<L>,
        expected_threshold_pk_commitments: [Field; H],
        pk0: [[Polynomial<N>; L]; H],
        pk1: [[Polynomial<N>; L]; H],
        pk0_agg: [Polynomial<N>; L],
        pk1_agg: [Polynomial<N>; L],
    ) -> Self {
        PkAggregation { configs, expected_threshold_pk_commitments, pk0, pk1, pk0_agg, pk1_agg }
    }

    /// Verifies that each honest party's public key hashes to its expected commitment from C1.
    ///
    /// Ensures that only correctly generated public key shares, as verified during
    /// key generation in C1, are included in the aggregation. Any substitution of
    /// a different key will cause this assertion to fail.
    fn verify_pk_commitments(self) {
        for i in 0..H {
            assert(
                compute_threshold_pk_commitment::<N, L, BIT_PK>(self.pk0[i], self.pk1[i])
                    == self.expected_threshold_pk_commitments[i],
                "PK commitment mismatch",
            );
        }
    }

    /// Verifies aggregation correctness for one component (pk0 or pk1) at a given CRT basis.
    ///
    /// For each coefficient position i, checks:
    ///   pk_agg[basis_idx][i] = sum(pk[h][basis_idx][i]) mod q_l  for h in 0..H
    ///
    /// The modular reduction ensures the aggregated coefficients stay within
    /// the valid range for the CRT modulus q_l.
    fn verify_pk_for_basis(
        self,
        pk: [[Polynomial<N>; L]; H],
        pk_agg: [Polynomial<N>; L],
        basis_idx: u32,
    ) {
        let q_l = self.configs.qis[basis_idx];
        let mod_q_l = ModU128::new(q_l);

        for coeff_idx in 0..N {
            // Sum pk coefficients from all honest parties
            let mut sum_pk: Field = 0;
            for party_idx in 0..H {
                sum_pk = sum_pk + pk[party_idx][basis_idx].coefficients[coeff_idx];
            }

            // Reduce mod q_l
            let sum_pk_reduced = mod_q_l.reduce_mod(sum_pk);

            // Verify equality
            assert(
                sum_pk_reduced == pk_agg[basis_idx].coefficients[coeff_idx],
                "pk aggregation mismatch",
            );
        }
    }

    /// Executes the public key aggregation circuit.
    ///
    /// Returns `commit(pk_agg)`, a commitment to the aggregated threshold public key,
    /// posted on-chain and consumed by the user-data-encryption circuit in P3.
    pub fn execute(self) -> Field {
        // 0. Verify pk commitments
        self.verify_pk_commitments();

        // 1. Verify pk0 & pk1 aggregations for each CRT basis
        for basis_idx in 0..L {
            self.verify_pk_for_basis(self.pk0, self.pk0_agg, basis_idx);
            self.verify_pk_for_basis(self.pk1, self.pk1_agg, basis_idx);
        }

        // 2. Commit to aggregated threshold public key
        compute_pk_aggregation_commitment::<N, L, BIT_PK>(self.pk0_agg, self.pk1_agg)
    }
}
