// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for the share-decryption circuit: honest ciphertexts, sum ciphertexts, secret key, and message.

use crate::ciphernodes_committee::CiphernodesCommitteeSize;
use crate::circuits::dkg::share_decryption::circuit::ShareDecryptionCircuitInput;
use crate::computation::DkgInputType;
use crate::CircuitsErrors;
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use fhe::bfv::Ciphertext;
use fhe::bfv::Encoding;
use fhe::bfv::Plaintext;
use fhe::bfv::{PublicKey, SecretKey};
use fhe::trbfv::{ShareManager, TRBFV};
use fhe_traits::FheEncoder;
use fhe_traits::FheEncrypter;
use rand::thread_rng;

impl ShareDecryptionCircuitInput {
    /// Generates sample data for the share-decryption circuit (decrypts a sum of honest ciphertexts under DKG secret key).
    pub fn generate_sample(
        preset: BfvPreset,
        committee: CiphernodesCommitteeSize,
        dkg_input_type: DkgInputType,
    ) -> Self {
        let (threshold_params, dkg_params) = build_pair_for_preset(preset).unwrap();
        let sd = preset.search_defaults().unwrap();

        let mut rng = thread_rng();
        let committee = committee.values();

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
                            .generate_smudging_error(sd.z as usize, sd.lambda as usize, &mut rng)
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

        ShareDecryptionCircuitInput {
            honest_ciphertexts,
            secret_key: dkg_secret_key,
        }
    }
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
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SecretKey,
        );

        assert_eq!(sample.honest_ciphertexts.len(), committee.n);
        assert_eq!(
            sample.secret_key.coeffs.len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
    }

    #[test]
    fn test_generate_smudging_noise_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SmudgingNoise,
        );

        assert_eq!(sample.honest_ciphertexts.len(), committee.n);
        assert_eq!(
            sample.secret_key.coeffs.len(),
            BfvPreset::InsecureThreshold512.metadata().degree
        );
    }
}
