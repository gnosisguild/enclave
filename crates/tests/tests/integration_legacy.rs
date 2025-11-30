// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use actix::Actor;
use alloy::primitives::{FixedBytes, I256, U256};
use anyhow::*;
use e3_ciphernode_builder::CiphernodeBuilder;
use e3_ciphernode_builder::CiphernodeHandle;
use e3_crypto::Cipher;
use e3_data::GetDump;
use e3_data::InMemStore;
use e3_events::EnclaveEventData;
use e3_events::BusHandle;
use e3_events::GetEvents;
use e3_events::{
    prelude::*, CiphernodeSelected, CiphertextOutputPublished, CommitteeFinalized,
    ConfigurationUpdated, E3Requested, E3id, EnclaveEvent, EventBus, EventBusConfig,
    HistoryCollector, OperatorActivationChanged, OrderedSet, PlaintextAggregated,
    PublicKeyAggregated, Seed, Shutdown, TakeEvents, TicketBalanceUpdated,
};
use e3_net::events::GossipData;
use e3_net::{events::NetEvent, NetEventTranslator};
use e3_sdk::bfv_helpers::decode_bytes_to_vec_u64;
use e3_sdk::bfv_helpers::decode_plaintext_to_vec_u64;
use e3_sdk::bfv_helpers::encode_bfv_params;
use e3_test_helpers::encrypt_ciphertext;
use e3_test_helpers::{
    create_random_eth_addrs, create_shared_rng_from_u64, get_common_setup, simulate_libp2p_net,
    AddToCommittee,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::SharedRng;
use fhe::{
    bfv::{BfvParameters, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
};
use fhe_traits::Serialize;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;

async fn setup_local_ciphernode(
    bus: &BusHandle<EnclaveEvent>,
    rng: &SharedRng,
    logging: bool,
    addr: &str,
    data: Option<Addr<InMemStore>>,
    cipher: &Arc<Cipher>,
) -> Result<CiphernodeHandle> {
    let mut builder = CiphernodeBuilder::new(rng.clone(), cipher.clone())
        .with_keyshare()
        .with_address(addr)
        .testmode_with_forked_bus(&bus.bus())
        .testmode_with_history()
        .testmode_with_errors()
        .with_pubkey_aggregation()
        .with_plaintext_aggregation()
        .with_sortition_score();

    if let Some(data) = data {
        builder = builder.with_datastore((&data).into());
    }

    if logging {
        builder = builder.with_logging()
    }

    let node = builder.build().await?;

    Ok(node)
}

fn generate_pk_share(
    params: &Arc<BfvParameters>,
    crp: &CommonRandomPoly,
    rng: &SharedRng,
    addr: &str,
) -> Result<PkSkShareTuple> {
    let sk = SecretKey::random(&params, &mut *rng.lock().unwrap());
    let pk = PublicKeyShare::new(&sk, crp.clone(), &mut *rng.lock().unwrap())?;
    Ok((pk, sk, addr.to_owned()))
}

fn generate_pk_shares(
    params: &Arc<BfvParameters>,
    crp: &CommonRandomPoly,
    rng: &SharedRng,
    eth_addrs: &Vec<String>,
) -> Result<Vec<PkSkShareTuple>> {
    let mut result = vec![];
    for addr in eth_addrs {
        result.push(generate_pk_share(params, crp, rng, addr)?);
    }
    Ok(result)
}

async fn create_local_ciphernodes(
    bus: &BusHandle<EnclaveEvent>,
    rng: &SharedRng,
    count: u32,
    cipher: &Arc<Cipher>,
) -> Result<Vec<CiphernodeHandle>> {
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

async fn setup_score_sortition_environment(
    bus: &BusHandle<EnclaveEvent>,
    eth_addrs: &Vec<String>,
    chain_id: u64,
) -> Result<()> {
    bus.dispatch(ConfigurationUpdated {
        parameter: "ticketPrice".to_string(),
        old_value: U256::ZERO,
        new_value: U256::from(10_000_000u64),
        chain_id,
    });

    let mut adder = AddToCommittee::new(bus, chain_id);
    for addr in eth_addrs {
        adder.add(addr).await?;

        bus.dispatch(TicketBalanceUpdated {
            operator: addr.clone(),
            delta: I256::try_from(1_000_000_000u64).unwrap(),
            new_balance: U256::from(1_000_000_000u64),
            reason: FixedBytes::ZERO,
            chain_id,
        });

        bus.dispatch(OperatorActivationChanged {
            operator: addr.clone(),
            active: true,
            chain_id,
        });
    }

    Ok(())
}

// Type for our tests to test against
type PkSkShareTuple = (PublicKeyShare, SecretKey, String);

fn aggregate_public_key(shares: &Vec<PkSkShareTuple>) -> Result<PublicKey> {
    Ok(shares
        .clone()
        .into_iter()
        .map(|(pk, _, _)| pk)
        .aggregate()?)
}

#[actix::test]
async fn test_public_key_aggregation_and_decryption() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_test_writer()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);

    // Setup
    let (bus, rng, seed, params, crpoly, _, _) = get_common_setup(None)?;
    let e3_id = E3id::new("1234", 1);
    let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);

    // Setup actual ciphernodes and dispatch add events
    let ciphernodes = create_local_ciphernodes(&bus, &rng, 3, &cipher).await?;
    let eth_addrs = ciphernodes
        .iter()
        .map(|tup| tup.address().to_owned())
        .collect::<Vec<_>>();

    println!("Adding ciphernodes...");

    setup_score_sortition_environment(&bus, &eth_addrs, 1).await?;

    let e3_request_event = E3Requested {
        e3_id: e3_id.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        seed: seed.clone(),
        threshold_m: 3,
        threshold_n: 3, // Need to use n now to suggest committee size
        ..E3Requested::default()
    };

    println!("Sending E3 event...");
    // Send the computation requested event
    bus.dispatch(e3_request_event.clone());

    // Test that we cannot send the same event twice
    bus.dispatch(e3_request_event.clone());

    // Finalize committee with all available nodes
    bus.dispatch(CommitteeFinalized {
        e3_id: e3_id.clone(),
        committee: eth_addrs.clone(),
        chain_id: 1,
    });

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_shares = generate_pk_shares(&params, &crpoly, &rng_test, &eth_addrs)?;
    let test_pubkey = aggregate_public_key(&test_shares)?;

    let expected_aggregated_event = PublicKeyAggregated {
        pubkey: test_pubkey.to_bytes(),
        e3_id: e3_id.clone(),
        nodes: OrderedSet::from(eth_addrs.clone()),
    };

    let history_collector = ciphernodes.get(2).unwrap().history().unwrap();
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(18))
        .await?;

    let aggregated_event: Vec<_> = history
        .into_iter()
        .filter_map(|e| match e.into_data() {
            EnclaveEventData::PublicKeyAggregated(data) => Some(data),
            _ => None,
        })
        .collect();

    assert!(
        !aggregated_event.is_empty(),
        "No PublicKeyAggregated event found"
    );
    assert_eq!(aggregated_event.last().unwrap(), &expected_aggregated_event);
    println!("Aggregating decryption...");
    // Aggregate decryption

    let raw_plaintext = vec![vec![1234, 567890]];
    let (ciphertext, expected) = encrypt_ciphertext(&params, test_pubkey, raw_plaintext)?;

    // Setup Ciphertext Published Event
    let ciphertext_published_event = CiphertextOutputPublished {
        ciphertext_output: ciphertext
            .iter()
            .map(|ct| ArcBytes::from_bytes(&ct.to_bytes()))
            .collect(),
        e3_id: e3_id.clone(),
    };

    bus.dispatch(ciphertext_published_event.clone());

    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(6))
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

#[actix::test]
async fn test_stopped_keyshares_retain_state() -> Result<()> {
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
        bus.dispatch(E3Requested {
            e3_id: e3_id.clone(),
            threshold_m: 2,
            threshold_n: 2,
            seed: seed.clone(),
            params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
            ..E3Requested::default()
        });

        bus.dispatch(CommitteeFinalized {
            e3_id: e3_id.clone(),
            committee: eth_addrs.clone(),
            chain_id: 1,
        });

        let history_collector = cn1.history().unwrap();
        let error_collector = cn1.errors().unwrap();
        let history = history_collector
            .send(TakeEvents::<EnclaveEvent>::new(14))
            .await?;
        let errors = error_collector.send(GetEvents::new()).await?;

        assert_eq!(errors.len(), 0);

        // SEND SHUTDOWN!
        bus.dispatch(Shutdown);

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

    // Reset history

    // Get the address and the data actor from the two ciphernodes
    // and rehydrate them to new actors

    // Apply the address and data node to two new actors
    // Here we test that hydration occurred sucessfully
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true })
        .start()
        .into();
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
    let raw_plaintext = vec![vec![1234, 567890]];
    let (ciphertext, expected) = encrypt_ciphertext(&params, pubkey, raw_plaintext)?;
    bus.dispatch(CiphertextOutputPublished {
        ciphertext_output: ciphertext
            .iter()
            .map(|ct| ArcBytes::from_bytes(&ct.to_bytes()))
            .collect(),
        e3_id: e3_id.clone(),
    });

    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(5))
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

#[actix::test]
async fn test_p2p_actor_forwards_events_to_network() -> Result<()> {
    // Setup elements in test
    let (cmd_tx, mut cmd_rx) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, _) = broadcast::channel(100); // Receive byte events from the network
    let bus: BusHandle<EnclaveEvent> =
        EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true })
            .start()
            .into();
    let history_collector = HistoryCollector::<EnclaveEvent>::new().start();
    bus.subscribe("*", history_collector.clone().recipient());
    let event_rx = Arc::new(event_tx.subscribe());
    // Pas cmd and event channels to NetEventTranslator
    NetEventTranslator::setup(&bus, &cmd_tx, &event_rx, "my-topic");

    // Capture messages from output on msgs vec
    let msgs: Arc<Mutex<Vec<GossipData>>> = Arc::new(Mutex::new(Vec::new()));

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
                msgs_loop.lock().await.push(msg.clone());
                event_tx.send(NetEvent::GossipData(msg))?;
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
        threshold_m: 3,
        threshold_n: 3,
        ..CiphernodeSelected::default()
    };

    bus.dispatch(evt_1.clone());
    bus.dispatch(evt_2.clone());
    bus.dispatch(local_evt_3.clone()); // This is a local event which should not be broadcast to the network

    // check the history of the event bus
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(3))
        .await?;

    assert_eq!(
        *msgs.lock().await,
        vec![
            GossipData::GossipBytes(bus.create_local(evt_1.clone()).to_bytes()?),
            GossipData::GossipBytes(bus.create_local(evt_2.clone()).to_bytes()?)
        ], // notice no local events
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
async fn test_duplicate_e3_id_with_different_chain_id() -> Result<()> {
    // Setup
    let (bus, rng, seed, params, crpoly, _, _) = get_common_setup(None)?;
    let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);

    // Setup actual ciphernodes and dispatch add events
    let ciphernodes = create_local_ciphernodes(&bus, &rng, 3, &cipher).await?;
    let eth_addrs = ciphernodes.iter().map(|tup| tup.address()).collect();

    setup_score_sortition_environment(&bus, &eth_addrs, 1).await?;
    setup_score_sortition_environment(&bus, &eth_addrs, 2).await?;

    // Send the computation requested event
    bus.dispatch(E3Requested {
        e3_id: E3id::new("1234", 1),
        threshold_m: 3,
        threshold_n: 3,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        ..E3Requested::default()
    });

    bus.dispatch(CommitteeFinalized {
        e3_id: E3id::new("1234", 1),
        committee: eth_addrs.clone(),
        chain_id: 1,
    });

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;

    let history_collector = ciphernodes.last().unwrap().history().unwrap();
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(28))
        .await?;

    assert_eq!(
        history.last().cloned().unwrap().into_data(),
        PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            e3_id: E3id::new("1234", 1),
            nodes: OrderedSet::from(eth_addrs.clone()),
        }
        .into()
    );

    // Send the computation requested event
    bus.dispatch(E3Requested {
        e3_id: E3id::new("1234", 2),
        threshold_m: 3,
        threshold_n: 3,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        ..E3Requested::default()
    });

    bus.dispatch(CommitteeFinalized {
        e3_id: E3id::new("1234", 2),
        committee: eth_addrs.clone(),
        chain_id: 2,
    });

    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;

    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(8))
        .await?;

    assert_eq!(
        history.last().cloned().unwrap().into_data(),
        PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            e3_id: E3id::new("1234", 2),
            nodes: OrderedSet::from(eth_addrs.clone()),
        }
        .into()
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_bus() -> Result<()> {
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    // Setup elements in test
    let (cmd_tx, _) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, event_rx) = broadcast::channel(100); // Receive byte events from the network
    let bus: BusHandle<EnclaveEvent> =
        EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true })
            .start()
            .into();
    let history_collector = HistoryCollector::<EnclaveEvent>::new().start();
    bus.subscribe("*", history_collector.clone().recipient());

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
        bus.create_local(event.clone()).to_bytes()?,
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
