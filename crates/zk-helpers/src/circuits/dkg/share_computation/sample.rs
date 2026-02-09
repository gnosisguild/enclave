// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for the share-computation circuit: committee, DKG public key,
//! secret (SK or smudging noise) in CRT form, Shamir shares, and parity matrices.

use crate::ciphernodes_committee::CiphernodesCommittee;
use crate::ciphernodes_committee::CiphernodesCommitteeSize;
use crate::computation::DkgInputType;
use crate::CircuitsErrors;
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_parity_matrix::build_generator_matrix;
use e3_parity_matrix::{null_space, ParityMatrix, ParityMatrixConfig};
use e3_polynomial::CrtPolynomial;
use fhe::bfv::{PublicKey, SecretKey};
use fhe::trbfv::{ShareManager, TRBFV};
use num_bigint::BigInt;
use num_bigint::BigUint;
use rand::thread_rng;

/// Shamir secret shares: one limb per CRT modulus (rows = parties, cols = polynomial coefficients).
pub type SecretShares = Vec<ndarray::Array2<BigInt>>;

/// Sample data for the **share-computation** circuit: committee, DKG public key, secret in CRT form,
/// Shamir shares, and parity matrices (secret-key or smudging-noise).
#[derive(Debug, Clone)]
pub struct ShareComputationSample {
    /// Committee information.
    pub committee: CiphernodesCommittee,
    /// DKG BFV public key.
    pub dkg_public_key: PublicKey,
    /// Secret in CRT form (SK or smudging noise).
    pub secret: CrtPolynomial,
    /// Secret shares (one [`ndarray::Array2<BigInt>`] per modulus).
    pub secret_sss: SecretShares,
    /// Parity check matrix per modulus (null space of generator).
    pub parity_matrix: Vec<ParityMatrix>,
}

impl ShareComputationSample {
    /// Generates sample data for the share-computation circuit.
    pub fn generate(
        preset: BfvPreset,
        committee_size: CiphernodesCommitteeSize,
        dkg_input_type: DkgInputType,
        num_ciphertexts: u128, // z in the search defaults
        lambda: u32,
    ) -> Self {
        let (threshold_params, dkg_params) = build_pair_for_preset(preset).unwrap();

        let mut rng = thread_rng();
        let committee = committee_size.values();

        let dkg_secret_key = SecretKey::random(&dkg_params, &mut rng);
        let dkg_public_key = PublicKey::new(&dkg_secret_key, &mut rng);

        let trbfv = TRBFV::new(committee.n, committee.threshold, threshold_params.clone())
            .unwrap_or_else(|e| panic!("Failed to create TRBFV: {:?}", e));
        let mut share_manager =
            ShareManager::new(committee.n, committee.threshold, threshold_params.clone());

        // Parity check matrix (null space of generator) per modulus: [L][N_PARTIES-T][N_PARTIES+1].
        let mut parity_matrix = Vec::with_capacity(threshold_params.moduli().len());
        for &qi in threshold_params.moduli() {
            let q = BigUint::from(qi);
            let g = build_generator_matrix(&ParityMatrixConfig {
                q: q.clone(),
                t: committee.threshold,
                n: committee.n,
            })
            .unwrap();
            let h = null_space(&g, &q).unwrap();
            parity_matrix.push(h);
        }

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
                    .generate_smudging_error(num_ciphertexts as usize, lambda as usize, &mut rng)
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
            committee,
            dkg_public_key,
            secret,
            secret_sss,
            parity_matrix,
        }
    }
}

/// Prepares a share-computation sample for testing using a threshold preset.
pub fn prepare_share_computation_sample_for_test(
    preset: BfvPreset,
    committee: CiphernodesCommitteeSize,
    dkg_input_type: DkgInputType,
) -> ShareComputationSample {
    let defaults = preset.search_defaults().unwrap();

    ShareComputationSample::generate(
        preset,
        committee,
        dkg_input_type,
        defaults.z,
        defaults.lambda,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::computation::DkgInputType;
    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_generate_secret_key_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_share_computation_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SecretKey,
        );

        assert_eq!(sample.committee.n, committee.n);
        assert_eq!(sample.committee.threshold, committee.threshold);
        assert_eq!(sample.committee.h, committee.h);
        assert_eq!(sample.dkg_public_key.c.c.len(), 2);
        assert_eq!(sample.secret_sss.len(), 2);
        assert_eq!(sample.secret.limbs.len(), 2);
    }

    #[test]
    fn test_generate_smudging_noise_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_share_computation_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SmudgingNoise,
        );

        assert_eq!(sample.committee.n, committee.n);
        assert_eq!(sample.committee.threshold, committee.threshold);
        assert_eq!(sample.dkg_public_key.c.c.len(), 2);
        assert_eq!(sample.secret_sss.len(), 2);
        assert_eq!(sample.secret.limbs.len(), 2);
    }
}
