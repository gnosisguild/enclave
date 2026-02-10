// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for the share-computation circuit: committee, DKG public key,
//! secret (SK or smudging noise) in CRT form, Shamir shares, and parity matrices.

use crate::ciphernodes_committee::CiphernodesCommitteeSize;
use crate::circuits::dkg::share_computation::utils::compute_parity_matrix;
use crate::computation::DkgInputType;
use crate::dkg::share_computation::ShareComputationCircuitInput;
use crate::CircuitsErrors;
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::CrtPolynomial;
use fhe::bfv::SecretKey;
use fhe::trbfv::{ShareManager, TRBFV};
use num_bigint::BigInt;
use rand::thread_rng;

pub type SecretShares = Vec<ndarray::Array2<BigInt>>;

impl ShareComputationCircuitInput {
    /// Generates sample data for the share-computation circuit.
    pub fn generate_sample(
        preset: BfvPreset,
        committee_size: CiphernodesCommitteeSize,
        dkg_input_type: DkgInputType,
    ) -> Self {
        let (threshold_params, _) = build_pair_for_preset(preset).unwrap();
        let sd = preset.search_defaults().unwrap();
        let mut rng = thread_rng();
        let committee = committee_size.values();

        let trbfv = TRBFV::new(committee.n, committee.threshold, threshold_params.clone())
            .unwrap_or_else(|e| panic!("Failed to create TRBFV: {:?}", e));
        let mut share_manager =
            ShareManager::new(committee.n, committee.threshold, threshold_params.clone());

        let parity_matrix =
            compute_parity_matrix(threshold_params.moduli(), committee.n, committee.threshold)
                .unwrap_or_else(|e| panic!("Failed to compute parity matrix: {}", e));

        let (secret, secret_sss) = match dkg_input_type {
            DkgInputType::SecretKey => {
                let threshold_secret_key = SecretKey::random(&threshold_params, &mut rng);

                let sk_poly = share_manager
                    .coeffs_to_poly_level0(threshold_secret_key.coeffs.clone().as_ref())
                    .unwrap();

                let sk_sss_u64 = share_manager
                    .generate_secret_shares_from_poly(sk_poly.clone(), rng)
                    .unwrap();

                let secret_sss: SecretShares = sk_sss_u64
                    .into_iter()
                    .map(|arr| arr.mapv(BigInt::from))
                    .collect();

                let sk_coeffs: Vec<BigInt> = threshold_secret_key
                    .coeffs
                    .iter()
                    .map(|&c| BigInt::from(c))
                    .collect();
                let mut secret_crt =
                    CrtPolynomial::from_mod_q_polynomial(&sk_coeffs, threshold_params.moduli());
                secret_crt.center(threshold_params.moduli()).unwrap();

                (secret_crt, secret_sss)
            }
            DkgInputType::SmudgingNoise => {
                let esi_coeffs = trbfv
                    .generate_smudging_error(committee.n as usize, sd.lambda as usize, &mut rng)
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!(
                            "Failed to generate smudging error: {:?}",
                            e
                        ))
                    })
                    .unwrap();
                let esi_poly = share_manager.bigints_to_poly(&esi_coeffs).unwrap();
                let esi_sss_u64 = share_manager
                    .generate_secret_shares_from_poly(esi_poly.clone(), rng)
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!("Failed to generate error shares: {:?}", e))
                    })
                    .unwrap();
                let secret_sss: SecretShares = esi_sss_u64
                    .into_iter()
                    .map(|arr| arr.mapv(BigInt::from))
                    .collect();

                let mut secret_crt =
                    CrtPolynomial::from_mod_q_polynomial(&esi_coeffs, threshold_params.moduli());
                secret_crt.center(threshold_params.moduli()).unwrap();

                (secret_crt, secret_sss)
            }
        };

        Self {
            dkg_input_type,
            n_parties: committee.n as u32,
            threshold: committee.threshold as u32,
            secret,
            secret_sss,
            parity_matrix,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::computation::DkgInputType;
    use crate::dkg::share_computation::ShareComputationCircuitInput;
    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_generate_secret_key_sample() {
        let committee_size = CiphernodesCommitteeSize::Small;
        let sample = ShareComputationCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee_size,
            DkgInputType::SecretKey,
        );
        assert_eq!(sample.n_parties, committee_size.values().n as u32);
        assert_eq!(sample.threshold, committee_size.values().threshold as u32);
        assert_eq!(sample.dkg_input_type, DkgInputType::SecretKey);
        assert_eq!(sample.secret_sss.len(), 2);
        assert_eq!(sample.secret.limbs.len(), 2);
    }

    #[test]
    fn test_generate_smudging_noise_sample() {
        let committee_size = CiphernodesCommitteeSize::Small;
        let sample = ShareComputationCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee_size,
            DkgInputType::SmudgingNoise,
        );
        assert_eq!(sample.n_parties, committee_size.values().n as u32);
        assert_eq!(sample.threshold, committee_size.values().threshold as u32);
        assert_eq!(sample.dkg_input_type, DkgInputType::SmudgingNoise);
        assert_eq!(sample.secret_sss.len(), 2);
        assert_eq!(sample.secret.limbs.len(), 2);
    }
}
