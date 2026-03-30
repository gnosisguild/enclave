// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C2 chunk circuit - range + parity on a slice of the public share array `y`.

use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

/// Parameters for share computation chunk (C2).
pub struct Configs<let L: u32> {
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

/// Share computation chunk (C2).
///
/// **Role:** For a slice of coefficients, range-check Shamir shares and Reed-Solomon parity (`H`).
///
/// **Consumes:** Public `y_chunk` aligned with the C2 base `y` array.
///
/// **Produces:** None (assert-only); wrappers glue chunks to the base proof.
pub struct ShareComputationChunk<let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SHARE: u32, let CHUNK_SIZE: u32> {
    configs: Configs<L>,
    /// Slice of y for this chunk, public so wrapper can enforce
    /// consistency with base circuit via public input equality
    y_chunk: [[[Field; N_PARTIES + 1]; L]; CHUNK_SIZE],
    /// Parity check matrix
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
}

impl<let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SHARE: u32, let CHUNK_SIZE: u32> ShareComputationChunk<L, N_PARTIES, T, BIT_SHARE, CHUNK_SIZE> {

    pub fn new(
        configs: Configs<L>,
        y_chunk: [[[Field; N_PARTIES + 1]; L]; CHUNK_SIZE],
        h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    ) -> Self {
        ShareComputationChunk { configs, y_chunk, h }
    }

    /// Range checks: shares y[i][j][k] for k >= 1 must be in [0, q_j)
    /// k == 0 is the secret itself, checked in base circuit via secret consistency
    fn check_range_bounds(self) {
        for mod_idx in 0..L {
            let q_j = self.configs.qis[mod_idx];
            for coeff_idx in 0..CHUNK_SIZE {
                for party_idx in 1..(N_PARTIES + 1) {
                    Polynomial::new([self.y_chunk[coeff_idx][mod_idx][party_idx]])
                        .range_check_standard::<BIT_SHARE>(q_j);
                }
            }
        }
    }

    /// Parity check: H[j] * y[i][j]^T == 0 mod q_j for all i in chunk, j
    fn verify_parity_check(self) {
        for coeff_idx in 0..CHUNK_SIZE {
            for mod_idx in 0..L {
                let q_j = self.configs.qis[mod_idx];
                let m = ModU128::new(q_j);
                for row in 0..(N_PARTIES - T) {
                    let mut sum: Field = 0;
                    for col in 0..(N_PARTIES + 1) {
                        sum += self.h[mod_idx][row][col] * self.y_chunk[coeff_idx][mod_idx][col];
                    }
                    assert(m.reduce_mod(sum) == 0, "Parity check failed");
                }
            }
        }
    }

    pub fn execute(self) {
        // Step 1: Range-check non-secret share columns.
        self.check_range_bounds();
        // Step 2: Reed-Solomon parity on this chunk.
        self.verify_parity_check();
    }
}
