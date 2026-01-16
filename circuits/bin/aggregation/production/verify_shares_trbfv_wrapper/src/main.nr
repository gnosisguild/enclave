use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::math::commitments::compute_aggregation_commitment;

// Number of proofs.
pub global N_PROOFS: u32 = 2;
/// Number of commitments per proof.
pub global N_COMMITMENTS: u32 = 20;

fn main(
    verification_key: UltraHonkVerificationKey,
    proofs: [UltraHonkProof; N_PROOFS],
    commitments: pub [[Field; N_COMMITMENTS]; N_PROOFS],
    key_hash: Field,
) -> pub Field {
    for i in 0..N_PROOFS {
        verify_honk_proof_non_zk(verification_key, proofs[i], commitments[i], key_hash);
    }

    let mut aggregated_commitments = Vec::new();

    for i in 0..N_PROOFS {
        for j in 0..N_COMMITMENTS {
            aggregated_commitments.push(commitments[i][j]);
        }
    }

    compute_aggregation_commitment(aggregated_commitments)
}
