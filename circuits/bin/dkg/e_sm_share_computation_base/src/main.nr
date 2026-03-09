use lib::configs::default::dkg::{
    L_THRESHOLD, N, SHARE_COMPUTATION_E_SM_BASE_CONFIGS, SHARE_COMPUTATION_E_SM_BIT_SECRET,
};
use lib::configs::default::{N_PARTIES, T};
use lib::core::dkg::share_computation::base::SmudgingNoiseShareComputationBase;
use lib::math::polynomial::Polynomial;

fn main(
    expected_secret_commitment: pub Field,
    e_sm_secret: [Polynomial<N>; L_THRESHOLD],
    // y is public so wrapper can enforce consistency with chunk circuits
    y: pub [[[Field; N_PARTIES + 1]; L_THRESHOLD]; N],
) -> pub [[Field; L_THRESHOLD]; N_PARTIES] {
    let circuit: SmudgingNoiseShareComputationBase<N, L_THRESHOLD, N_PARTIES, T, SHARE_COMPUTATION_E_SM_BIT_SECRET> = SmudgingNoiseShareComputationBase::new(
        SHARE_COMPUTATION_E_SM_BASE_CONFIGS,
        expected_secret_commitment,
        e_sm_secret,
        y,
    );
    circuit.execute()
}
