// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use bytesize::ByteSize;
use e3_bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_crypto::Cipher;
use e3_fhe::create_crp;
use e3_test_helpers::{
    create_seed_from_u64, create_shared_rng_from_u64, reporters::SizeReporter, usecase_helpers,
};
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
    let start = Instant::now();
    let mut reporter = SizeReporter::new();

    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_test_writer()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);
    let rng = create_shared_rng_from_u64(42);

    let (degree, plaintext_modulus, moduli) = (
        8192usize,
        1000u64,
        &[
            36028797055270913u64,
            36028797054222337u64,
            36028797053698049u64,
            36028797051863041u64,
        ],
    );

    // BFV result: BfvParameters { polynomial_degree: 8192, plaintext_modulus: 1000, moduli: [36028797055270913, 36028797054222337, 36028797053698049, 36028797051863041] }

    let params_raw = build_bfv_params_arc(degree, plaintext_modulus, moduli);
    let params = ArcBytes::from_bytes(encode_bfv_params(&params_raw.clone()));

    reporter.log("BfvParameters", &params.extract_bytes());

    // E3Parameters
    let threshold_m = 24;
    let threshold_n = 50;
    let esi_per_ct = 3;
    let seed = create_seed_from_u64(123);
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let trbfv_config = TrBFVConfig::new(params, threshold_n, threshold_m);
    let error_size = ArcBytes::from_bytes(BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        threshold_n as usize,
        threshold_m as usize,
    )?));

    let crp_raw = create_crp(
        trbfv_config.params(),
        Arc::new(Mutex::new(ChaCha20Rng::from_seed(seed.into()))),
    );

    let shares_hash_map = usecase_helpers::generate_shares_hash_map(
        &trbfv_config,
        esi_per_ct,
        &error_size,
        &crp_raw,
        &rng,
        &cipher,
        &mut reporter,
    )?;

    for share in shares_hash_map.iter() {
        let share = bincode::serialize(&share.1)?;
        reporter.log("ThresholdShare", &share);
    }

    let pubkey =
        usecase_helpers::get_public_key(&shares_hash_map, trbfv_config.params(), &crp_raw)?;

    reporter.log("PublicKey", &pubkey.to_bytes());

    let shares = to_ordered_vec(shares_hash_map);
    let decryption_keys =
        usecase_helpers::get_decryption_keys(shares, &cipher, &trbfv_config, &mut reporter)?;

    for (_, (es_poly_sum, sk_poly_sum)) in decryption_keys.iter() {
        let es_poly_sum = bincode::serialize(&es_poly_sum)?;
        let sk_poly_sum = bincode::serialize(&sk_poly_sum)?;
        reporter.log("es_poly_sum", &es_poly_sum);
        reporter.log("sk_poly_sum", &sk_poly_sum);
        reporter.log("decryption_key", &[es_poly_sum, sk_poly_sum].concat())
    }

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
        .map(|ct| ArcBytes::from_bytes((*ct).clone().to_bytes()))
        .collect::<Vec<ArcBytes>>();

    let joined_ciphertext = ciphertexts
        .clone()
        .iter()
        .map(|ct| ct.extract_bytes())
        .collect::<Vec<_>>()
        .concat();

    reporter.log("Encrypted output from application", &joined_ciphertext);

    let mut decryption_shares = HashMap::new();
    for party_id in 0..=threshold_m as usize {
        // for party_id in [1, 4, 2] {
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
        reporter.log(
            "Single decryption Share",
            &d_share_poly
                .clone()
                .iter()
                .map(|bb| bb.extract_bytes())
                .collect::<Vec<_>>()
                .concat(),
        );
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
    reporter.log_time("Entire test", start.elapsed());
    println!("{}", reporter.to_size_table());
    println!("{}", reporter.to_timing_table());
    Ok(())
}
