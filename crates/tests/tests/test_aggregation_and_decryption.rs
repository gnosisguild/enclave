// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::*;
use e3_crypto::Cipher;
use e3_data::GetDump;
use e3_data::InMemStore;
use e3_events::{
    CiphernodeSelected, CiphertextOutputPublished, E3Requested, E3id, EnclaveEvent, EventBus,
    EventBusConfig, GetErrors, HistoryCollector, OrderedSet, PlaintextAggregated,
    PublicKeyAggregated, Seed, Shutdown, Subscribe, TakeHistory,
};
use e3_fhe::SharedRng;
use e3_net::{events::NetEvent, NetEventTranslator};
use e3_sdk::bfv_helpers::encode_bfv_params;
use e3_test_helpers::ciphernode_builder::CiphernodeBuilder;
use e3_test_helpers::ciphernode_system::CiphernodeSimulated;
use e3_test_helpers::{
    create_random_eth_addrs, create_shared_rng_from_u64, get_common_setup, simulate_libp2p_net,
    AddToCommittee,
};
use fhe::{
    bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
};
use fhe_traits::{FheEncoder, FheEncrypter, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;

async fn setup_local_ciphernode(
    bus: &Addr<EventBus<EnclaveEvent>>,
    rng: &SharedRng,
    logging: bool,
    addr: &str,
    data: Option<Addr<InMemStore>>,
    cipher: &Arc<Cipher>,
) -> Result<CiphernodeSimulated> {
    let mut builder = CiphernodeBuilder::new(rng.clone(), cipher.clone())
        .with_address(addr)
        .with_source_bus(bus)
        .with_history()
        .with_errors()
        .with_pubkey_aggregation()
        .with_plaintext_aggregation();

    if let Some(data) = data {
        builder = builder.with_data(data);
    }

    if logging {
        builder = builder.with_logging()
    }

    let node = builder.build().await?;

    Ok(node)
    // // create data actor for saving data
    // let local_bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
    // // Pipe all source events to the local bus
    // let history = EventBus::<EnclaveEvent>::history(&local_bus);
    // let errors = EventBus::<EnclaveEvent>::error(&local_bus);
    // EventBus::pipe(bus, &local_bus);
    // let data_actor = data.unwrap_or_else(|| InMemStore::new(logging).start());
    // let store = DataStore::from(&data_actor);
    // let repositories = store.repositories();
    //
    // // create ciphernode actor for managing ciphernode flow
    // let sortition = Sortition::attach(&local_bus, repositories.sortition()).await?;
    // CiphernodeSelector::attach(&local_bus, &sortition, addr);
    //
    // E3Router::builder(&local_bus, store)
    //     .with(FheExtension::create(&local_bus, &rng))
    //     .with(PublicKeyAggregatorExtension::create(&local_bus, &sortition))
    //     .with(PlaintextAggregatorExtension::create(&local_bus, &sortition))
    //     .with(KeyshareExtension::create(&local_bus, addr, &cipher))
    //     .build()
    //     .await?;
    //
    // SimpleLogger::<EnclaveEvent>::attach(addr, local_bus.clone());
    // // Ok((
    // //     addr.to_owned(),
    // //     data_actor.clone(),
    // //     sortition,
    // //     router,
    // //     logger,
    // //     local_bus,
    // //     history,
    // //     error,
    // // ))
    // Ok(CiphernodeSimulated {
    //     bus: local_bus,
    //     history,
    //     store: data_actor.clone(),
    //     address: addr.to_owned(),
    //     errors,
    // })
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
    bus: &Addr<EventBus<EnclaveEvent>>,
    rng: &SharedRng,
    count: u32,
    cipher: &Arc<Cipher>,
) -> Result<Vec<CiphernodeSimulated>> {
    let eth_addrs = create_random_eth_addrs(count);
    let mut result = vec![];
    for addr in &eth_addrs {
        println!("Setting up eth addr: {}", addr);
        let tuple = setup_local_ciphernode(&bus, &rng, true, addr, None, cipher).await?;
        result.push(tuple);
    }
    // let local_buses: Vec<_> = result.iter().map(|n| &n.5).collect();
    simulate_libp2p_net(&result);

    Ok(result)
}

fn encrypt_ciphertext(
    params: &Arc<BfvParameters>,
    pubkey: PublicKey,
    raw_plaintext: Vec<u64>,
) -> Result<(Arc<Ciphertext>, Vec<u8>)> {
    let padded = &pad_end(&raw_plaintext, 0, 2048);
    let mut bytes = Vec::with_capacity(padded.len() * 8);
    for value in padded {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let expected = bytes;
    let pt = Plaintext::try_encode(&raw_plaintext, Encoding::poly(), &params)?;
    let ciphertext = pubkey.try_encrypt(&pt, &mut ChaCha20Rng::seed_from_u64(42))?;
    Ok((Arc::new(ciphertext), expected))
}

fn pad_end(input: &[u64], pad: u64, total: usize) -> Vec<u64> {
    let len = input.len();
    let mut cop = input.to_vec();
    cop.extend(std::iter::repeat(pad).take(total - len));
    cop
}

async fn add_ciphernodes(
    bus: &Addr<EventBus<EnclaveEvent>>,
    addrs: &Vec<String>,
    chain_id: u64,
) -> Result<Vec<EnclaveEvent>> {
    let mut committee = AddToCommittee::new(&bus, chain_id);
    let mut evts: Vec<EnclaveEvent> = vec![];

    for addr in addrs {
        evts.push(committee.add(addr).await?);
    }
    Ok(evts)
}

// Type for our tests to test against
type PkSkShareTuple = (PublicKeyShare, SecretKey, String);
// type DecryptionShareTuple = (Vec<u8>, String);

fn aggregate_public_key(shares: &Vec<PkSkShareTuple>) -> Result<PublicKey> {
    Ok(shares
        .clone()
        .into_iter()
        .map(|(pk, _, _)| pk)
        .aggregate()?)
}

// fn to_decryption_shares(
//     shares: &Vec<PkSkShareTuple>,
//     ciphertext: &Arc<Ciphertext>,
//     rng: &SharedRng,
// ) -> Result<Vec<DecryptionShareTuple>> {
//     let mut results = vec![];
//     for (_, sk, addr) in shares {
//         results.push((
//             DecryptionShare::new(&sk, &ciphertext, &mut *rng.lock().unwrap())?.to_bytes(),
//             addr.to_owned(),
//         ));
//     }
//
//     Ok(results)
// }
//
// /// Helper to create keyshare events from eth addresses and generated shares
// fn to_keyshare_events(shares: &Vec<PkSkShareTuple>, e3_id: &E3id) -> Vec<EnclaveEvent> {
//     let mut result = Vec::new();
//     for i in 0..shares.len() {
//         result.push(EnclaveEvent::from(KeyshareCreated {
//             pubkey: shares[i].0.to_bytes(),
//             e3_id: e3_id.clone(),
//             node: shares[i].2.clone(),
//         }));
//     }
//     result
// }

// fn to_decryptionshare_events(
//     decryption_shares: &Vec<DecryptionShareTuple>,
//     e3_id: &E3id,
// ) -> Vec<EnclaveEvent> {
//     let mut result = Vec::new();
//     for i in 0..decryption_shares.len() {
//         result.push(EnclaveEvent::from(DecryptionshareCreated {
//             decryption_share: decryption_shares[i].0.clone(),
//             e3_id: e3_id.clone(),
//             node: decryption_shares[i].1.clone(),
//         }));
//     }
//     result
// }

#[actix::test]
async fn test_public_key_aggregation_and_decryption() -> Result<()> {
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

    add_ciphernodes(&bus, &eth_addrs, 1).await?;

    let e3_request_event = EnclaveEvent::from(E3Requested {
        e3_id: e3_id.clone(),
        params: Arc::new(encode_bfv_params(&params)),
        seed: seed.clone(),
        threshold_m: 3,
        ..E3Requested::default()
    });

    // Send the computation requested event
    bus.send(e3_request_event.clone()).await?;

    // Test that we cannot send the same event twice
    bus.send(e3_request_event.clone()).await?;

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_shares = generate_pk_shares(&params, &crpoly, &rng_test, &eth_addrs)?;
    let test_pubkey = aggregate_public_key(&test_shares)?;

    let expected_aggregated_event = PublicKeyAggregated {
        pubkey: test_pubkey.to_bytes(),
        e3_id: e3_id.clone(),
        nodes: OrderedSet::from(eth_addrs.clone()),
    };

    let history_collector = ciphernodes.first().unwrap().history().unwrap();

    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(9))
        .await?;

    let aggregated_event: Vec<_> = history
        .into_iter()
        .filter_map(|e| match e {
            EnclaveEvent::PublicKeyAggregated { data, .. } => Some(data),
            _ => None,
        })
        .collect();

    assert_eq!(aggregated_event, vec![expected_aggregated_event]);

    // Aggregate decryption

    // TODO:
    // Making these values large (especially the yes value) requires changing
    // the params we use here - as we tune the FHE we need to take care
    let raw_plaintext = vec![1234u64, 873827u64];
    let (ciphertext, expected) = encrypt_ciphertext(&params, test_pubkey, raw_plaintext)?;

    // Setup Ciphertext Published Event
    let ciphertext_published_event = EnclaveEvent::from(CiphertextOutputPublished {
        ciphertext_output: ciphertext.to_bytes(),
        e3_id: e3_id.clone(),
    });

    bus.send(ciphertext_published_event.clone()).await?;
    let expected_plaintext_agg_event = PlaintextAggregated {
        e3_id: e3_id.clone(),
        decrypted_output: expected.clone(),
    };

    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(6))
        .await?;

    let aggregated_event = history
        .into_iter()
        .filter_map(|e| match e {
            EnclaveEvent::PlaintextAggregated { data, .. } => Some(data),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(aggregated_event, vec![expected_plaintext_agg_event]);

    Ok(())
}

#[actix::test]
async fn test_stopped_keyshares_retain_state() -> Result<()> {
    let e3_id = E3id::new("1234", 1);
    let (rng, cn1_address, cn1_data, cn2_address, cn2_data, cipher, history, params, crpoly) = {
        let (bus, rng, seed, params, crpoly, ..) = get_common_setup(None)?;
        let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);
        let ciphernodes = create_local_ciphernodes(&bus, &rng, 2, &cipher).await?;
        let eth_addrs = ciphernodes.iter().map(|n| n.address()).collect::<Vec<_>>();

        add_ciphernodes(&bus, &eth_addrs, 1).await?;

        let [cn1, cn2] = &ciphernodes.as_slice() else {
            panic!("Not enough elements")
        };

        // Send e3request
        bus.send(
            EnclaveEvent::from(E3Requested {
                e3_id: e3_id.clone(),
                threshold_m: 2,
                seed: seed.clone(),
                params: Arc::new(encode_bfv_params(&params)),
                ..E3Requested::default()
            })
            .clone(),
        )
        .await?;
        let history_collector = cn1.history().unwrap();
        let error_collector = cn1.errors().unwrap();
        let history = history_collector
            .send(TakeHistory::<EnclaveEvent>::new(7))
            .await?;
        let errors = error_collector
            .send(GetErrors::<EnclaveEvent>::new())
            .await?;

        assert_eq!(errors.len(), 0);

        // SEND SHUTDOWN!
        bus.send(EnclaveEvent::from(Shutdown)).await?;

        // This is probably overkill but required to ensure that all the data is written
        sleep(Duration::from_secs(1)).await;

        let cn1_dump = cn1.store().send(GetDump).await??;
        let cn2_dump = cn2.store().send(GetDump).await??;

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
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
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
        .filter_map(|evt| match evt {
            EnclaveEvent::KeyshareCreated { data, .. } => {
                PublicKeyShare::deserialize(&data.pubkey, &params, crpoly.clone()).ok()
            }
            _ => None,
        })
        .aggregate()?;

    // Publish the ciphertext
    let raw_plaintext = vec![1234u64, 873827u64];
    let (ciphertext, expected) = encrypt_ciphertext(&params, pubkey, raw_plaintext)?;
    bus.send(
        EnclaveEvent::from(CiphertextOutputPublished {
            ciphertext_output: ciphertext.to_bytes(),
            e3_id: e3_id.clone(),
        })
        .clone(),
    )
    .await?;

    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(4))
        .await?;

    let actual = history.iter().find_map(|evt| match evt {
        EnclaveEvent::PlaintextAggregated { data, .. } => Some(data.decrypted_output.clone()),
        _ => None,
    });
    assert_eq!(actual, Some(expected));

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_network() -> Result<()> {
    // Setup elements in test
    let (cmd_tx, mut cmd_rx) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, _) = broadcast::channel(100); // Receive byte events from the network
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
    let history_collector = HistoryCollector::<EnclaveEvent>::new().start();
    bus.do_send(Subscribe::new("*", history_collector.clone().recipient()));
    let event_rx = event_tx.subscribe();
    // Pas cmd and event channels to NetEventTranslator
    NetEventTranslator::setup(bus.clone(), cmd_tx.clone(), event_rx, "my-topic");

    // Capture messages from output on msgs vec
    let msgs: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
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

    let evt_1 = EnclaveEvent::from(PlaintextAggregated {
        e3_id: E3id::new("1235", 1),
        decrypted_output: vec![1, 2, 3, 4],
    });

    let evt_2 = EnclaveEvent::from(PlaintextAggregated {
        e3_id: E3id::new("1236", 1),
        decrypted_output: vec![1, 2, 3, 4],
    });

    let local_evt_3 = EnclaveEvent::from(CiphernodeSelected {
        e3_id: E3id::new("1235", 1),
        threshold_m: 3,
        ..CiphernodeSelected::default()
    });

    bus.do_send(evt_1.clone());
    bus.do_send(evt_2.clone());
    bus.do_send(local_evt_3.clone()); // This is a local event which should not be broadcast to the network

    // check the history of the event bus
    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(3))
        .await?;

    assert_eq!(
        *msgs.lock().await,
        vec![evt_1.to_bytes()?, evt_2.to_bytes()?], // notice no local events
        "NetEventTranslator did not transmit correct events to the network"
    );

    assert_eq!(
        history,
        vec![evt_1, evt_2, local_evt_3], // all local events that have been broadcast but no
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
    add_ciphernodes(&bus, &eth_addrs, 1).await?;
    add_ciphernodes(&bus, &eth_addrs, 2).await?;

    // Send the computation requested event
    bus.send(EnclaveEvent::from(E3Requested {
        e3_id: E3id::new("1234", 1),
        threshold_m: 3,
        seed: seed.clone(),
        params: Arc::new(encode_bfv_params(&params)),
        ..E3Requested::default()
    }))
    .await?;

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;

    let history_collector = ciphernodes.last().unwrap().history().unwrap();
    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(12))
        .await?;

    assert_eq!(
        history.last().unwrap(),
        &EnclaveEvent::from(PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            e3_id: E3id::new("1234", 1),
            nodes: OrderedSet::from(eth_addrs.clone()),
        })
    );

    // Send the computation requested event
    bus.send(EnclaveEvent::from(E3Requested {
        e3_id: E3id::new("1234", 2),
        threshold_m: 3,
        seed: seed.clone(),
        params: Arc::new(encode_bfv_params(&params)),
        ..E3Requested::default()
    }))
    .await?;

    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;

    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(6))
        .await?;

    assert_eq!(
        history.last().unwrap(),
        &EnclaveEvent::from(PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            e3_id: E3id::new("1234", 2),
            nodes: OrderedSet::from(eth_addrs.clone()),
        })
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_bus() -> Result<()> {
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    // Setup elements in test
    let (cmd_tx, _) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, event_rx) = broadcast::channel(100); // Receive byte events from the network
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
    let history_collector = HistoryCollector::<EnclaveEvent>::new().start();
    bus.do_send(Subscribe::new("*", history_collector.clone().recipient()));

    NetEventTranslator::setup(bus.clone(), cmd_tx.clone(), event_rx, "mytopic");

    // Capture messages from output on msgs vec
    let event = EnclaveEvent::from(E3Requested {
        e3_id: E3id::new("1235", 1),
        threshold_m: 3,
        seed: seed.clone(),
        params: Arc::new(vec![1, 2, 3, 4]),
        ..E3Requested::default()
    });

    // lets send an event from the network
    let _ = event_tx.send(NetEvent::GossipData(event.to_bytes()?));

    // check the history of the event bus
    let history = history_collector
        .send(TakeHistory::<EnclaveEvent>::new(1))
        .await?;

    assert_eq!(history, vec![event]);

    Ok(())
}
