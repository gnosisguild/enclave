// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::configs::default::dkg::{
    L_THRESHOLD, PARITY_MATRIX, SHARE_COMPUTATION_BIT_SHARE, SHARE_COMPUTATION_CHUNK_CONFIGS,
    SHARE_COMPUTATION_CHUNK_SIZE,
};
use lib::configs::default::{N_PARTIES, T};
use lib::core::dkg::share_computation::chunk::ShareComputationChunk;

fn main(
    // y_chunk is public so wrapper can enforce consistency with base circuit
    y_chunk: pub [[[Field; N_PARTIES + 1]; L_THRESHOLD]; SHARE_COMPUTATION_CHUNK_SIZE],
) {
    let circuit: ShareComputationChunk<L_THRESHOLD, N_PARTIES, T, SHARE_COMPUTATION_BIT_SHARE, SHARE_COMPUTATION_CHUNK_SIZE> =
        ShareComputationChunk::new(SHARE_COMPUTATION_CHUNK_CONFIGS, y_chunk, PARITY_MATRIX);
    circuit.execute()
}
