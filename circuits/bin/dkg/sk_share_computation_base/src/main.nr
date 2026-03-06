use lib::configs::default::dkg::{
    L_THRESHOLD, N, SHARE_COMPUTATION_CHUNK_SIZE, SHARE_COMPUTATION_N_CHUNKS,
    SHARE_COMPUTATION_SK_BASE_CONFIGS, SHARE_COMPUTATION_SK_BIT_SECRET,
};
use lib::configs::default::{N_PARTIES, T};
use lib::core::dkg::share_computation::base::SecretKeyShareComputationBase;
use lib::math::polynomial::Polynomial;

fn main(
    expected_secret_commitment: pub Field,
    sk_secret: Polynomial<N>,
    y: [[[Field; N_PARTIES + 1]; L_THRESHOLD]; N],
) -> pub ([Field; SHARE_COMPUTATION_N_CHUNKS], [[Field; L_THRESHOLD]; N_PARTIES]) {
    let sk_share_computation_base: SecretKeyShareComputationBase<N, L_THRESHOLD, N_PARTIES, T, SHARE_COMPUTATION_SK_BIT_SECRET, SHARE_COMPUTATION_CHUNK_SIZE, SHARE_COMPUTATION_N_CHUNKS> = SecretKeyShareComputationBase::new(
        SHARE_COMPUTATION_SK_BASE_CONFIGS,
        expected_secret_commitment,
        sk_secret,
        y,
    );
    sk_share_computation_base.execute()
}
