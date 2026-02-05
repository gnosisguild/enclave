// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for the share-decryption circuit: honest ciphertexts, sum ciphertexts, secret key, and message.

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
use fhe_traits::FheDecrypter;
use fhe_traits::FheEncoder;
use fhe_traits::FheEncrypter;
use rand::thread_rng;

/// Sample data for the share-decryption circuit: honest ciphertexts, sum ciphertexts, secret key, and message.
#[derive(Debug, Clone)]
pub struct ShareDecryptionSample {
    /// H honest party ciphertexts (multiple encrypted shares from different parties)
    /// Structure: honest_ciphertexts[party_idx][trbfv_basis]
    pub honest_ciphertexts: Vec<Vec<Ciphertext>>,
    /// The sum of all honest ciphertexts per TRBFV basis (what we're actually decrypting)
    /// Structure: sum_ciphertexts[trbfv_basis]
    pub sum_ciphertexts: Vec<Ciphertext>,
    /// BFV secret key used for decryption (private witness)
    pub secret_key: SecretKey,
    /// The decrypted message (aggregate share values) - same for all TRBFV bases
    pub message: Plaintext,
}

impl ShareDecryptionSample {
    /// Generates sample data for the share-decryption circuit (decrypts a sum of honest ciphertexts under DKG secret key).
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

        let mut honest_ciphertexts: Vec<Vec<Ciphertext>> = Vec::new();
        let num_honest = committee.n;
        for _ in 0..num_honest {
            let mut party_cts = Vec::new();
            for _ in 0..threshold_params.moduli().len() {
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
                            .generate_smudging_error(
                                num_ciphertexts as usize,
                                lambda as usize,
                                &mut rng,
                            )
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
                                CircuitsErrors::Sample(format!(
                                    "Failed to generate error shares: {:?}",
                                    e
                                ))
                            })
                            .unwrap();

                        esi_sss_u64[0].row(0).to_vec()
                    }
                };

                let pt = Plaintext::try_encode(&share_row, Encoding::poly(), &dkg_params).unwrap();

                let ct = dkg_public_key.try_encrypt(&pt, &mut rng).unwrap();
                party_cts.push(ct);
            }
            honest_ciphertexts.push(party_cts);
        }

        // Compute the sum of all honest ciphertexts per TRBFV basis (homomorphic addition)
        // For each TRBFV basis: sum_ct[l] = ct_1[l] + ct_2[l] + ... + ct_H[l]
        let mut sum_ciphertexts: Vec<Ciphertext> = Vec::new();
        let num_moduli = threshold_params.moduli().len();
        for trbfv_basis_idx in 0..num_moduli {
            let mut sum_ct = honest_ciphertexts[0][trbfv_basis_idx].clone();
            for party_idx in 1..honest_ciphertexts.len() {
                sum_ct = &sum_ct + &honest_ciphertexts[party_idx][trbfv_basis_idx];
            }
            sum_ciphertexts.push(sum_ct);
        }

        // Decrypt the sum for the first TRBFV basis to get the aggregate plaintext
        // (The message should be the same for all TRBFV bases since we're decrypting the same aggregate)
        let decrypted_pt = dkg_secret_key.try_decrypt(&sum_ciphertexts[0]).unwrap();

        ShareDecryptionSample {
            honest_ciphertexts,
            sum_ciphertexts,
            secret_key: dkg_secret_key,
            message: decrypted_pt,
        }
    }
}

/// Prepares a share-decryption sample for testing using a threshold preset.
pub fn prepare_share_decryption_sample_for_test(
    preset: BfvPreset,
    committee: CiphernodesCommitteeSize,
    dkg_input_type: DkgInputType,
) -> ShareDecryptionSample {
    let defaults = preset.search_defaults().unwrap();

    ShareDecryptionSample::generate(
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
    use e3_fhe_params::{BfvPreset, DEFAULT_BFV_PRESET};

    #[test]
    fn test_generate_secret_key_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_share_decryption_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SecretKey,
        );

        assert_eq!(sample.honest_ciphertexts.len(), committee.n);
        assert_eq!(sample.sum_ciphertexts.len(), committee.threshold);
        assert_eq!(
            sample.secret_key.coeffs.len(),
            DEFAULT_BFV_PRESET.metadata().degree
        );
        assert_eq!(
            sample.message.value.len(),
            DEFAULT_BFV_PRESET.metadata().degree
        );
    }

    #[test]
    fn test_generate_smudging_noise_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_share_decryption_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SmudgingNoise,
        );

        assert_eq!(sample.honest_ciphertexts.len(), committee.n);
        assert_eq!(sample.sum_ciphertexts.len(), committee.threshold);
        assert_eq!(
            sample.secret_key.coeffs.len(),
            DEFAULT_BFV_PRESET.metadata().degree
        );
        assert_eq!(
            sample.message.value.len(),
            DEFAULT_BFV_PRESET.metadata().degree
        );
    }
}
