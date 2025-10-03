// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use anyhow::{bail, Context, Result};
use e3_crypto::Cipher;
use e3_events::{
    CiphertextOutputPublished, E3Requested, E3id, EnclaveEvent, EventBus, EventBusConfig,
    PlaintextAggregated, ThresholdShare,
};
use e3_multithread::Multithread;
use e3_sdk::bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_test_helpers::ciphernode_builder::CiphernodeBuilder;
use e3_test_helpers::ciphernode_system::CiphernodeSystemBuilder;
use e3_test_helpers::{
    create_crp_from_seed, create_seed_from_u64, create_shared_rng_from_u64, rand_eth_addr,
    usecase_helpers, AddToCommittee,
};
use e3_trbfv::helpers::calculate_error_size;
use e3_trbfv::{trbfv_config, TrBFVConfig};
use e3_utils::utility_types::ArcBytes;
use fhe::bfv::PublicKey;
use fhe_traits::{DeserializeParametrized, Serialize};
use num_bigint::BigUint;
use std::time::Duration;
use std::{fs, sync::Arc};

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

/// Test trbfv
#[actix::test]
#[serial_test::serial]
async fn test_trbfv_actor() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_test_writer()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    // NOTE: Here we are trying to make it as clear as possible as to what is going on so attempting to
    // avoid over abstracting test helpers and favouring straight forward single descriptive
    // functions alongside explanations

    ///////////////////////////////////////////////////////////////////////////////////
    // 1. Setup ThresholdKeyshare system
    //
    //   - E3Router
    //   - ThresholdKeyshare
    //   - Multithread actor
    //   - 7 nodes (so as to check for some nodes not getting selected)
    //   - Loopback libp2p simulation
    ///////////////////////////////////////////////////////////////////////////////////

    // Create rng
    let rng = create_shared_rng_from_u64(42);

    // Create test rng for testing
    let test_rng = create_shared_rng_from_u64(42);

    // Create "trigger" bus
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();

    // Parameteres
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
    // Params for BFV
    let params_raw = build_bfv_params_arc(degree, plaintext_modulus, moduli);

    // Encoded Params
    let params = ArcBytes::from_bytes(encode_bfv_params(&params_raw.clone()));

    // round information
    let threshold_m = 2;
    let threshold_n = 5;
    let esi_per_ct = 3;
    let seed = create_seed_from_u64(123);
    let error_size = ArcBytes::from_bytes(BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        threshold_n,
        threshold_m,
    )?));

    // Cipher
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let mut adder = AddToCommittee::new(&bus, 1);

    // Actor system setup
    let multithread = Multithread::attach(
        rng.clone(),
        cipher.clone(),
        // Multithread::get_max_threads_minus(2),
        1, // TODO: There is a bug running multithread around thread starvation. We may have to
           // setup a queue
    );

    let nodes = CiphernodeSystemBuilder::new()
        // Adding 7 total nodes of which we are only choosing 5 for the committee
        .add_group(1, || async {
            let addr = rand_eth_addr(&rng);
            println!("Building collector {}!", addr);
            CiphernodeBuilder::new(rng.clone(), cipher.clone())
                .with_address(&addr)
                .with_injected_multithread(multithread.clone())
                .with_history()
                .with_trbfv()
                .with_pubkey_aggregation()
                .with_threshold_plaintext_aggregation()
                .with_source_bus(&bus)
                .with_logging()
                .build()
                .await
        })
        .add_group(6, || async {
            let addr = rand_eth_addr(&rng);
            println!("Building normal {}", &addr);
            CiphernodeBuilder::new(rng.clone(), cipher.clone())
                .with_address(&addr)
                .with_injected_multithread(multithread.clone())
                .with_trbfv()
                .with_source_bus(&bus)
                .with_logging()
                .build()
                .await
        })
        .simulate_libp2p()
        .build()
        .await?;

    for node in nodes.iter() {
        adder.add(&node.address()).await?;
    }

    // Flush all events
    nodes.flush_all_history(100).await?;

    ///////////////////////////////////////////////////////////////
    // RUN TEST CALCULATION
    ///////////////////////////////////////////////////////////////

    let test_trbfv_config =
        TrBFVConfig::new(params.clone(), threshold_n as u64, threshold_m as u64);
    let test_crp = create_crp_from_seed(&test_trbfv_config.params(), &seed)?;

    let shares_hash_map = usecase_helpers::generate_shares_hash_map(
        &test_trbfv_config,
        esi_per_ct,
        &error_size,
        &test_crp,
        &test_rng,
        &cipher,
    )?;

    let test_pubkey =
        usecase_helpers::get_public_key(&shares_hash_map, test_trbfv_config.params(), &test_crp)?;

    ///////////////////////////////////////////////////////////////////////////////////
    // 2. Trigger E3Requested
    //
    //   - m=2.
    //   - n=5
    //   - error_size -> calculate using calculate_error_size
    //   - esi_per_ciphertext = 3
    ///////////////////////////////////////////////////////////////////////////////////

    // Prepare round

    // Calculate Error Size for E3Program (this will be done by the E3Program implementor)

    // Trigger actor DKG
    let e3_id = E3id::new("0", 1);

    let e3_requested = E3Requested {
        e3_id: e3_id.clone(),
        threshold_m,
        threshold_n,
        seed: seed.clone(),
        error_size,
        esi_per_ct: esi_per_ct as usize,
        params,
    };

    let event = EnclaveEvent::from(e3_requested);

    bus.do_send(event);

    // NOTE: We are using node 0 as the aggregator but it is not selected in this seed which is why
    // there is no CiphernodeSelected event
    let expected = vec![
        "E3Requested",
        "ThresholdShareCreated",
        "ThresholdShareCreated",
        "ThresholdShareCreated",
        "ThresholdShareCreated",
        "ThresholdShareCreated",
        "KeyshareCreated",
        "KeyshareCreated",
        "KeyshareCreated",
        "KeyshareCreated",
        "KeyshareCreated",
        "PublicKeyAggregated",
    ];

    let h = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;

    assert_eq!(h.event_types(), expected);

    let threshold_events: Vec<Arc<ThresholdShare>> = h
        .iter()
        .cloned()
        .filter_map(|e| match e {
            EnclaveEvent::ThresholdShareCreated { data, .. } => Some(data.share),
            _ => None,
        })
        .collect();

    println!(
        "{:?}",
        threshold_events
            .iter()
            .map(|d| d.party_id)
            .collect::<Vec<u64>>()
    );

    // Aggregate decryption

    // First we get the public key
    println!("Getting public key");
    let Some(EnclaveEvent::PublicKeyAggregated {
        data: pubkey_event, ..
    }) = h.last().clone()
    else {
        panic!("Was expecting event to be PublicKeyAggregated");
    };

    let pubkey_bytes = pubkey_event.pubkey.clone();
    let pubkey = PublicKey::from_bytes(&pubkey_bytes, &params_raw)?;

    // assert_eq!(pubkey, test_pubkey, "Pubkeys were not equal");

    println!("Generating inputs this takes some time...");

    // Create the inputs
    let num_votes_per_voter = 3;
    let num_voters = 30;
    let (inputs, numbers) = e3_test_helpers::application::generate_ciphertexts(
        &pubkey,
        params_raw.clone(),
        num_voters,
        num_votes_per_voter,
    );

    println!("Running application to generate outputs...");
    let outputs =
        e3_test_helpers::application::run_application(&inputs, params_raw, num_votes_per_voter);

    println!("Have outputs. Creating ciphertexts...");
    let ciphertexts = outputs
        .into_iter()
        .map(|ct| ArcBytes::from_bytes((*ct).clone().to_bytes()))
        .collect::<Vec<ArcBytes>>();

    // Created the event
    println!("Publishing CiphertextOutputPublished...");
    let ciphertext_published_event = EnclaveEvent::from(CiphertextOutputPublished {
        ciphertext_output: ciphertexts,
        e3_id: e3_id.clone(),
    });

    bus.send(ciphertext_published_event.clone()).await?;

    println!("CiphertextOutputPublished event has been dispatched!");

    // Lets grab decryption share events
    let expected = vec![
        "CiphertextOutputPublished",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "PlaintextAggregated",
    ];

    let h = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;

    assert_eq!(h.event_types(), expected);

    let Some(EnclaveEvent::PlaintextAggregated {
        data:
            PlaintextAggregated {
                decrypted_output: plaintext,
                ..
            },
        ..
    }) = h.last()
    else {
        bail!("bad event")
    };

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
