// Level 2: final_wrapper
use bb_proof_verification::{UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof};
use lib::configs::default::dkg::{L_THRESHOLD, SHARE_COMPUTATION_N_BATCHES as N_BATCHES};
use lib::configs::default::N_PARTIES;
use lib::math::commitments::compute_recursive_aggregation_commitment;
pub global PARTY_COMMITMENTS_SIZE: u32 = N_PARTIES * L_THRESHOLD;
pub global BATCH_WRAPPER_PUBLIC_INPUTS: u32 = PARTY_COMMITMENTS_SIZE + 1; // party_commitments + aggregated commitment

fn main(
    batch_verification_key: UltraHonkVerificationKey,
    batch_proofs: [UltraHonkZKProof; N_BATCHES],
    batch_public_inputs: [[Field; BATCH_WRAPPER_PUBLIC_INPUTS]; N_BATCHES],
    batch_key_hash: pub Field,
) -> pub Field {
    for i in 0..N_BATCHES {
        verify_honk_proof(
            batch_verification_key,
            batch_proofs[i],
            batch_public_inputs[i],
            batch_key_hash,
        );
    }

    let mut aggregated_public_inputs = Vec::new();
    for i in 0..N_BATCHES {
        for j in 0..BATCH_WRAPPER_PUBLIC_INPUTS {
            aggregated_public_inputs.push(batch_public_inputs[i][j]);
        }
    }

    compute_recursive_aggregation_commitment(aggregated_public_inputs)
}
