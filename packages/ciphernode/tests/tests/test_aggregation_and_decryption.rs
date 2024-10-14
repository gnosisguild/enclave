use data::{DataStore, InMemDataStore};
use enclave_core::{
    CiphernodeAdded, CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated,
    E3RequestComplete, E3Requested, E3id, EnclaveEvent, EventBus, GetHistory, KeyshareCreated,
    OrderedSet, PlaintextAggregated, PublicKeyAggregated, ResetHistory, Seed,
};
use fhe::{setup_crp_params, ParamsWithCrp, SharedRng};
use logger::SimpleLogger;
use p2p::P2p;
use router::{
    CiphernodeSelector, E3RequestRouter, LazyFhe, LazyKeyshare, LazyPlaintextAggregator,
    LazyPublicKeyAggregator,
};
use sortition::Sortition;

use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::*;
use fhe_rs::{
    bfv::{BfvParameters, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{FheEncoder, FheEncrypter, Serialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::{sync::mpsc::channel, time::sleep};

// Simulating a local node
async fn setup_local_ciphernode(bus: Addr<EventBus>, rng: SharedRng, logging: bool, addr: &str) {
    // create data actor for saving data
    let data_actor = InMemDataStore::new(logging).start(); // TODO: Use a sled backed Data Actor
    let store = DataStore::from_in_mem(data_actor);

    // create ciphernode actor for managing ciphernode flow
    let sortition = Sortition::attach(bus.clone());
    CiphernodeSelector::attach(bus.clone(), sortition.clone(), addr);

    E3RequestRouter::builder(bus.clone(), store)
        .add_hook(LazyFhe::create(rng))
        .add_hook(LazyPublicKeyAggregator::create(
            bus.clone(),
            sortition.clone(),
        ))
        .add_hook(LazyPlaintextAggregator::create(
            bus.clone(),
            sortition.clone(),
        ))
        .add_hook(LazyKeyshare::create(bus.clone(), addr))
        .build();

    SimpleLogger::attach(addr, bus.clone());
}

fn generate_pk_share(
    params: Arc<BfvParameters>,
    crp: CommonRandomPoly,
    rng: SharedRng,
) -> Result<(PublicKeyShare, SecretKey)> {
    let sk = SecretKey::random(&params, &mut *rng.lock().unwrap());
    let pk = PublicKeyShare::new(&sk, crp.clone(), &mut *rng.lock().unwrap())?;
    Ok((pk, sk))
}

#[actix::test]
async fn test_public_key_aggregation_and_decryption() -> Result<()> {
    // Setup EventBus
    let bus = EventBus::new(true).start();
    let rng = Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(42)));
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    let eth_addrs: Vec<String> = (0..3)
        .map(|_| Address::from_slice(&rand::thread_rng().gen::<[u8; 20]>()).to_string())
        .collect();

    setup_local_ciphernode(bus.clone(), rng.clone(), true, &eth_addrs[0]).await;
    setup_local_ciphernode(bus.clone(), rng.clone(), true, &eth_addrs[1]).await;
    setup_local_ciphernode(bus.clone(), rng.clone(), true, &eth_addrs[2]).await;

    let e3_id = E3id::new("1234");

    let ParamsWithCrp {
        crp_bytes, params, ..
    } = setup_crp_params(
        &[0x3FFFFFFF000001],
        2048,
        1032193,
        Arc::new(std::sync::Mutex::new(ChaCha20Rng::from_seed(
            seed.clone().into(),
        ))),
    );

    let regevt_1 = EnclaveEvent::from(CiphernodeAdded {
        address: eth_addrs[0].clone(),
        index: 0,
        num_nodes: 1,
    });

    bus.send(regevt_1.clone()).await?;

    let regevt_2 = EnclaveEvent::from(CiphernodeAdded {
        address: eth_addrs[1].clone(),
        index: 1,
        num_nodes: 2,
    });

    bus.send(regevt_2.clone()).await?;

    let regevt_3 = EnclaveEvent::from(CiphernodeAdded {
        address: eth_addrs[2].clone(),
        index: 2,
        num_nodes: 3,
    });

    bus.send(regevt_3.clone()).await?;

    let event = EnclaveEvent::from(E3Requested {
        e3_id: e3_id.clone(),
        threshold_m: 3,
        seed: seed.clone(),
        params: params.to_bytes(),
        src_chain_id: 1,
    });
    // Send the computation requested event
    bus.send(event.clone()).await?;

    // Test that we cannot send the same event twice
    bus.send(event).await?;

    let history = bus.send(GetHistory).await?;

    let rng_test = Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(42)));

    let crpoly = CommonRandomPoly::deserialize(&crp_bytes.clone(), &params)?;

    let (p1, sk1) = generate_pk_share(params.clone(), crpoly.clone(), rng_test.clone())?;
    let (p2, sk2) = generate_pk_share(params.clone(), crpoly.clone(), rng_test.clone())?;
    let (p3, sk3) = generate_pk_share(params.clone(), crpoly.clone(), rng_test.clone())?;

    let pubkey: PublicKey = vec![p1.clone(), p2.clone(), p3.clone()]
        .into_iter()
        .aggregate()?;

    println!("&&&& {}", history[8].event_type());
    assert_eq!(history.len(), 9);
    assert_eq!(
        history,
        vec![
            regevt_1,
            regevt_2,
            regevt_3,
            EnclaveEvent::from(E3Requested {
                e3_id: e3_id.clone(),
                threshold_m: 3,
                seed: seed.clone(),
                params: params.to_bytes(),
                src_chain_id: 1
            }),
            EnclaveEvent::from(CiphernodeSelected {
                e3_id: e3_id.clone(),
                threshold_m: 3,
            }),
            EnclaveEvent::from(KeyshareCreated {
                pubkey: p1.to_bytes(),
                e3_id: e3_id.clone(),
                node: eth_addrs[0].clone()
            }),
            EnclaveEvent::from(KeyshareCreated {
                pubkey: p2.to_bytes(),
                e3_id: e3_id.clone(),
                node: eth_addrs[1].clone()
            }),
            EnclaveEvent::from(KeyshareCreated {
                pubkey: p3.to_bytes(),
                e3_id: e3_id.clone(),
                node: eth_addrs[2].clone()
            }),
            EnclaveEvent::from(PublicKeyAggregated {
                pubkey: pubkey.to_bytes(),
                e3_id: e3_id.clone(),
                nodes: OrderedSet::from(eth_addrs.clone()),
                src_chain_id: 1
            }),
        ]
    );

    // Aggregate decryption
    bus.send(ResetHistory).await?;
    fn pad_end(input: &[u64], pad: u64, total: usize) -> Vec<u64> {
        let len = input.len();
        let mut cop = input.to_vec();
        cop.extend(std::iter::repeat(pad).take(total - len));
        cop
    }
    // TODO:
    // Making these values large (especially the yes value) requires changing
    // the params we use here - as we tune the FHE we need to take care
    let yes = 1234u64;
    let no = 873827u64;

    let raw_plaintext = vec![yes, no];
    let padded = &pad_end(&raw_plaintext, 0, 2048);
    let expected_raw_plaintext = bincode::serialize(&padded)?;
    let pt = Plaintext::try_encode(&raw_plaintext, Encoding::poly(), &params)?;

    let ciphertext = pubkey.try_encrypt(&pt, &mut ChaCha20Rng::seed_from_u64(42))?;

    let event = EnclaveEvent::from(CiphertextOutputPublished {
        ciphertext_output: ciphertext.to_bytes(),
        e3_id: e3_id.clone(),
    });

    let arc_ct = Arc::new(ciphertext);

    let ds1 = DecryptionShare::new(&sk1, &arc_ct, &mut *rng_test.lock().unwrap())?.to_bytes();
    let ds2 = DecryptionShare::new(&sk2, &arc_ct, &mut *rng_test.lock().unwrap())?.to_bytes();
    let ds3 = DecryptionShare::new(&sk3, &arc_ct, &mut *rng_test.lock().unwrap())?.to_bytes();

    // let ds1 = sk1
    bus.send(event.clone()).await?;

    sleep(Duration::from_millis(1)).await; // need to push to next tick
    let history = bus.send(GetHistory).await?;

    assert_eq!(history.len(), 6);

    assert_eq!(
        history,
        vec![
            event.clone(),
            EnclaveEvent::from(DecryptionshareCreated {
                decryption_share: ds1.clone(),
                e3_id: e3_id.clone(),
                node: eth_addrs[0].clone()
            }),
            EnclaveEvent::from(DecryptionshareCreated {
                decryption_share: ds2.clone(),
                e3_id: e3_id.clone(),
                node: eth_addrs[1].clone()
            }),
            EnclaveEvent::from(DecryptionshareCreated {
                decryption_share: ds3.clone(),
                e3_id: e3_id.clone(),
                node: eth_addrs[2].clone()
            }),
            EnclaveEvent::from(PlaintextAggregated {
                e3_id: e3_id.clone(),
                decrypted_output: expected_raw_plaintext.clone(),
                src_chain_id: 1
            }),
            EnclaveEvent::from(E3RequestComplete {
                e3_id: e3_id.clone()
            })
        ]
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_network() -> Result<()> {
    // Setup elements in test
    let (tx, mut output) = channel(100); // Transmit byte events to the network
    let (input, rx) = channel(100); // Receive byte events from the network
    let bus = EventBus::new(true).start();
    P2p::spawn_and_listen(bus.clone(), tx.clone(), rx);

    // Capture messages from output on msgs vec
    let msgs: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
    let msgs_loop = msgs.clone();

    tokio::spawn(async move {
        while let Some(msg) = output.recv().await {
            msgs_loop.lock().await.push(msg.clone());
            let _ = input.send(msg).await;
            // loopback to simulate a rebroadcast message
            // if this  manages to broadcast an event to the
            // event bus we will expect to see an extra event on
            // the bus
        }
    });

    let evt_1 = EnclaveEvent::from(PlaintextAggregated {
        e3_id: E3id::new("1235"),
        decrypted_output: vec![1, 2, 3, 4],
        src_chain_id: 1,
    });

    let evt_2 = EnclaveEvent::from(PlaintextAggregated {
        e3_id: E3id::new("1236"),
        decrypted_output: vec![1, 2, 3, 4],
        src_chain_id: 1,
    });

    let local_evt_3 = EnclaveEvent::from(CiphernodeSelected {
        e3_id: E3id::new("1235"),
        threshold_m: 3,
    });

    bus.do_send(evt_1.clone());
    bus.do_send(evt_2.clone());
    bus.do_send(local_evt_3.clone()); // This is a local event which should not be broadcast to the network

    sleep(Duration::from_millis(1)).await; // need to push to next tick

    // check the history of the event bus
    let history = bus.send(GetHistory).await?;

    assert_eq!(
        *msgs.lock().await,
        vec![evt_1.to_bytes()?, evt_2.to_bytes()?], // notice no local events
        "P2p did not transmit correct events to the network"
    );

    assert_eq!(
        history,
        vec![evt_1, evt_2, local_evt_3], // all local events that have been broadcast but no
        // events from the loopback
        "P2p must not retransmit forwarded event to event bus"
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_bus() -> Result<()> {
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    // Setup elements in test
    let (tx, _) = channel(100); // Transmit byte events to the network
    let (input, rx) = channel(100); // Receive byte events from the network
    let bus = EventBus::new(true).start();
    P2p::spawn_and_listen(bus.clone(), tx.clone(), rx);

    // Capture messages from output on msgs vec
    let event = EnclaveEvent::from(E3Requested {
        e3_id: E3id::new("1235"),
        threshold_m: 3,
        seed: seed.clone(),
        params: vec![1, 2, 3, 4],
        src_chain_id: 1,
    });

    // lets send an event from the network
    let _ = input.send(event.to_bytes()?).await;

    sleep(Duration::from_millis(1)).await; // need to push to next tick

    // check the history of the event bus
    let history = bus.send(GetHistory).await?;

    assert_eq!(history, vec![event]);

    Ok(())
}
