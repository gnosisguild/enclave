use lib::configs::default::dkg::{
    L_THRESHOLD, PARITY_MATRIX, SHARE_COMPUTATION_BIT_SHARE, SHARE_COMPUTATION_CHUNK_SIZE,
    SHARE_COMPUTATION_SK_CHUNK_CONFIGS,
};
use lib::configs::default::{N_PARTIES, T};
use lib::core::dkg::share_computation::chunk::ShareComputationChunk;

global CHUNK_IDX: u32 = 0; // change per chunk main

fn main(
    chunk_commitment: pub Field,
    y_chunk: [[[Field; N_PARTIES + 1]; L_THRESHOLD]; SHARE_COMPUTATION_CHUNK_SIZE],
) {
    let circuit: ShareComputationChunk<L_THRESHOLD, N_PARTIES, T, SHARE_COMPUTATION_BIT_SHARE, SHARE_COMPUTATION_CHUNK_SIZE, CHUNK_IDX> = ShareComputationChunk::new(
        SHARE_COMPUTATION_SK_CHUNK_CONFIGS,
        chunk_commitment,
        y_chunk,
        PARITY_MATRIX,
    );
    circuit.execute()
}
