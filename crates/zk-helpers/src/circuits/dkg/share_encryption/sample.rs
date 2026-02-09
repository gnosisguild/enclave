// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for the share-encryption circuit: DKG public key, plaintext,
//! ciphertext, and encryption randomness (u_rns, e0_rns, e1_rns) for testing and codegen.

use crate::ciphernodes_committee::CiphernodesCommitteeSize;
use crate::computation::DkgInputType;
use crate::CircuitsErrors;
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use fhe::bfv::Ciphertext;
use fhe::bfv::Encoding;
use fhe::bfv::Plaintext;
use fhe::bfv::{PublicKey, SecretKey};
use fhe::trbfv::{ShareManager, TRBFV};
use fhe_math::rq::Poly;
use fhe_traits::FheEncoder;
use rand::thread_rng;

/// Sample data for the share-encryption circuit: plaintext, ciphertext, keys, and RNS randomness.
#[derive(Debug, Clone)]
pub struct ShareEncryptionSample {
    pub plaintext: Plaintext,
    pub ciphertext: Ciphertext,
    pub public_key: PublicKey,
    pub secret_key: SecretKey,
    pub u_rns: Poly,
    pub e0_rns: Poly,
    pub e1_rns: Poly,
}

impl ShareEncryptionSample {
    /// Generates sample data for the share-encryption circuit (encrypts a share row under DKG pk).
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

        let share_row = match dkg_input_type {
            DkgInputType::SecretKey => {
                let threshold_secret_key = SecretKey::random(&threshold_params, &mut rng);

                let sk_poly = share_manager
                    .coeffs_to_poly_level0(threshold_secret_key.coeffs.clone().as_ref())
                    .unwrap();

                let sk_sss_u64 = share_manager
                    .generate_secret_shares_from_poly(sk_poly.clone(), &mut rng)
                    .unwrap();

                sk_sss_u64[0].row(0).to_vec()
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
                    .generate_secret_shares_from_poly(esi_poly.clone(), &mut rng.clone())
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!("Failed to generate error shares: {:?}", e))
                    })
                    .unwrap();

                esi_sss_u64[0].row(0).to_vec()
            }
        };

        let pt = Plaintext::try_encode(&share_row, Encoding::poly(), &dkg_params).unwrap();

        let (_ct, u_rns, e0_rns, e1_rns) =
            dkg_public_key.try_encrypt_extended(&pt, &mut rng).unwrap();

        ShareEncryptionSample {
            plaintext: pt,
            ciphertext: _ct,
            public_key: dkg_public_key,
            secret_key: dkg_secret_key,
            u_rns,
            e0_rns,
            e1_rns,
        }
    }
}

/// Prepares a share-encryption sample for testing using a threshold preset.
pub fn prepare_share_encryption_sample_for_test(
    preset: BfvPreset,
    committee: CiphernodesCommitteeSize,
    dkg_input_type: DkgInputType,
) -> ShareEncryptionSample {
    let defaults = preset.search_defaults().unwrap();

    ShareEncryptionSample::generate(
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
        let sample = prepare_share_encryption_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SecretKey,
        );

        assert_eq!(sample.public_key.c.c.len(), 2);
        assert_eq!(
            sample.plaintext.value.len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
        assert_eq!(sample.ciphertext.c.len(), 2);
        assert_eq!(
            sample.u_rns.coefficients().len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
        assert_eq!(
            sample.e0_rns.coefficients().len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
        assert_eq!(
            sample.e1_rns.coefficients().len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
    }

    #[test]
    fn test_generate_smudging_noise_sample() {
        let sample = prepare_share_encryption_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SmudgingNoise,
        );

        assert_eq!(sample.public_key.c.c.len(), 2);
        assert_eq!(sample.ciphertext.c.len(), 2);
        assert_eq!(
            sample.u_rns.coefficients().len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
        assert_eq!(
            sample.e0_rns.coefficients().len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
        assert_eq!(
            sample.e1_rns.coefficients().len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
    }
}
