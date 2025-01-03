use aggregator::ext::{PlaintextAggregatorExtension, PublicKeyAggregatorExtension};
use crypto::Cipher;
use data::RepositoriesFactory;
use data::{DataStore, InMemStore};
use e3_request::E3Router;
use events::{
    CiphernodeAdded, CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated,
    E3RequestComplete, E3Requested, E3id, EnclaveEvent, EventBus, EventBusConfig, GetErrors,
    GetHistory, KeyshareCreated, OrderedSet, PlaintextAggregated, PublicKeyAggregated,
    ResetHistory, Seed, Shutdown,
};
use fhe::ext::FheExtension;
use fhe::{setup_crp_params, ParamsWithCrp, SharedRng};
use keyshare::ext::KeyshareExtension;
use logger::SimpleLogger;
use net::{events::NetworkPeerEvent, NetworkManager};
use sortition::SortitionRepositoryFactory;
use sortition::{CiphernodeSelector, Sortition};

use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::*;
use fhe_rs::{
    bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{FheEncoder, FheEncrypter, Serialize};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, Mutex};
use tokio::{sync::mpsc, time::sleep};

// Simulating a local node
type LocalCiphernodeTuple = (
    String, // Address
    Addr<InMemStore>,
    Addr<Sortition>,
    Addr<E3Router>,
    Addr<SimpleLogger<EnclaveEvent>>,
);

async fn setup_local_ciphernode(
    bus: &Addr<EventBus<EnclaveEvent>>,
    rng: &SharedRng,
    logging: bool,
    addr: &str,
    data: Option<Addr<InMemStore>>,
    cipher: &Arc<Cipher>,
) -> Result<LocalCiphernodeTuple> {
    // create data actor for saving data
    let data_actor = data.unwrap_or_else(|| InMemStore::new(logging).start());
    let store = DataStore::from(&data_actor);
    let repositories = store.repositories();
    // create ciphernode actor for managing ciphernode flow
    let sortition = Sortition::attach(&bus, repositories.sortition()).await?;
    CiphernodeSelector::attach(&bus, &sortition, addr);

    let router = E3Router::builder(&bus, store)
        .with(FheExtension::create(&bus, &rng))
        .with(PublicKeyAggregatorExtension::create(&bus, &sortition))
        .with(PlaintextAggregatorExtension::create(&bus, &sortition))
        .with(KeyshareExtension::create(&bus, addr, &cipher))
        .build()
        .await?;

    let logger = SimpleLogger::<EnclaveEvent>::attach(addr, bus.clone());
    Ok((addr.to_owned(), data_actor, sortition, router, logger))
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

fn create_random_eth_addrs(how_many: u32) -> Vec<String> {
    (0..how_many)
        .map(|_| Address::from_slice(&rand::thread_rng().gen::<[u8; 20]>()).to_string())
        .collect()
}

fn create_shared_rng_from_u64(value: u64) -> Arc<std::sync::Mutex<ChaCha20Rng>> {
    Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(value)))
}

fn create_seed_from_u64(value: u64) -> Seed {
    Seed(ChaCha20Rng::seed_from_u64(value).get_seed())
}

fn create_crp_bytes_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
    seed: &Seed,
) -> (Vec<u8>, Arc<BfvParameters>) {
    let ParamsWithCrp {
        crp_bytes, params, ..
    } = setup_crp_params(
        moduli,
        degree,
        plaintext_modulus,
        Arc::new(std::sync::Mutex::new(ChaCha20Rng::from_seed(
            seed.clone().into(),
        ))),
    );
    (crp_bytes, params)
}

/// Test helper to add addresses to the committee by creating events on the event bus
struct AddToCommittee {
    bus: Addr<EventBus<EnclaveEvent>>,
    count: usize,
}

impl AddToCommittee {
    fn new(bus: &Addr<EventBus<EnclaveEvent>>) -> Self {
        Self {
            bus: bus.clone(),
            count: 0,
        }
    }
    async fn add(&mut self, address: &str) -> Result<EnclaveEvent> {
        let evt = EnclaveEvent::from(CiphernodeAdded {
            address: address.to_owned(),
            index: self.count,
            num_nodes: self.count + 1,
        });

        self.count += 1;

        self.bus.send(evt.clone()).await?;

        Ok(evt)
    }
}

async fn create_local_ciphernodes(
    bus: &Addr<EventBus<EnclaveEvent>>,
    rng: &SharedRng,
    count: u32,
    cipher: &Arc<Cipher>,
) -> Result<Vec<LocalCiphernodeTuple>> {
    let eth_addrs = create_random_eth_addrs(count);
    let mut result = vec![];
    for addr in &eth_addrs {
        let tuple = setup_local_ciphernode(&bus, &rng, true, addr, None, cipher).await?;
        result.push(tuple);
    }

    Ok(result)
}

fn encrypt_ciphertext(
    params: &Arc<BfvParameters>,
    pubkey: PublicKey,
    raw_plaintext: Vec<u64>,
) -> Result<(Arc<Ciphertext>, Vec<u8>)> {
    let padded = &pad_end(&raw_plaintext, 0, 2048);
    let expected = bincode::serialize(&padded)?;
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
) -> Result<Vec<EnclaveEvent>> {
    let mut committee = AddToCommittee::new(&bus);
    let mut evts: Vec<EnclaveEvent> = vec![];

    for addr in addrs {
        evts.push(committee.add(addr).await?);
    }
    Ok(evts)
}

// Type for our tests to test against
type PkSkShareTuple = (PublicKeyShare, SecretKey, String);
type DecryptionShareTuple = (Vec<u8>, String);

fn aggregate_public_key(shares: &Vec<PkSkShareTuple>) -> Result<PublicKey> {
    Ok(shares
        .clone()
        .into_iter()
        .map(|(pk, _, _)| pk)
        .aggregate()?)
}

fn to_decryption_shares(
    shares: &Vec<PkSkShareTuple>,
    ciphertext: &Arc<Ciphertext>,
    rng: &SharedRng,
) -> Result<Vec<DecryptionShareTuple>> {
    let mut results = vec![];
    for (_, sk, addr) in shares {
        results.push((
            DecryptionShare::new(&sk, &ciphertext, &mut *rng.lock().unwrap())?.to_bytes(),
            addr.to_owned(),
        ));
    }

    Ok(results)
}

/// Helper to create keyshare events from eth addresses and generated shares
fn to_keyshare_events(shares: &Vec<PkSkShareTuple>, e3_id: &E3id) -> Vec<EnclaveEvent> {
    let mut result = Vec::new();
    for i in 0..shares.len() {
        result.push(EnclaveEvent::from(KeyshareCreated {
            pubkey: shares[i].0.to_bytes(),
            e3_id: e3_id.clone(),
            node: shares[i].2.clone(),
        }));
    }
    result
}

fn to_decryptionshare_events(
    decryption_shares: &Vec<DecryptionShareTuple>,
    e3_id: &E3id,
) -> Vec<EnclaveEvent> {
    let mut result = Vec::new();
    for i in 0..decryption_shares.len() {
        result.push(EnclaveEvent::from(DecryptionshareCreated {
            decryption_share: decryption_shares[i].0.clone(),
            e3_id: e3_id.clone(),
            node: decryption_shares[i].1.clone(),
        }));
    }
    result
}

fn get_common_setup() -> Result<(
    Addr<EventBus<EnclaveEvent>>,
    SharedRng,
    Seed,
    Arc<BfvParameters>,
    CommonRandomPoly,
    E3id,
)> {
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: true,
    })
    .start();
    let rng = create_shared_rng_from_u64(42);
    let seed = create_seed_from_u64(123);
    let (crp_bytes, params) = create_crp_bytes_params(&[0x3FFFFFFF000001], 2048, 1032193, &seed);
    let crpoly = CommonRandomPoly::deserialize(&crp_bytes.clone(), &params)?;
    let e3_id = E3id::new("1234");

    Ok((bus, rng, seed, params, crpoly, e3_id))
}

#[actix::test]
async fn test_public_key_aggregation_and_decryption() -> Result<()> {
    // Setup
    let (bus, rng, seed, params, crpoly, e3_id) = get_common_setup()?;
    let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);

    // Setup actual ciphernodes and dispatch add events
    let ciphernode_addrs = create_local_ciphernodes(&bus, &rng, 3, &cipher).await?;
    let eth_addrs = ciphernode_addrs
        .iter()
        .map(|tup| tup.0.to_owned())
        .collect();
    let add_events = add_ciphernodes(&bus, &eth_addrs).await?;
    let e3_request_event = EnclaveEvent::from(E3Requested {
        e3_id: e3_id.clone(),
        threshold_m: 3,
        seed: seed.clone(),
        params: params.to_bytes(),
        src_chain_id: 1,
    });

    // Send the computation requested event
    bus.send(e3_request_event.clone()).await?;

    // Test that we cannot send the same event twice
    bus.send(e3_request_event.clone()).await?;

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_shares = generate_pk_shares(&params, &crpoly, &rng_test, &eth_addrs)?;
    let test_pubkey = aggregate_public_key(&test_shares)?;

    // Assemble the expected history
    // Rust doesn't have a spread operator so this is a little awkward
    let mut expected_history = vec![];
    expected_history.extend(add_events); // start with add events
    expected_history.extend(vec![
        // The e3 request
        e3_request_event,
        // Ciphernode is selected
        EnclaveEvent::from(CiphernodeSelected {
            e3_id: e3_id.clone(),
            threshold_m: 3,
        }),
    ]);
    // Keyshare events
    expected_history.extend(to_keyshare_events(&test_shares, &e3_id));
    expected_history.extend(vec![
        // Our key has been aggregated
        EnclaveEvent::from(PublicKeyAggregated {
            pubkey: test_pubkey.to_bytes(),
            e3_id: e3_id.clone(),
            nodes: OrderedSet::from(eth_addrs.clone()),
            src_chain_id: 1,
        }),
    ]);

    let history = bus.send(GetHistory::<EnclaveEvent>::new()).await?;
    assert_eq!(history.len(), 9);
    assert_eq!(history, expected_history);
    bus.send(ResetHistory).await?;

    // Aggregate decryption

    // TODO:
    // Making these values large (especially the yes value) requires changing
    // the params we use here - as we tune the FHE we need to take care
    let raw_plaintext = vec![1234u64, 873827u64];
    let (ciphertext, expected) = encrypt_ciphertext(&params, test_pubkey, raw_plaintext)?;
    let decryption_events = to_decryptionshare_events(
        &to_decryption_shares(&test_shares, &ciphertext, &rng_test)?,
        &e3_id,
    );

    // Setup Ciphertext Published Event
    let ciphertext_published_event = EnclaveEvent::from(CiphertextOutputPublished {
        ciphertext_output: ciphertext.to_bytes(),
        e3_id: e3_id.clone(),
    });

    bus.send(ciphertext_published_event.clone()).await?;

    sleep(Duration::from_millis(1)).await; // need to push to next tick

    // Assemble the expected history
    // Rust doesn't have a spread operator so this is a little awkward
    let mut expected_history = vec![];
    expected_history.extend(vec![ciphertext_published_event.clone()]);
    expected_history.extend(decryption_events);
    expected_history.extend(vec![
        EnclaveEvent::from(PlaintextAggregated {
            e3_id: e3_id.clone(),
            decrypted_output: expected.clone(),
            src_chain_id: 1,
        }),
        EnclaveEvent::from(E3RequestComplete {
            e3_id: e3_id.clone(),
        }),
    ]);

    let history = bus.send(GetHistory::<EnclaveEvent>::new()).await?;
    assert_eq!(history.len(), 6);
    assert_eq!(history, expected_history);

    Ok(())
}

#[actix::test]
async fn test_stopped_keyshares_retain_state() -> Result<()> {
    let (bus, rng, seed, params, crpoly, e3_id) = get_common_setup()?;
    let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);

    let eth_addrs = create_random_eth_addrs(2);

    let cn1 = setup_local_ciphernode(&bus, &rng, true, &eth_addrs[0], None, &cipher).await?;
    let cn2 = setup_local_ciphernode(&bus, &rng, true, &eth_addrs[1], None, &cipher).await?;
    add_ciphernodes(&bus, &eth_addrs).await?;

    // Send e3request
    bus.send(
        EnclaveEvent::from(E3Requested {
            e3_id: e3_id.clone(),
            threshold_m: 2,
            seed: seed.clone(),
            params: params.to_bytes(),
            src_chain_id: 1,
        })
        .clone(),
    )
    .await?;

    let history = bus.send(GetHistory::<EnclaveEvent>::new()).await?;
    let errors = bus.send(GetErrors::<EnclaveEvent>::new()).await?;

    println!("{:?}", errors);

    assert_eq!(errors.len(), 0);

    // SEND SHUTDOWN!
    bus.send(EnclaveEvent::from(Shutdown)).await?;

    // Reset history
    bus.send(ResetHistory).await?;

    // Check event count is correct
    assert_eq!(history.len(), 7);

    // Get the address and the data actor from the two ciphernodes
    // and rehydrate them to new actors
    let (addr1, data1, ..) = cn1;
    let (addr2, data2, ..) = cn2;

    // Apply the address and data node to two new actors
    // Here we test that hydration occurred sucessfully
    setup_local_ciphernode(&bus, &rng, true, &addr1, Some(data1), &cipher).await?;
    setup_local_ciphernode(&bus, &rng, true, &addr2, Some(data2), &cipher).await?;
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

    let history = bus.send(GetHistory::<EnclaveEvent>::new()).await?;

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
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: true,
    })
    .start();
    let net_bus = EventBus::<NetworkPeerEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: false,
    })
    .start();
    // Pas cmd and event channels to NetworkManager
    NetworkManager::setup(bus.clone(), net_bus.clone(), cmd_tx.clone(), "my-topic");

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
                net::events::NetworkPeerCommand::GossipPublish { data, .. } => Some(data),
                _ => None,
            } {
                msgs_loop.lock().await.push(msg.clone());
                net_bus.do_send(NetworkPeerEvent::GossipData(msg));
            }
            // if this  manages to broadcast an event to the
            // event bus we will expect to see an extra event on
            // the bus but we don't because we handle this
        }
        anyhow::Ok(())
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
    let history = bus.send(GetHistory::<EnclaveEvent>::new()).await?;

    assert_eq!(
        *msgs.lock().await,
        vec![evt_1.to_bytes()?, evt_2.to_bytes()?], // notice no local events
        "NetworkManager did not transmit correct events to the network"
    );

    assert_eq!(
        history,
        vec![evt_1, evt_2, local_evt_3], // all local events that have been broadcast but no
        // events from the loopback
        "NetworkManager must not retransmit forwarded event to event bus"
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_bus() -> Result<()> {
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    // Setup elements in test
    let (cmd_tx, _) = mpsc::channel(100); // Transmit byte events to the network
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: true,
    })
    .start();
    let net_bus = EventBus::<NetworkPeerEvent>::new(EventBusConfig {
        capture_history: true,
        deduplicate: false,
    })
    .start();
    NetworkManager::setup(bus.clone(), net_bus.clone(), cmd_tx.clone(), "mytopic");

    // Capture messages from output on msgs vec
    let event = EnclaveEvent::from(E3Requested {
        e3_id: E3id::new("1235"),
        threshold_m: 3,
        seed: seed.clone(),
        params: vec![1, 2, 3, 4],
        src_chain_id: 1,
    });

    // lets send an event from the network
    net_bus.do_send(NetworkPeerEvent::GossipData(event.to_bytes()?));

    sleep(Duration::from_millis(1)).await; // need to push to next tick

    // check the history of the event bus
    let history = bus.send(GetHistory::<EnclaveEvent>::new()).await?;

    assert_eq!(history, vec![event]);

    Ok(())
}
