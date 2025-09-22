// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use e3_bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_crypto::Cipher;
use e3_events::ThresholdShare;
use e3_fhe::create_crp;
use e3_test_helpers::create_shared_rng_from_u64;
use e3_trbfv::{
    calculate_decryption_key::{
        calculate_decryption_key, CalculateDecryptionKeyRequest, CalculateDecryptionKeyResponse,
    },
    calculate_decryption_share::{
        calculate_decryption_share, CalculateDecryptionShareRequest,
        CalculateDecryptionShareResponse,
    },
    calculate_threshold_decryption::{
        calculate_threshold_decryption, CalculateThresholdDecryptionRequest,
        CalculateThresholdDecryptionResponse,
    },
    gen_esi_sss::{gen_esi_sss, GenEsiSssRequest, GenEsiSssResponse},
    gen_pk_share_and_sk_sss::{
        gen_pk_share_and_sk_sss, GenPkShareAndSkSssRequest, GenPkShareAndSkSssResponse,
    },
    helpers::calculate_error_size,
    shares::{EncryptableVec, PvwEncrypted, PvwEncryptedVecExt, ShamirShare, SharedSecret},
    TrBFVConfig,
};
use e3_utils::{to_ordered_vec, ArcBytes};
use fhe::{
    bfv::PublicKey,
    mbfv::{AggregateIter, PublicKeyShare},
};
use fhe_traits::Serialize;
use num_bigint::BigUint;

#[tokio::test]
async fn test_trbfv_isolation() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_test_writer()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);
    let rng = create_shared_rng_from_u64(42);

    let (degree, plaintext_modulus, moduli) = (
        8192usize,
        16384u64,
        &[
            0x1FFFFFFEA0001u64, // 562949951979521
            0x1FFFFFFE88001u64, // 562949951881217
            0x1FFFFFFE48001u64, // 562949951619073
            0xfffffebc001u64,   //
        ] as &[u64],
    );

    let params_raw = build_bfv_params_arc(degree, plaintext_modulus, moduli);
    let params = ArcBytes::from_bytes(encode_bfv_params(&params_raw.clone()));

    let crp_raw = create_crp(params_raw.clone(), rng.clone());
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    // let seed = create_seed_from_u64(123);
    let error_size = ArcBytes::from_bytes(BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        5,
        3,
    )?));

    // Application vars
    let num_votes_per_voter = 3;
    let num_voters = 1000;

    // Parameters
    let threshold_m = 2;
    let threshold_n = 5;
    let esi_per_ct = 3;

    let trbfv_config = TrBFVConfig::new(params, threshold_n, threshold_m);
    let crp = ArcBytes::from_bytes(crp_raw.to_bytes());
    let mut shares_hash_map = HashMap::new();

    for party_id in 0u64..threshold_n {
        let GenEsiSssResponse { esi_sss } = gen_esi_sss(
            &rng,
            &cipher,
            GenEsiSssRequest {
                esi_per_ct,
                error_size: error_size.clone(),
                trbfv_config: trbfv_config.clone(),
            },
        )?;

        let GenPkShareAndSkSssResponse { sk_sss, pk_share } = gen_pk_share_and_sk_sss(
            &rng,
            &cipher,
            GenPkShareAndSkSssRequest {
                trbfv_config: trbfv_config.clone(),
                crp: crp.clone(),
            },
        )?;

        // Simulate actor boundry and SharesGenerated
        let sk_sss = PvwEncrypted::new(sk_sss.decrypt(&cipher)?)?;
        let esi_sss: Vec<PvwEncrypted<SharedSecret>> = esi_sss
            .into_iter()
            .map(|s| PvwEncrypted::new(s.decrypt(&cipher)?))
            .collect::<Result<_>>()?;

        shares_hash_map.insert(
            party_id,
            ThresholdShare {
                party_id,
                esi_sss,
                sk_sss,
                pk_share,
            },
        );
    }

    let pubkey: PublicKey = shares_hash_map
        .clone()
        .into_iter()
        .map(|(_, v)| v.pk_share)
        .map(|k| {
            PublicKeyShare::deserialize(&k, &params_raw, crp_raw.clone())
                .context("Could not deserialize public key")
        })
        .collect::<Result<Vec<PublicKeyShare>>>()?
        .into_iter()
        .aggregate()?;

    // All shares_hash_map should receive the same encrypted list from all other shares_hash_map
    let shares = to_ordered_vec(shares_hash_map);
    let received_sss = shares
        .iter()
        .map(|ts| ts.sk_sss.clone().pvw_decrypt())
        .collect::<Result<Vec<SharedSecret>>>()?;

    let received_esi_sss: Vec<Vec<SharedSecret>> = shares
        .iter()
        .map(|ts| ts.esi_sss.clone().to_vec_decrypted())
        .collect::<Result<Vec<_>>>()?;

    // Individualize based on node
    let mut decryption_keys = HashMap::new();
    for party_id in 0..threshold_n as usize {
        let sk_sss_collected = received_sss
            .clone()
            .into_iter()
            .map(|sss| sss.extract_party_share(party_id))
            .collect::<Result<Vec<_>>>()?;

        let esi_sss_collected: Vec<Vec<ShamirShare>> = received_esi_sss
            .clone()
            .into_iter()
            .map(|s| {
                s.into_iter()
                    .map(|ss| ss.extract_party_share(party_id))
                    .collect()
            })
            .collect::<Result<_>>()?;

        let CalculateDecryptionKeyResponse {
            es_poly_sum,
            sk_poly_sum,
        } = calculate_decryption_key(
            &cipher,
            CalculateDecryptionKeyRequest {
                trbfv_config: trbfv_config.clone(),
                esi_sss_collected: esi_sss_collected
                    .into_iter()
                    .map(|s| s.encrypt(&cipher))
                    .collect::<Result<_>>()?,
                sk_sss_collected: sk_sss_collected.encrypt(&cipher)?,
            },
        )?;
        decryption_keys.insert(party_id, (es_poly_sum, sk_poly_sum));
    }

    // Create the inputs
    let (inputs, numbers) = application::generate_ciphertexts(
        &pubkey,
        params_raw.clone(),
        num_voters,
        num_votes_per_voter,
    );

    let outputs = application::run_application(&inputs, params_raw, num_votes_per_voter);

    // Encrypt the plaintext
    let mut decryption_shares = HashMap::new();
    let ciphertexts = outputs
        .into_iter()
        .map(|ct| ArcBytes::from_bytes((*ct).clone().to_bytes()))
        .collect::<Vec<ArcBytes>>();

    for party_id in 0..=threshold_m as usize {
        let (es_poly_sum, sk_poly_sum) = decryption_keys.get(&party_id).unwrap();
        let CalculateDecryptionShareResponse { d_share_poly } = calculate_decryption_share(
            &cipher,
            CalculateDecryptionShareRequest {
                sk_poly_sum: sk_poly_sum.clone(),
                trbfv_config: trbfv_config.clone(),
                es_poly_sum: es_poly_sum.clone(),
                ciphertexts: ciphertexts.clone(),
            },
        )?;
        decryption_shares.insert(party_id as u64, d_share_poly);
    }

    let d_share_polys: Vec<(u64, Vec<ArcBytes>)> = decryption_shares.into_iter().collect();

    let CalculateThresholdDecryptionResponse { plaintext } =
        calculate_threshold_decryption(CalculateThresholdDecryptionRequest {
            ciphertexts,
            trbfv_config: trbfv_config.clone(),
            d_share_polys,
        })?;

    let results = plaintext
        .into_iter()
        .map(|a| {
            bincode::deserialize(&a.extract_bytes()).context("Could not deserialize plaintext")
        })
        .collect::<Result<Vec<Vec<u64>>>>()?;

    let results: Vec<u64> = results
        .into_iter()
        .map(|r| r.first().unwrap().clone())
        .collect();

    // Show summation result
    let mut expected_result = vec![0u64; 3];
    for vals in &numbers {
        for j in 0..num_votes_per_voter {
            expected_result[j] += vals[j];
        }
    }

    for (i, (res, exp)) in results.iter().zip(expected_result.iter()).enumerate() {
        println!("Tally {i} result = {res} / {exp}");
        assert_eq!(res, exp);
    }
    Ok(())
}

mod application {
    use rand::{distributions::Uniform, prelude::Distribution, thread_rng};

    use fhe_traits::{FheEncoder, FheEncrypter};
    use std::sync::Arc;

    use fhe::bfv::{self, Ciphertext, Encoding, Plaintext, PublicKey};

    /// Each Voter encrypts `num_votes_per_voter` random bits and returns the ciphertexts along with
    /// the underlying plaintexts for verification.
    pub fn generate_ciphertexts(
        pk: &PublicKey,
        params: Arc<bfv::BfvParameters>,
        num_voters: usize,
        num_votes_per_voter: usize,
    ) -> (Vec<Vec<Ciphertext>>, Vec<Vec<u64>>) {
        let dist = Uniform::new_inclusive(0, 1);
        let mut rng = thread_rng();
        let numbers: Vec<Vec<u64>> = (0..num_voters)
            .map(|_| {
                (0..num_votes_per_voter)
                    .map(|_| dist.sample(&mut rng))
                    .collect()
            })
            .collect();

        let ciphertexts: Vec<Vec<Ciphertext>> = numbers
            .iter()
            .map(|vals| {
                let mut rng = thread_rng();
                vals.iter()
                    .map(|&val| {
                        let pt = Plaintext::try_encode(&[val], Encoding::poly(), &params).unwrap();
                        pk.try_encrypt(&pt, &mut rng).unwrap()
                    })
                    .collect()
            })
            .collect();
        (ciphertexts, numbers)
    }

    /// Tally the submitted ciphertexts column-wise to produce aggregated sums.
    pub fn run_application(
        ciphertexts: &[Vec<Ciphertext>],
        params: Arc<bfv::BfvParameters>,
        num_votes_per_voter: usize,
    ) -> Vec<Arc<Ciphertext>> {
        if ciphertexts.is_empty() {
            return Vec::new();
        }

        let mut sums: Vec<Ciphertext> = (0..num_votes_per_voter)
            .map(|_| Ciphertext::zero(&params))
            .collect();

        for ct_group in ciphertexts {
            for (j, ciphertext) in ct_group.iter().enumerate() {
                sums[j] += ciphertext;
            }
        }

        sums.into_iter().map(Arc::new).collect()
    }
}
