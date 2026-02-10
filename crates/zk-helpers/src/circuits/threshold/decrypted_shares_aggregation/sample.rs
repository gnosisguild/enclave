// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for decrypted shares aggregation circuit.
//!
//! Produces TRBFV parties with secret/public key shares, collects and aggregates shares,
//! encrypts a message, computes T+1 decryption shares, and decrypts to obtain the message.
//! The result is used as input for witness computation and codegen.

use crate::circuits::computation::Computation;
use crate::threshold::decrypted_shares_aggregation::computation::Configs;
use crate::{
    threshold::decrypted_shares_aggregation::DecryptedSharesAggregationCircuitInput,
    CiphernodesCommittee,
};
use e3_fhe_params::{build_pair_for_preset, BfvPreset};
use fhe::bfv::{Encoding, Plaintext, PublicKey, SecretKey};
use fhe::mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare};
use fhe::trbfv::{ShareManager, TRBFV};
use fhe_math::rq::{Poly, Representation};
use fhe_traits::FheDecoder;
use fhe_traits::{FheEncoder, FheEncrypter};
use ndarray::Array2;
use rand::rngs::OsRng;
use std::sync::Arc;

struct Party {
    pk_share: PublicKeyShare,
    sk_sss: Vec<Array2<u64>>,
    esi_sss: Vec<Array2<u64>>,
    sk_sss_collected: Vec<Array2<u64>>,
    es_sss_collected: Vec<Array2<u64>>,
    sk_poly_sum: Poly,
    es_poly_sum: Poly,
}

impl DecryptedSharesAggregationCircuitInput {
    /// Generates sample data for the decrypted shares aggregation circuit:
    /// TRBFV setup, parties with sk/pk shares and smudging error shares, share collection
    /// and aggregation, encryption of a message, T+1 decryption shares, and threshold decrypt.
    pub fn generate_sample(preset: BfvPreset, committee: CiphernodesCommittee) -> Self {
        let (threshold_params, _) = build_pair_for_preset(preset).unwrap();

        let sd = preset.search_defaults().unwrap();

        let num_parties = committee.n;
        let threshold = committee.threshold;
        let degree = threshold_params.degree();
        let num_moduli = threshold_params.moduli().len();

        let trbfv = TRBFV::new(num_parties, threshold, threshold_params.clone()).unwrap();
        let mut rng = OsRng;
        let mut thread_rng = rand::thread_rng();

        let crp = CommonRandomPoly::new(&threshold_params, &mut rng).unwrap();

        let ctx = threshold_params.ctx_at_level(0).unwrap();

        let mut parties: Vec<Party> = (0..num_parties)
            .map(|_| {
                let sk_share = SecretKey::random(&threshold_params, &mut rng);
                let pk_share =
                    PublicKeyShare::new(&sk_share, crp.clone(), &mut thread_rng).unwrap();

                let mut share_manager =
                    ShareManager::new(num_parties, threshold, threshold_params.clone());
                let sk_poly = share_manager
                    .coeffs_to_poly_level0(sk_share.coeffs.as_ref())
                    .unwrap();

                let sk_sss = share_manager
                    .generate_secret_shares_from_poly(sk_poly, &mut rng)
                    .unwrap();

                let esi_coeffs = trbfv
                    .generate_smudging_error(sd.z as usize, sd.lambda as usize, &mut rng)
                    .unwrap();
                let esi_poly = share_manager.bigints_to_poly(&esi_coeffs).unwrap();
                let esi_sss = share_manager
                    .generate_secret_shares_from_poly(esi_poly, &mut rng)
                    .unwrap();

                let sk_sss_collected = Vec::with_capacity(num_parties);
                let es_sss_collected = Vec::with_capacity(num_parties);
                let sk_poly_sum = Poly::zero(&ctx, Representation::PowerBasis);
                let es_poly_sum = Poly::zero(&ctx, Representation::PowerBasis);

                Party {
                    pk_share,
                    sk_sss,
                    esi_sss,
                    sk_sss_collected,
                    es_sss_collected,
                    sk_poly_sum,
                    es_poly_sum,
                }
            })
            .collect();

        // Collect shares: for each party i, sk_sss_collected is one Array2 per sender j
        // (same as Vec<ShamirShare>). Each Array2 has shape (num_moduli, degree): row m = share from j for modulus m.
        for i in 0..num_parties {
            parties[i].sk_sss_collected = (0..num_parties)
                .map(|j| {
                    let data: Vec<u64> = (0..num_moduli)
                        .flat_map(|m| {
                            parties[j].sk_sss[m]
                                .row(i)
                                .iter()
                                .copied()
                                .collect::<Vec<_>>()
                        })
                        .collect();
                    Array2::from_shape_vec((num_moduli, degree), data)
                        .map_err(|e| format!("sk_sss_collected shape: {:?}", e))
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            parties[i].es_sss_collected = (0..num_parties)
                .map(|j| {
                    let data: Vec<u64> = (0..num_moduli)
                        .flat_map(|m| {
                            parties[j].esi_sss[m]
                                .row(i)
                                .iter()
                                .copied()
                                .collect::<Vec<_>>()
                        })
                        .collect();
                    Array2::from_shape_vec((num_moduli, degree), data)
                        .map_err(|e| format!("es_sss_collected shape: {:?}", e))
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        }

        // Aggregate collected shares to get sk_poly_sum and es_poly_sum per party
        for party in parties.iter_mut() {
            let share_manager = ShareManager::new(num_parties, threshold, threshold_params.clone());
            party.sk_poly_sum = share_manager
                .aggregate_collected_shares(&party.sk_sss_collected)
                .unwrap();
            party.es_poly_sum = share_manager
                .aggregate_collected_shares(&party.es_sss_collected)
                .unwrap();
        }

        // Aggregate public key
        let public_key: PublicKey = parties
            .iter()
            .map(|p| p.pk_share.clone())
            .collect::<Vec<_>>()
            .iter()
            .cloned()
            .aggregate()
            .unwrap();

        // Build message: max_msg_non_zero_coeffs from config, tiled from CRISP-style pattern, pad to degree
        let configs = Configs::compute(preset, &()).unwrap();
        let n = configs.max_msg_non_zero_coeffs;
        let pattern: Vec<u64> = vec![
            2, 1, 5, 2, 1, 2, 3, 2, 4, 3, 3, 3, 2, 3, 3, 1, 2, 3, 4, 6, 1, 5, 1, 1, 2, 1, 2,
        ];
        let mut message: Vec<u64> = (0..n).map(|i| pattern[i % pattern.len()]).collect();
        message.resize(degree, 0);

        let pt = Plaintext::try_encode(&message, Encoding::poly(), &threshold_params).unwrap();
        let ciphertext = public_key.try_encrypt(&pt, &mut thread_rng).unwrap();

        let ciphertext = Arc::new(ciphertext);

        // Decryption shares from T+1 parties (1-based party IDs)
        let honest_parties = threshold + 1;
        let mut d_share_polys: Vec<Poly> = Vec::with_capacity(honest_parties);

        for party in parties.iter().take(honest_parties) {
            let share_manager = ShareManager::new(num_parties, threshold, threshold_params.clone());
            // For a single ciphertext, es_poly_sum is one Poly per party
            let d_share = share_manager
                .decryption_share(
                    Arc::clone(&ciphertext),
                    party.sk_poly_sum.clone(),
                    party.es_poly_sum.clone(),
                )
                .unwrap();
            d_share_polys.push(d_share);
        }

        let reconstructing_parties: Vec<usize> = (1..=honest_parties).collect();

        // Threshold decrypt to obtain message (verify)
        let share_manager = ShareManager::new(num_parties, threshold, threshold_params.clone());
        let plaintext = share_manager
            .decrypt_from_shares(
                d_share_polys.clone(),
                reconstructing_parties.clone(),
                Arc::clone(&ciphertext),
            )
            .unwrap();

        let message_vec = Vec::<u64>::try_decode(&plaintext, Encoding::poly()).unwrap();

        DecryptedSharesAggregationCircuitInput {
            committee,
            d_share_polys,
            reconstructing_parties,
            message_vec,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        computation::Computation,
        threshold::decrypted_shares_aggregation::{
            DecryptedSharesAggregationCircuitInput, Witness,
        },
        CiphernodesCommitteeSize,
    };
    use e3_fhe_params::BfvPreset;
    use num_bigint::BigInt;

    /// Sample generation and witness computation: output shapes match circuit expectations.
    #[test]
    fn test_generate_sample() {
        let preset = BfvPreset::InsecureThreshold512;
        let committee = CiphernodesCommitteeSize::Small.values();

        let sample = DecryptedSharesAggregationCircuitInput::generate_sample(preset, committee);
        let witness = Witness::compute(preset, &sample).unwrap();

        assert_eq!(
            witness.decryption_shares.len(),
            sample.committee.threshold + 1
        );
        assert_eq!(witness.party_ids.len(), sample.reconstructing_parties.len());
        let configs =
            crate::threshold::decrypted_shares_aggregation::computation::Configs::compute(
                preset,
                &(),
            )
            .unwrap();
        assert_eq!(witness.message.len(), configs.max_msg_non_zero_coeffs);
    }

    /// Witness message matches sample (ascending order: index 0 = constant term).
    #[test]
    fn test_witness_message_matches_sample() {
        use crate::threshold::decrypted_shares_aggregation::computation::Configs;
        let preset = BfvPreset::InsecureThreshold512;
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = DecryptedSharesAggregationCircuitInput::generate_sample(preset, committee);
        let witness = Witness::compute(preset, &sample).unwrap();
        let configs = Configs::compute(preset, &()).unwrap();
        let n = configs.max_msg_non_zero_coeffs;
        for i in 0..n {
            let expected = sample.message_vec.get(i).copied().unwrap_or(0);
            let w = &witness.message[i];
            let exp = BigInt::from(expected);
            assert_eq!(w, &exp, "message coeff {} mismatch", i);
        }
    }
}
