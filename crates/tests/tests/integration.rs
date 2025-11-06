// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use alloy::primitives::{FixedBytes, I256, U256};
use anyhow::{bail, Context, Result};
use e3_ciphernode_builder::CiphernodeBuilder;
use e3_crypto::Cipher;
use e3_events::{
    CiphertextOutputPublished, CommitteeFinalized, ConfigurationUpdated, E3Requested, E3id,
    EnclaveEvent, EventBus, EventBusConfig, OperatorActivationChanged, PlaintextAggregated,
    TicketBalanceUpdated,
};
use e3_multithread::Multithread;
use e3_sdk::bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_test_helpers::ciphernode_system::CiphernodeSystemBuilder;
use e3_test_helpers::{create_seed_from_u64, create_shared_rng_from_u64, AddToCommittee};
use e3_trbfv::helpers::calculate_error_size;
use e3_utils::rand_eth_addr;
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

async fn setup_score_sortition_environment(
    bus: &actix::Addr<EventBus<EnclaveEvent>>,
    eth_addrs: &Vec<String>,
    chain_id: u64,
) -> Result<()> {
    bus.send(EnclaveEvent::from(ConfigurationUpdated {
        parameter: "ticketPrice".to_string(),
        old_value: U256::ZERO,
        new_value: U256::from(10_000_000u64),
        chain_id,
    }))
    .await?;

    let mut adder = AddToCommittee::new(bus, chain_id);
    for addr in eth_addrs {
        adder.add(addr).await?;

        bus.send(EnclaveEvent::from(TicketBalanceUpdated {
            operator: addr.clone(),
            delta: I256::try_from(1_000_000_000u64).unwrap(),
            new_balance: U256::from(1_000_000_000u64),
            reason: FixedBytes::ZERO,
            chain_id,
        }))
        .await?;

        bus.send(EnclaveEvent::from(OperatorActivationChanged {
            operator: addr.clone(),
            active: true,
            chain_id,
        }))
        .await?;
    }

    Ok(())
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

    // Create "trigger" bus
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();

    // Parameters (128bits of security)
    let (degree, plaintext_modulus, moduli) = (
        8192,
        1000,
        &[
            36028797055270913,
            36028797054222337,
            36028797053698049,
            36028797051863041,
        ],
    );

    // Params for BFV
    let params_raw = build_bfv_params_arc(degree, plaintext_modulus, moduli, None);

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
                .testmode_with_history()
                .with_trbfv()
                .with_pubkey_aggregation()
                .with_sortition_score()
                .with_threshold_plaintext_aggregation()
                .testmode_with_forked_bus(&bus)
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
                .with_sortition_score()
                .testmode_with_forked_bus(&bus)
                .with_logging()
                .build()
                .await
        })
        .simulate_libp2p()
        .build()
        .await?;

    let chain_id = 1u64;
    let eth_addrs: Vec<String> = nodes.iter().map(|n| n.address()).collect();
    setup_score_sortition_environment(&bus, &eth_addrs, chain_id).await?;

    // Flush all events
    nodes.flush_all_history(100).await?;

    ///////////////////////////////////////////////////////////////////////////////////
    // 2. Trigger E3Requested
    //
    //   - m=2.
    //   - n=5
    //   - error_size -> calculate using calculate_error_size
    //   - esi_per_ciphertext = 3
    ///////////////////////////////////////////////////////////////////////////////////

    // Prepare round

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

    // For score sortition, we need to wait for nodes to process E3Requested and run sortition
    // Since TicketGenerated is a local-only event (not shared across network), we can't collect it
    // we need to manually construct the committee that sortition would select

    // For seed=123, these 5 nodes get selected by sortition:
    // 0x8f32E487328F04927f20c4B14399e4F3123763df (ticket 6)
    // 0x95b8a2b9b93aE9e0F13e215A49b8C53172c4f4ba (ticket 68)
    // 0x8966a013047aef67Cac52Bc96eB77bC11B5D2572 (ticket 95)
    // 0x2B1eD59AC30f668B5b9EcF3D8718A44C15E0E479 (ticket 15)
    // 0x83A06c5Ac9E4207526C3eFA79812808428Dd5FaB (ticket 12)
    let committee: Vec<String> = vec![
        "0x8f32E487328F04927f20c4B14399e4F3123763df".to_string(),
        "0x95b8a2b9b93aE9e0F13e215A49b8C53172c4f4ba".to_string(),
        "0x8966a013047aef67Cac52Bc96eB77bC11B5D2572".to_string(),
        "0x2B1eD59AC30f668B5b9EcF3D8718A44C15E0E479".to_string(),
        "0x83A06c5Ac9E4207526C3eFA79812808428Dd5FaB".to_string(),
    ];

    println!("Emitting CommitteeFinalized with {} nodes", committee.len());

    bus.send(EnclaveEvent::from(CommitteeFinalized {
        e3_id: e3_id.clone(),
        committee,
        chain_id,
    }))
    .await?;

    let expected = vec![
        "E3Requested",
        "CommitteeFinalized",
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
