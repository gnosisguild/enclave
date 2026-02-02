// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::compute_pk_aggregation_commitment;
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

/// Public Key Aggregation (Circuit 5).
///
/// Verifies that for each CRT basis l and each coefficient i:
/// - pk0_agg[l][i] = sum_h(pk0[h][l][i]) mod q_l
/// - pk1_agg[l][i] = sum_h(pk1[h][l][i]) mod q_l
pub struct PkAggregation<let N: u32, let H: u32, let L: u32, let BIT_PK: u32> {
    /// Circuit parameters including CRT moduli
    configs: Configs<L>,

    /// Expected commitments to threshold public key (from C1)
    /// We need one commitment from each honest party (H).
    /// (public witness)
    expected_threshold_pk_commitments: [Field; H],

    /// Individual public keys from H honest parties
    /// pk0[party_idx][basis_idx] - first component of public key for each party and CRT basis
    /// (committed witnesses)
    pk0: [[Polynomial<N>; L]; H],
    /// pk1[party_idx][basis_idx] - second component of public key for each party and CRT basis
    /// (committed witnesses)
    pk1: [[Polynomial<N>; L]; H],

    /// Claimed aggregated public key
    /// pk0_agg[basis_idx] - first component of aggregated public key for each CRT basis
    /// (committed witnesses)
    pk0_agg: [Polynomial<N>; L],
    /// pk1_agg[basis_idx] - second component of aggregated public key for each CRT basis
    /// (committed witnesses)
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

    /// Verifies that pk hashes to each expected_threshold_pk_commitment
    fn verify_pk_commitments(self) {
        for i in 0..H {
            assert(
                compute_pk_aggregation_commitment::<N, L, BIT_PK>(self.pk0[i], self.pk1[i])
                    == self.expected_threshold_pk_commitments[i],
                "PK commitment mismatch",
            );
        }
    }

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

    /// Main verification function
    /// Returns commitment to aggregated threshold public key
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
