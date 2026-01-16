use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::math::commitments::compute_aggregation_commitment;

fn main(
    verification_key: UltraHonkVerificationKey,
    proofs: [UltraHonkProof; 2],
    commitments: pub [Field; 2],
    key_hash: Field,
) -> pub Field {
    verify_honk_proof_non_zk(verification_key, proofs[0], [commitments[0]], key_hash);
    verify_honk_proof_non_zk(verification_key, proofs[1], [commitments[1]], key_hash);

    let mut commitments_vec = Vec::new();

    commitments_vec.push(commitments[0]);
    commitments_vec.push(commitments[1]);

    compute_aggregation_commitment(commitments_vec)
}
