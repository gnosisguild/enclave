// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use e3_bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_crypto::Cipher;
use e3_fhe::create_crp;
use e3_test_helpers::{create_seed_from_u64, create_shared_rng_from_u64, usecase_helpers};
use e3_trbfv::{
    calculate_decryption_share::{
        calculate_decryption_share, CalculateDecryptionShareRequest,
        CalculateDecryptionShareResponse,
    },
    calculate_threshold_decryption::{
        calculate_threshold_decryption, CalculateThresholdDecryptionRequest,
        CalculateThresholdDecryptionResponse,
    },
    helpers::calculate_error_size,
    TrBFVConfig,
};
use e3_utils::{to_ordered_vec, ArcBytes};
use fhe_traits::Serialize;
use num_bigint::BigUint;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

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

    let params_raw = build_bfv_params_arc(degree, plaintext_modulus, moduli, None);
    let params = ArcBytes::from_bytes(&encode_bfv_params(&params_raw.clone()));

    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let error_size = ArcBytes::from_bytes(&BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        5,
        3,
    )?));

    // E3Parameters
    let threshold_m = 2;
    let threshold_n = 5;
    let esi_per_ct = 3;
    let seed = create_seed_from_u64(123);

    let trbfv_config = TrBFVConfig::new(params, threshold_n, threshold_m);
    let crp_raw = create_crp(
        trbfv_config.params(),
        Arc::new(Mutex::new(ChaCha20Rng::from_seed(seed.into()))),
    );

    // let crp = ArcBytes::from_bytes(crp_raw.to_bytes());
    let shares_hash_map = usecase_helpers::generate_shares_hash_map(
        &trbfv_config,
        esi_per_ct,
        &error_size,
        &crp_raw,
        &rng,
        &cipher,
    )?;

    let pubkey =
        usecase_helpers::get_public_key(&shares_hash_map, trbfv_config.params(), &crp_raw)?;
    let shares = to_ordered_vec(shares_hash_map);
    let decryption_keys = usecase_helpers::get_decryption_keys(shares, &cipher, &trbfv_config)?;
    // Create the inputs
    let num_votes_per_voter = 3;
    let num_voters = 30;
    let (inputs, numbers) = e3_test_helpers::application::generate_ciphertexts(
        &pubkey,
        params_raw.clone(),
        num_voters,
        num_votes_per_voter,
    );

    let outputs =
        e3_test_helpers::application::run_application(&inputs, params_raw, num_votes_per_voter);

    // Encrypt the plaintext
    let ciphertexts = outputs
        .into_iter()
        .map(|ct| ArcBytes::from_bytes(&(*ct).clone().to_bytes()))
        .collect::<Vec<ArcBytes>>();

    let mut decryption_shares = HashMap::new();
    // for party_id in 0..=threshold_m as usize {
    for party_id in [1, 4, 2] {
        let (es_poly_sum, sk_poly_sum) = decryption_keys.get(&party_id).unwrap();
        println!("calculate_decryption_share for party_id={}", party_id);
        let CalculateDecryptionShareResponse { d_share_poly } = calculate_decryption_share(
            &cipher,
            CalculateDecryptionShareRequest {
                name: format!("party_id({})", party_id),
                sk_poly_sum: sk_poly_sum.clone(),
                trbfv_config: trbfv_config.clone(),
                es_poly_sum: es_poly_sum.clone(),
                ciphertexts: ciphertexts.clone(),
            },
        )?;

        // store the decryption shares in a hash map indexed by party_id
        decryption_shares.insert(party_id as u64, d_share_poly);
    }

    // Get a vector of the shares
    let d_share_polys: Vec<(u64, Vec<ArcBytes>)> = decryption_shares.into_iter().collect();

    let CalculateThresholdDecryptionResponse { plaintext } =
        // NOTE: data prep in this function will sort the decryption shares by party_id
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
