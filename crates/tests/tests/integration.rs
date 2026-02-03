// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use alloy::primitives::{Address, FixedBytes, I256, U256};
use anyhow::{bail, Result};
use e3_bfv_client::decode_bytes_to_vec_u64;
use e3_ciphernode_builder::{CiphernodeBuilder, EventSystem};
use e3_crypto::Cipher;
use e3_events::{
    prelude::*, BusHandle, CiphertextOutputPublished, CommitteeFinalized, ConfigurationUpdated,
    E3Requested, E3id, EnclaveEvent, EnclaveEventData, OperatorActivationChanged,
    PlaintextAggregated, Seed, TakeEvents, TicketBalanceUpdated,
};
use e3_fhe_params::DEFAULT_BFV_PRESET;
use e3_fhe_params::{encode_bfv_params, BfvParamSet};
use e3_multithread::{Multithread, MultithreadReport, ToReport};
use e3_net::events::{GossipData, NetEvent};
use e3_net::NetEventTranslator;
use e3_sortition::{calculate_buffer_size, RegisteredNode, ScoreSortition, Ticket};
use e3_test_helpers::ciphernode_system::CiphernodeSystemBuilder;
use e3_test_helpers::{create_seed_from_u64, create_shared_rng_from_u64, AddToCommittee};
use e3_trbfv::helpers::calculate_error_size;
use e3_utils::rand_eth_addr;
use e3_utils::utility_types::ArcBytes;
use fhe::bfv::PublicKey;
use fhe_traits::{DeserializeParametrized, Serialize};
use num_bigint::BigUint;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::time::{Duration, Instant};
use std::{fs, sync::Arc};
use tokio::{
    sync::{broadcast, mpsc},
    time::sleep,
};

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

/// Determines the committee for a given E3 request using deterministic sortition.
///
/// This function runs the same sortition algorithm that the ciphernodes use internally,
/// ensuring the test committee matches what the nodes will compute.
///
/// # Arguments
/// * `e3_id` - The E3 computation ID
/// * `seed` - The random seed for sortition
/// * `threshold_m` - Minimum nodes required for decryption
/// * `threshold_n` - Committee size
/// * `registered_addrs` - List of node addresses eligible for selection
/// * `collector_addr` - Address of the collector node (for validation)
///
/// # Returns
/// A tuple of (committee_addresses, buffer_addresses)
fn determine_committee(
    e3_id: &E3id,
    seed: Seed,
    threshold_m: usize,
    threshold_n: usize,
    registered_addrs: &[String],
    collector_addr: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let buffer = calculate_buffer_size(threshold_m, threshold_n);
    let total_selection_size = threshold_n + buffer;

    // Calculate tickets based on the same balance/ticket_price ratio as production
    // ticket_price = 10_000_000, balance = 1_000_000_000
    // => num_tickets = 1_000_000_000 / 10_000_000 = 100 tickets per node
    const TICKET_PRICE: u64 = 10_000_000;
    const BALANCE: u64 = 1_000_000_000;
    let num_tickets = BALANCE / TICKET_PRICE;

    let registered_nodes: Vec<RegisteredNode> = registered_addrs
        .iter()
        .map(|addr| {
            let address: Address = addr.parse().unwrap();
            let tickets: Vec<Ticket> = (0..num_tickets)
                .map(|ticket_id| Ticket { ticket_id })
                .collect();
            RegisteredNode { address, tickets }
        })
        .collect();

    let winners = ScoreSortition::new(total_selection_size).get_committee(
        e3_id.clone(),
        seed,
        &registered_nodes,
    )?;

    let committee: Vec<String> = winners
        .iter()
        .take(threshold_n)
        .map(|w| w.address.to_string())
        .collect();

    let buffer_nodes: Vec<String> = winners
        .iter()
        .skip(threshold_n)
        .map(|w| w.address.to_string())
        .collect();

    for addr in &committee {
        if addr.eq_ignore_ascii_case(collector_addr) {
            bail!(
                "Collector node was selected in committee. \
                 This should never happen as collector should not be registered for sortition.\n\
                 Collector: {}\n\
                 Registered nodes: {}",
                collector_addr,
                registered_addrs.len()
            );
        }
    }

    Ok((committee, buffer_nodes))
}

async fn setup_score_sortition_environment(
    bus: &BusHandle,
    eth_addrs: &Vec<String>,
    chain_id: u64,
) -> Result<()> {
    bus.publish(ConfigurationUpdated {
        parameter: "ticketPrice".to_string(),
        old_value: U256::ZERO,
        new_value: U256::from(10_000_000u64),
        chain_id,
    })?;

    let mut adder = AddToCommittee::new(bus, chain_id);
    for addr in eth_addrs {
        adder.add(addr).await?;

        bus.publish(TicketBalanceUpdated {
            operator: addr.clone(),
            delta: I256::try_from(1_000_000_000u64).unwrap(),
            new_balance: U256::from(1_000_000_000u64),
            reason: FixedBytes::ZERO,
            chain_id,
        })?;

        bus.publish(OperatorActivationChanged {
            operator: addr.clone(),
            active: true,
            chain_id,
        })?;
    }

    Ok(())
}

fn serialize_report(report: &[(&str, Duration)]) -> String {
    let max_key_len = report.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

    report
        .iter()
        .map(|(key, duration)| {
            format!(
                "{:width$}: {:.3}s",
                key,
                duration.as_secs_f64(),
                width = max_key_len
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Test trbfv
#[actix::test]
#[serial_test::serial]
async fn test_trbfv_actor() -> Result<()> {
    println!("Running test_trbfv_actor...");
    let mut report: Vec<(&str, Duration)> = vec![];
    let whole_test = Instant::now();
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

    let setup = Instant::now();

    // Create rng
    let rng = create_shared_rng_from_u64(42);

    // Create "trigger" bus
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;

    // Parameters (128bits of security)
    let params_raw = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();

    // Encoded Params
    let params = ArcBytes::from_bytes(&encode_bfv_params(&params_raw.clone()));

    // round information
    let threshold_m = 2;
    let threshold_n = 5;
    let esi_per_ct = 3;

    // WARNING: INSECURE SECURITY PARAMETER LAMBDA.
    // This is just for INSECURE parameter set.
    // This is not secure and should not be used in production.
    // For production use lambda = 80.
    let lambda = 2;

    let seed = create_seed_from_u64(123);
    let error_size = ArcBytes::from_bytes(&BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        threshold_n,
        threshold_m,
        lambda,
    )?));

    // Cipher
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);

    // Actor system setup
    // Seems like you cannot send more than one job at a time to rayon
    let concurrent_jobs = 1; // leaving at 1
    let max_threadroom = Multithread::get_max_threads_minus(1);
    let task_pool = Multithread::create_taskpool(max_threadroom, concurrent_jobs);
    let multithread_report = MultithreadReport::new(max_threadroom, concurrent_jobs).start();

    let nodes = CiphernodeSystemBuilder::new()
        // Adding 20 total nodes: 5 for committee + 4 buffer = 9 selected, 11 unselected
        .add_group(1, || async {
            let addr = rand_eth_addr(&rng);
            println!("Building collector {}!", addr);
            CiphernodeBuilder::new(&addr, rng.clone(), cipher.clone())
                .with_address(&addr)
                .testmode_with_history()
                .with_shared_taskpool(&task_pool)
                .with_multithread_concurrent_jobs(concurrent_jobs)
                .with_shared_multithread_report(&multithread_report)
                .with_trbfv()
                .with_pubkey_aggregation()
                .with_sortition_score()
                .with_threshold_plaintext_aggregation()
                .testmode_with_forked_bus(bus.event_bus())
                .with_logging()
                .build()
                .await
        })
        .add_group(19, || async {
            let addr = rand_eth_addr(&rng);
            println!("Building normal {}", &addr);
            CiphernodeBuilder::new(&addr, rng.clone(), cipher.clone())
                .with_address(&addr)
                .with_shared_taskpool(&task_pool)
                .with_multithread_concurrent_jobs(concurrent_jobs)
                .with_shared_multithread_report(&multithread_report)
                .with_trbfv()
                .with_sortition_score()
                .testmode_with_forked_bus(bus.event_bus())
                .with_logging()
                .build()
                .await
        })
        .simulate_libp2p()
        .build()
        .await?;

    report.push(("Setup", setup.elapsed()));

    let committee_setup = Instant::now();
    let chain_id = 1u64;

    // Only register nodes 1-19 in sortition (exclude collector at index 0).
    // This ensures the collector is never selected, making the test deterministic.
    // The collector node will observe events as a non-participant.
    let collector_addr = nodes.get(0).unwrap().address();
    let eth_addrs: Vec<String> = nodes
        .iter()
        .skip(1) // Skip the collector node
        .map(|n| n.address())
        .collect();

    println!(
        "Test setup: {} registered nodes, {} threshold, collector (observer): {}",
        eth_addrs.len(),
        threshold_n,
        collector_addr
    );

    setup_score_sortition_environment(&bus, &eth_addrs, chain_id).await?;

    // Flush all events
    nodes.flush_all_history(100).await?;
    report.push(("Committee Setup", committee_setup.elapsed()));

    ///////////////////////////////////////////////////////////////////////////////////
    // 2. Trigger E3Requested
    //
    //   - m=2.
    //   - n=5
    //   - lambda=2
    //   - error_size -> calculate using calculate_error_size
    //   - esi_per_ciphertext = 3
    ///////////////////////////////////////////////////////////////////////////////////

    // Prepare round
    let e3_requested_timer = Instant::now();
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

    println!(
        "Publishing E3Requested: e3_id={}, threshold={}/{}",
        e3_id, threshold_m, threshold_n
    );
    bus.publish(e3_requested)?;

    sleep(Duration::from_millis(500)).await;

    let (committee, buffer_nodes) = determine_committee(
        &e3_id,
        seed,
        threshold_m,
        threshold_n,
        &eth_addrs,
        &collector_addr,
    )?;

    println!(
        "Committee selected: {} nodes, {} buffer nodes",
        committee.len(),
        buffer_nodes.len()
    );

    let expected = vec!["E3Requested"];
    let _ = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;

    bus.publish(CommitteeFinalized {
        e3_id: e3_id.clone(),
        committee: committee.clone(),
        chain_id,
    })?;

    let committee_finalized_timer = Instant::now();

    let expected = vec!["CommitteeFinalized"];
    let _ = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;

    report.push((
        "Committee Finalization",
        committee_finalized_timer.elapsed(),
    ));

    // First, wait for all EncryptionKeyCreated events (BFV key exchange)
    let encryption_keys_timer = Instant::now();
    let expected = vec![
        "EncryptionKeyCreated",
        "EncryptionKeyCreated",
        "EncryptionKeyCreated",
        "EncryptionKeyCreated",
        "EncryptionKeyCreated",
    ];
    let _ = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;
    report.push((
        "All EncryptionKeyCreated events",
        encryption_keys_timer.elapsed(),
    ));

    // Then wait for all ThresholdShareCreated events
    // With domain-level splitting, each of the 5 parties publishes 5 events (one per target party)
    // Total: 5 parties × 5 targets = 25 events
    let shares_timer = Instant::now();
    let expected: Vec<&str> = (0..25).map(|_| "ThresholdShareCreated").collect();
    let _ = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;
    report.push(("All ThresholdShareCreated events", shares_timer.elapsed()));

    let shares_to_pubkey_agg_timer = Instant::now();
    let expected = vec![
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

    report.push((
        "ThresholdShares -> PublicKeyAggregated",
        shares_to_pubkey_agg_timer.elapsed(),
    ));

    report.push((
        "E3Request -> PublicKeyAggregated",
        e3_requested_timer.elapsed(),
    ));
    let app_gen_timer = Instant::now();
    assert_eq!(h.event_types(), expected);
    // Aggregate decryption

    // First we get the public key
    println!("Getting public key");
    let Some(EnclaveEventData::PublicKeyAggregated(pubkey_event)) = h.last().map(|e| e.get_data())
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
    report.push(("Application CT Gen", app_gen_timer.elapsed()));

    let running_app_timer = Instant::now();
    println!("Running application to generate outputs...");
    let outputs = e3_test_helpers::application::run_application(
        &inputs,
        params_raw.clone(),
        num_votes_per_voter,
    );
    report.push(("Running FHE Application", running_app_timer.elapsed()));

    let publishing_ct_timer = Instant::now();
    println!("Have outputs. Creating ciphertexts...");
    let ciphertexts = outputs
        .into_iter()
        .map(|ct| ArcBytes::from_bytes(&(*ct).clone().to_bytes()))
        .collect::<Vec<ArcBytes>>();

    // Created the event
    println!("Publishing CiphertextOutputPublished...");
    let ciphertext_published_event = CiphertextOutputPublished {
        ciphertext_output: ciphertexts,
        e3_id: e3_id.clone(),
    };

    bus.publish(ciphertext_published_event.clone())?;

    println!("CiphertextOutputPublished event has been dispatched!");

    // Lets grab decryption share events
    let expected = vec![
        "CiphertextOutputPublished",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "ComputeRequest",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "ComputeResponse",
        "PlaintextAggregated",
    ];

    let h = nodes
        .take_history_with_timeout(0, expected.len(), Duration::from_secs(1000))
        .await?;

    assert_eq!(h.event_types(), expected);
    report.push((
        "Ciphertext published -> PlaintextAggregated",
        publishing_ct_timer.elapsed(),
    ));

    let Some(EnclaveEventData::PlaintextAggregated(PlaintextAggregated {
        decrypted_output: plaintext,
        ..
    })) = h.last().map(|e| e.get_data())
    else {
        bail!("bad event")
    };

    let results = plaintext
        .into_iter()
        .map(|a| decode_bytes_to_vec_u64(&a.extract_bytes()).expect("error decoding bytes"))
        .collect::<Vec<Vec<u64>>>();

    let results: Vec<u64> = results
        .into_iter()
        .map(|r| r.first().unwrap().clone())
        .collect();

    // Show summation result (mod plaintext modulus)
    let plaintext_modulus = params_raw.clone().plaintext();
    let mut expected_result = vec![0u64; 3];
    for vals in &numbers {
        for j in 0..num_votes_per_voter {
            expected_result[j] = (expected_result[j] + vals[j]) % plaintext_modulus;
        }
    }

    for (i, (res, exp)) in results.iter().zip(expected_result.iter()).enumerate() {
        println!("Tally {i} result = {res} / {exp}");
        assert_eq!(res, exp);
    }

    let mt_report = multithread_report.send(ToReport).await.unwrap();
    println!("{}", mt_report);

    report.push(("Entire Test", whole_test.elapsed()));
    println!("{}", serialize_report(&report));

    Ok(())
}

// ============================================================================
// Networking and P2P Tests
// ============================================================================

#[actix::test]
async fn test_p2p_actor_forwards_events_to_network() -> Result<()> {
    use e3_events::{CiphernodeSelected, EnclaveEvent, TakeEvents, Unsequenced};
    use e3_net::events::GossipData;
    use e3_net::{events::NetEvent, NetEventTranslator};
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::{broadcast, Mutex};

    // Setup elements in test
    let (cmd_tx, mut cmd_rx) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, _) = broadcast::channel(100); // Receive byte events from the network
    let system = EventSystem::new("test");
    let bus = system.handle()?;
    let history_collector = bus.history();
    let event_rx = Arc::new(event_tx.subscribe());
    // Pas cmd and event channels to NetEventTranslator
    NetEventTranslator::setup(&bus, &cmd_tx, &event_rx, "my-topic");

    // Capture messages from output on msgs vec
    let msgs: Arc<Mutex<Vec<EnclaveEventData>>> = Arc::new(Mutex::new(Vec::new()));

    let msgs_loop = msgs.clone();

    tokio::spawn(async move {
        // Pull events from command channel
        while let Some(cmd) = cmd_rx.recv().await {
            // If the command is a GossipPublish then extract it and save it whilst sending it to
            // the event bus as if it was gossiped from the network and ended up as an external
            // message this simulates a rebroadcast message
            if let Some(msg) = match cmd {
                e3_net::events::NetCommand::GossipPublish { data, .. } => Some(data),
                _ => None,
            } {
                if let GossipData::GossipBytes(_) = msg {
                    let event: EnclaveEvent<Unsequenced> = msg.clone().try_into().unwrap();
                    let (data, _) = event.split();
                    msgs_loop.lock().await.push(data);
                    event_tx.send(NetEvent::GossipData(msg)).unwrap();
                }
            }
            // if this  manages to broadcast an event to the
            // event bus we will expect to see an extra event on
            // the bus but we don't because we handle this
        }
        anyhow::Ok(())
    });

    let evt_1 = PlaintextAggregated {
        e3_id: E3id::new("1235", 1),
        decrypted_output: vec![ArcBytes::from_bytes(&[1, 2, 3, 4])],
    };

    let evt_2 = PlaintextAggregated {
        e3_id: E3id::new("1236", 1),
        decrypted_output: vec![ArcBytes::from_bytes(&[1, 2, 3, 4])],
    };

    let local_evt_3 = CiphernodeSelected {
        e3_id: E3id::new("1235", 1),
        threshold_m: 2,
        threshold_n: 5,
        ..CiphernodeSelected::default()
    };

    bus.publish(evt_1.clone())?;
    bus.publish(evt_2.clone())?;
    bus.publish(local_evt_3.clone())?; // This is a local event which should not be broadcast to the network

    // check the history of the event bus
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(3))
        .await?;

    assert_eq!(
        *msgs.lock().await,
        vec![evt_1.clone().into(), evt_2.clone().into()], // notice no local events
        "NetEventTranslator did not transmit correct events to the network"
    );

    assert_eq!(
        history
            .into_iter()
            .map(|e| e.into_data())
            .collect::<Vec<_>>(),
        vec![evt_1.into(), evt_2.into(), local_evt_3.into()], // all local events that have been broadcast but no
        // events from the loopback
        "NetEventTranslator must not retransmit forwarded event to event bus"
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_bus() -> Result<()> {
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    // Setup elements in test
    let (cmd_tx, _) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, event_rx) = broadcast::channel(100); // Receive byte events from the network
    let system = EventSystem::new("test").with_fresh_bus();
    let bus = system.handle()?;
    let history_collector = bus.history();

    NetEventTranslator::setup(&bus, &cmd_tx, &Arc::new(event_rx), "mytopic");

    // Capture messages from output on msgs vec
    let event = E3Requested {
        e3_id: E3id::new("1235", 1),
        threshold_m: 3,
        threshold_n: 3,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&[1, 2, 3, 4]),
        ..E3Requested::default()
    };

    // lets send an event from the network
    let _ = event_tx.send(NetEvent::GossipData(GossipData::GossipBytes(
        bus.event_from(event.clone(), None)?.to_bytes()?,
    )));

    // check the history of the event bus
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(1))
        .await?;

    assert_eq!(
        history
            .into_iter()
            .map(|e| e.into_data())
            .collect::<Vec<EnclaveEventData>>(),
        vec![event.into()]
    );

    Ok(())
}

// ============================================================================
// Legacy Tests Pending Port to trBFV
// ============================================================================

/// Test that stopped keyshares retain their state after restart.
/// This test needs to be ported to the new trBFV system once Sync is completed.
#[actix::test]
#[ignore = "Needs to be ported to trBFV system after Sync is completed"]
async fn test_stopped_keyshares_retain_state() -> Result<()> {
    use e3_bfv_client::{decode_bytes_to_vec_u64, decode_plaintext_to_vec_u64};
    use e3_data::{GetDump, InMemStore};
    use e3_events::{EventBus, EventBusConfig, GetEvents, Shutdown, TakeEvents};
    use e3_test_helpers::{create_random_eth_addrs, get_common_setup, simulate_libp2p_net};
    use fhe::{
        bfv::PublicKey,
        mbfv::{AggregateIter, PublicKeyShare},
    };
    use fhe_traits::Serialize;
    use std::time::Duration;
    use tokio::time::sleep;

    async fn setup_local_ciphernode(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        logging: bool,
        addr: &str,
        store: Option<actix::Addr<InMemStore>>,
        cipher: &Arc<Cipher>,
    ) -> Result<e3_ciphernode_builder::CiphernodeHandle> {
        let mut builder = CiphernodeBuilder::new(&addr, rng.clone(), cipher.clone())
            .with_trbfv()
            .with_address(addr)
            .testmode_with_forked_bus(bus.event_bus())
            .testmode_with_history()
            .testmode_with_errors()
            .with_pubkey_aggregation()
            .with_threshold_plaintext_aggregation()
            .with_sortition_score();

        if let Some(ref in_mem_store) = store {
            builder = builder.with_in_mem_datastore(in_mem_store);
        }

        if logging {
            builder = builder.with_logging()
        }

        let node = builder.build().await?;
        Ok(node)
    }

    async fn create_local_ciphernodes(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        count: u32,
        cipher: &Arc<Cipher>,
    ) -> Result<Vec<e3_ciphernode_builder::CiphernodeHandle>> {
        let eth_addrs = create_random_eth_addrs(count);
        let mut result = vec![];
        for addr in &eth_addrs {
            println!("Setting up eth addr: {}", addr);
            let tuple = setup_local_ciphernode(&bus, &rng, true, addr, None, cipher).await?;
            result.push(tuple);
        }
        simulate_libp2p_net(&result);
        Ok(result)
    }

    let e3_id = E3id::new("1234", 1);
    let (rng, cn1_address, cn1_data, cn2_address, cn2_data, cipher, history, params, crpoly) = {
        let (bus, rng, seed, params, crpoly, _, _) = get_common_setup(None)?;
        let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);
        let ciphernodes = create_local_ciphernodes(&bus, &rng, 2, &cipher).await?;
        let eth_addrs = ciphernodes.iter().map(|n| n.address()).collect::<Vec<_>>();

        setup_score_sortition_environment(&bus, &eth_addrs, 1).await?;

        let [cn1, cn2] = &ciphernodes.as_slice() else {
            panic!("Not enough elements")
        };

        // Send e3request
        bus.publish(E3Requested {
            e3_id: e3_id.clone(),
            threshold_m: 2,
            threshold_n: 2,
            seed: seed.clone(),
            params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
            ..E3Requested::default()
        })?;

        bus.publish(CommitteeFinalized {
            e3_id: e3_id.clone(),
            committee: eth_addrs.clone(),
            chain_id: 1,
        })?;

        let history_collector = cn1.history().unwrap();
        let error_collector = cn1.errors().unwrap();
        let history = history_collector
            .send(TakeEvents::<e3_events::EnclaveEvent>::new(14))
            .await?;
        let errors = error_collector.send(GetEvents::new()).await?;

        assert_eq!(errors.len(), 0);

        // SEND SHUTDOWN!
        bus.publish(Shutdown)?;

        // This is probably overkill but required to ensure that all the data is written
        sleep(Duration::from_secs(1)).await;

        // Unwrap does not matter as we are in a test
        let cn1_dump = cn1.in_mem_store().unwrap().send(GetDump).await??;
        let cn2_dump = cn2.in_mem_store().unwrap().send(GetDump).await??;

        (
            rng,
            cn1.address(),
            cn1_dump,
            cn2.address(),
            cn2_dump,
            cipher,
            history,
            params,
            crpoly,
        )
    };

    let bus = EventSystem::in_mem("cn2")
        .with_event_bus(
            EventBus::<e3_events::EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start(),
        )
        .handle()?;
    let cn1 = setup_local_ciphernode(
        &bus,
        &rng,
        true,
        &cn1_address,
        Some(InMemStore::from_dump(cn1_data, true)?.start()),
        &cipher,
    )
    .await?;
    let cn2 = setup_local_ciphernode(
        &bus,
        &rng,
        true,
        &cn2_address,
        Some(InMemStore::from_dump(cn2_data, true)?.start()),
        &cipher,
    )
    .await?;
    let history_collector = cn1.history().unwrap();
    simulate_libp2p_net(&[cn1, cn2]);

    println!("getting collector from cn1.6");

    // get the public key from history.
    let pubkey: PublicKey = history
        .iter()
        .filter_map(|evt| match evt.get_data() {
            EnclaveEventData::KeyshareCreated(data) => {
                PublicKeyShare::deserialize(&data.pubkey, &params, crpoly.clone()).ok()
            }
            _ => None,
        })
        .aggregate()?;

    // Publish the ciphertext
    use e3_test_helpers::encrypt_ciphertext;
    let raw_plaintext = vec![vec![4, 5]];
    let (ciphertext, expected) = encrypt_ciphertext(&params, pubkey, raw_plaintext)?;
    bus.publish(CiphertextOutputPublished {
        ciphertext_output: ciphertext
            .iter()
            .map(|ct| ArcBytes::from_bytes(&ct.to_bytes()))
            .collect(),
        e3_id: e3_id.clone(),
    })?;

    let history = history_collector
        .send(TakeEvents::<e3_events::EnclaveEvent>::new(5))
        .await?;

    let actual = history
        .into_iter()
        .filter_map(|e| match e.into_data() {
            EnclaveEventData::PlaintextAggregated(data) => Some(data),
            _ => None,
        })
        .collect::<Vec<_>>()
        .first()
        .unwrap()
        .clone();

    assert_eq!(
        actual
            .decrypted_output
            .iter()
            .map(|b| decode_bytes_to_vec_u64(b).unwrap())
            .collect::<Vec<Vec<u64>>>(),
        expected
            .iter()
            .map(|p| decode_plaintext_to_vec_u64(p).unwrap())
            .collect::<Vec<Vec<u64>>>()
    );

    Ok(())
}

/// Test that duplicate E3 IDs work correctly with different chain IDs.
/// This test needs to be ported to use trBFV instead of legacy keyshare.
#[actix::test]
#[ignore = "Needs to be ported to trBFV system"]
async fn test_duplicate_e3_id_with_different_chain_id() -> Result<()> {
    use e3_bfv_client::compute_pk_commitment;
    use e3_events::{OrderedSet, PublicKeyAggregated, TakeEvents};
    use e3_test_helpers::{
        create_random_eth_addrs, create_shared_rng_from_u64, get_common_setup, simulate_libp2p_net,
    };
    use fhe::{
        bfv::{BfvParameters, PublicKey, SecretKey},
        mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
    };
    use fhe_traits::Serialize;

    type PkSkShareTuple = (PublicKeyShare, SecretKey, String);

    async fn setup_local_ciphernode(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        logging: bool,
        addr: &str,
        store: Option<actix::Addr<e3_data::InMemStore>>,
        cipher: &Arc<Cipher>,
    ) -> Result<e3_ciphernode_builder::CiphernodeHandle> {
        let mut builder = CiphernodeBuilder::new(&addr, rng.clone(), cipher.clone())
            .with_trbfv()
            .with_address(addr)
            .testmode_with_forked_bus(bus.event_bus())
            .testmode_with_history()
            .testmode_with_errors()
            .with_pubkey_aggregation()
            .with_threshold_plaintext_aggregation()
            .with_sortition_score();

        if let Some(ref in_mem_store) = store {
            builder = builder.with_in_mem_datastore(in_mem_store);
        }

        if logging {
            builder = builder.with_logging()
        }

        let node = builder.build().await?;
        Ok(node)
    }

    async fn create_local_ciphernodes(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        count: u32,
        cipher: &Arc<Cipher>,
    ) -> Result<Vec<e3_ciphernode_builder::CiphernodeHandle>> {
        let eth_addrs = create_random_eth_addrs(count);
        let mut result = vec![];
        for addr in &eth_addrs {
            println!("Setting up eth addr: {}", addr);
            let tuple = setup_local_ciphernode(&bus, &rng, true, addr, None, cipher).await?;
            result.push(tuple);
        }
        simulate_libp2p_net(&result);
        Ok(result)
    }

    fn generate_pk_share(
        params: &Arc<BfvParameters>,
        crp: &CommonRandomPoly,
        rng: &e3_utils::SharedRng,
        addr: &str,
    ) -> Result<PkSkShareTuple> {
        let sk = SecretKey::random(&params, &mut *rng.lock().unwrap());
        let pk = PublicKeyShare::new(&sk, crp.clone(), &mut *rng.lock().unwrap())?;
        Ok((pk, sk, addr.to_owned()))
    }

    fn generate_pk_shares(
        params: &Arc<BfvParameters>,
        crp: &CommonRandomPoly,
        rng: &e3_utils::SharedRng,
        eth_addrs: &Vec<String>,
    ) -> Result<Vec<PkSkShareTuple>> {
        let mut result = vec![];
        for addr in eth_addrs {
            result.push(generate_pk_share(params, crp, rng, addr)?);
        }
        Ok(result)
    }

    fn aggregate_public_key(shares: &Vec<PkSkShareTuple>) -> Result<PublicKey> {
        Ok(shares
            .clone()
            .into_iter()
            .map(|(pk, _, _)| pk)
            .aggregate()?)
    }

    // Setup
    let (bus, rng, seed, params, crpoly, _, _) = get_common_setup(None)?;
    let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);

    // Setup actual ciphernodes and dispatch add events
    let ciphernodes = create_local_ciphernodes(&bus, &rng, 3, &cipher).await?;
    let eth_addrs = ciphernodes.iter().map(|tup| tup.address()).collect();

    setup_score_sortition_environment(&bus, &eth_addrs, 1).await?;
    setup_score_sortition_environment(&bus, &eth_addrs, 2).await?;

    // Send the computation requested event
    bus.publish(E3Requested {
        e3_id: E3id::new("1234", 1),
        threshold_m: 2,
        threshold_n: 5,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        ..E3Requested::default()
    })?;

    bus.publish(CommitteeFinalized {
        e3_id: E3id::new("1234", 1),
        committee: eth_addrs.clone(),
        chain_id: 1,
    })?;

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;
    let public_key_hash = compute_pk_commitment(
        test_pubkey.to_bytes(),
        params.degree(),
        params.plaintext(),
        params.moduli().to_vec(),
    )?;

    let history_collector = ciphernodes.last().unwrap().history().unwrap();
    let history = history_collector
        .send(TakeEvents::<e3_events::EnclaveEvent>::new(28))
        .await?;

    assert_eq!(
        history.last().cloned().unwrap().into_data(),
        PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            public_key_hash,
            e3_id: E3id::new("1234", 1),
            nodes: OrderedSet::from(eth_addrs.clone()),
        }
        .into()
    );

    // Send the computation requested event
    bus.publish(E3Requested {
        e3_id: E3id::new("1234", 2),
        threshold_m: 2,
        threshold_n: 5,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        ..E3Requested::default()
    })?;

    bus.publish(CommitteeFinalized {
        e3_id: E3id::new("1234", 2),
        committee: eth_addrs.clone(),
        chain_id: 2,
    })?;

    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;

    let public_key_hash = compute_pk_commitment(
        test_pubkey.to_bytes(),
        params.degree(),
        params.plaintext(),
        params.moduli().to_vec(),
    )?;

    let history = history_collector
        .send(TakeEvents::<e3_events::EnclaveEvent>::new(8))
        .await?;

    assert_eq!(
        history.last().cloned().unwrap().into_data(),
        PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            public_key_hash,
            e3_id: E3id::new("1234", 2),
            nodes: OrderedSet::from(eth_addrs.clone()),
        }
        .into()
    );

    Ok(())
}
