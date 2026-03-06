// Chunk circuit for C2. Verifies range checks and parity check for a chunk of
// CHUNK_SIZE coefficients. CHUNK_IDX identifies which chunk this is, used to
// verify consistency against the chunk commitment from the base circuit.

use crate::math::commitments::compute_share_computation_chunk_commitment;
use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

pub struct Configs<let L: u32> {
    pub qis: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L]) -> Self {
        Configs { qis }
    }
}

pub struct ShareComputationChunk<let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SHARE: u32, let CHUNK_SIZE: u32, let CHUNK_IDX: u32> {
    configs: Configs<L>,
    /// Public input from base circuit
    chunk_commitment: Field,
    /// Slice of y for this chunk: y[CHUNK_IDX*CHUNK_SIZE..(CHUNK_IDX+1)*CHUNK_SIZE]
    y_chunk: [[[Field; N_PARTIES + 1]; L]; CHUNK_SIZE],
    /// Parity check matrix
    h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
}

impl<let L: u32, let N_PARTIES: u32, let T: u32, let BIT_SHARE: u32, let CHUNK_SIZE: u32, let CHUNK_IDX: u32> ShareComputationChunk<L, N_PARTIES, T, BIT_SHARE, CHUNK_SIZE, CHUNK_IDX> {

    pub fn new(
        configs: Configs<L>,
        chunk_commitment: Field,
        y_chunk: [[[Field; N_PARTIES + 1]; L]; CHUNK_SIZE],
        h: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L],
    ) -> Self {
        ShareComputationChunk { configs, chunk_commitment, y_chunk, h }
    }

    /// Verifies y_chunk is consistent with chunk_commitment from base circuit.
    /// CHUNK_IDX is a compile-time generic so each chunk circuit is uniquely bound
    /// to its position, a prover cannot reuse the same chunk data in a different position.
    fn verify_chunk_commitment(self) {
        let computed = compute_share_computation_chunk_commitment::<N_PARTIES, L, CHUNK_SIZE>(
            self.y_chunk,
            CHUNK_IDX,
        );
        assert(computed == self.chunk_commitment, "chunk commitment mismatch");
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
        self.verify_chunk_commitment();
        self.check_range_bounds();
        self.verify_parity_check();
    }
}
