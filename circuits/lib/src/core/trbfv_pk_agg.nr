// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::compute_greco_pk_agg_commitment;
use crate::math::commitments::{compute_pk_trbfv_commitment, prepare_pk_commitment_payload};
use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for TRBFV public key aggregation circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli for each basis: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

/// TRBFV Public Key Aggregation (Circuit 5).
///
/// Verifies that for each CRT basis l and each coefficient i:
/// - pk0_agg[l][i] = sum_h(pk0[h][l][i]) mod q_l
/// - pk1_agg[l][i] = sum_h(pk1[h][l][i]) mod q_l
pub struct TrbfvPublicKeyAggregation<let N: u32, let H: u32, let L: u32, let BIT_PK: u32> {
    /// Circuit parameters including CRT moduli
    configs: Configs<L>,

    /// Expected commitments to public key TRBFV (from C1)
    /// We need one comment from each honest party (H).
    expected_pk_trbfv_commitments: [Field; H],

    /// Individual public keys from H honest parties (public inputs)
    /// pk0[party_idx][basis_idx] - first component of public key for each party and CRT basis
    pk0: [[Polynomial<N>; L]; H],
    /// pk1[party_idx][basis_idx] - second component of public key for each party and CRT basis
    pk1: [[Polynomial<N>; L]; H],

    /// Claimed aggregated public key (public inputs)
    /// pk0_agg[basis_idx] - first component of aggregated public key for each CRT basis
    pk0_agg: [Polynomial<N>; L],
    /// pk1_agg[basis_idx] - second component of aggregated public key for each CRT basis
    pk1_agg: [Polynomial<N>; L],
}

impl<let N: u32, let H: u32, let L: u32, let BIT_PK: u32> TrbfvPublicKeyAggregation<N, H, L, BIT_PK> {
    pub fn new(
        configs: Configs<L>,
        expected_pk_trbfv_commitments: [Field; H],
        pk0: [[Polynomial<N>; L]; H],
        pk1: [[Polynomial<N>; L]; H],
        pk0_agg: [Polynomial<N>; L],
        pk1_agg: [Polynomial<N>; L],
    ) -> Self {
        TrbfvPublicKeyAggregation {
            configs,
            expected_pk_trbfv_commitments,
            pk0,
            pk1,
            pk0_agg,
            pk1_agg,
        }
    }

    /// Verifies that pk trbfv hashes to each expected_pk_trbfv_commitment
    fn verify_pk_commitments(self) {
        for i in 0..H {
            let payload = prepare_pk_commitment_payload::<N, L, BIT_PK>(self.pk0[i], self.pk1[i]);
            let computed_commitment = compute_pk_trbfv_commitment(payload);
            assert(
                computed_commitment == self.expected_pk_trbfv_commitments[i],
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
    /// Returns commitment to aggregated public key
    pub fn verify(self) -> Field {
        // 0. Verify pk commitments
        self.verify_pk_commitments();

        // 1. Verify pk & pk1 aggregations for each basis
        for basis_idx in 0..L {
            self.verify_pk_for_basis(self.pk0, self.pk0_agg, basis_idx);
            self.verify_pk_for_basis(self.pk1, self.pk1_agg, basis_idx);
        }

        // 2. Commit to aggregated public key
        let payload = prepare_pk_commitment_payload::<N, L, BIT_PK>(self.pk0_agg, self.pk1_agg);
        compute_greco_pk_agg_commitment(payload)
    }
}
