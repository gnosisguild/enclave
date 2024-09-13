#![crate_name = "enclave_core"]
#![crate_type = "lib"]
// #![warn(missing_docs, unused_imports)]

mod ciphernode;
mod ciphernode_selector;
mod data;
mod enclave_contract;
mod eventbus;
mod events;
mod fhe;
mod logger;
mod ordered_set;
mod p2p;
mod plaintext_aggregator;
mod publickey_aggregator;
mod registry;
mod serializers;
mod sortition;

// TODO: this is too permissive
pub use actix::prelude::*;
pub use ciphernode::*;
pub use ciphernode_selector::*;
pub use data::*;
pub use eventbus::*;
pub use events::*;
pub use fhe::*;
pub use logger::*;
pub use p2p::*;
pub use plaintext_aggregator::*;
pub use publickey_aggregator::*;
pub use registry::*;
pub use sortition::*;

// TODO: move these out to a test folder
#[cfg(test)]
mod tests {
    use crate::{
        ciphernode_selector::CiphernodeSelector,
        data::Data,
        eventbus::{EventBus, GetHistory},
        events::{CommitteeRequested, E3id, EnclaveEvent, KeyshareCreated, PublicKeyAggregated},
        p2p::P2p,
        serializers::{
            CiphertextSerializer, DecryptionShareSerializer, PublicKeySerializer,
            PublicKeyShareSerializer,
        },
        CiphernodeAdded, CiphernodeSelected, CiphertextOutputPublished, DecryptionshareCreated,
        PlaintextAggregated, Registry, ResetHistory, SharedRng, Sortition,
    };
    use actix::prelude::*;
    use alloy_primitives::Address;
    use anyhow::*;
    use fhe::{
        bfv::{BfvParameters, BfvParametersBuilder, Encoding, Plaintext, PublicKey, SecretKey},
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
    async fn setup_local_ciphernode(
        bus: Addr<EventBus>,
        rng: SharedRng,
        logging: bool,
        addr: Address,
    ) {
        // create data actor for saving data
        let data = Data::new(logging).start(); // TODO: Use a sled backed Data Actor

        // create ciphernode actor for managing ciphernode flow
        let sortition = Sortition::attach(bus.clone());
        CiphernodeSelector::attach(bus.clone(), sortition.clone(), addr);
        Registry::attach(bus.clone(), data.clone(), sortition, rng, addr).await;
    }

    fn setup_bfv_params(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
    ) -> Arc<BfvParameters> {
        BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(moduli)
            .build_arc()
            .unwrap()
    }

    fn set_up_crp(params: Arc<BfvParameters>, rng: SharedRng) -> CommonRandomPoly {
        CommonRandomPoly::new(&params, &mut *rng.lock().unwrap()).unwrap()
    }

    fn generate_pk_share(
        params: Arc<BfvParameters>,
        crp: CommonRandomPoly,
        rng: SharedRng,
    ) -> Result<(Vec<u8>, SecretKey)> {
        let sk = SecretKey::random(&params, &mut *rng.lock().unwrap());
        let pk = PublicKeyShareSerializer::to_bytes(
            PublicKeyShare::new(&sk, crp.clone(), &mut *rng.lock().unwrap())?,
            params.clone(),
            crp,
        )?;
        Ok((pk, sk))
    }

    struct NewParamsWithCrp {
        pub moduli: Vec<u64>,
        pub degree: usize,
        pub plaintext_modulus: u64,
        pub crp_bytes: Vec<u8>,
        pub params: Arc<BfvParameters>,
    }
    fn setup_crp_params(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
        rng: SharedRng,
    ) -> NewParamsWithCrp {
        let params = setup_bfv_params(moduli, degree, plaintext_modulus);
        let crp = set_up_crp(params.clone(), rng);
        NewParamsWithCrp {
            moduli: moduli.to_vec(),
            degree,
            plaintext_modulus,
            crp_bytes: crp.to_bytes(),
            params,
        }
    }

    #[actix::test]
    async fn test_public_key_aggregation_and_decryption() -> Result<()> {
        // Setup EventBus
        let bus = EventBus::new(true).start();

        let rng = Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(42)));

        let eth_addrs: Vec<Address> = (0..3)
            .map(|_| Address::from_slice(&rand::thread_rng().gen::<[u8; 20]>()))
            .collect();

        setup_local_ciphernode(bus.clone(), rng.clone(), true, eth_addrs[0]).await;
        setup_local_ciphernode(bus.clone(), rng.clone(), true, eth_addrs[1]).await;
        setup_local_ciphernode(bus.clone(), rng.clone(), true, eth_addrs[2]).await;

        let e3_id = E3id::new("1234");

        let NewParamsWithCrp {
            moduli,
            degree,
            plaintext_modulus,
            crp_bytes,
            params,
        } = setup_crp_params(
            &[0x3FFFFFFF000001],
            2048,
            1032193,
            Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(42))),
        );

        let regevt_1 = EnclaveEvent::from(CiphernodeAdded {
            address: eth_addrs[0],
            index: 0,
            num_nodes: 1,
        });

        bus.send(regevt_1.clone()).await?;

        let regevt_2 = EnclaveEvent::from(CiphernodeAdded {
            address: eth_addrs[1],
            index: 1,
            num_nodes: 2,
        });

        bus.send(regevt_2.clone()).await?;

        let regevt_3 = EnclaveEvent::from(CiphernodeAdded {
            address: eth_addrs[2],
            index: 2,
            num_nodes: 3,
        });

        bus.send(regevt_3.clone()).await?;

        let event = EnclaveEvent::from(CommitteeRequested {
            e3_id: e3_id.clone(),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
            moduli: moduli.clone(),
            degree,
            plaintext_modulus,
            crp: crp_bytes.clone(),
        });

        // Send the computation requested event
        bus.send(event.clone()).await?;

        // Test that we cannot send the same event twice
        bus.send(event).await?;

        let history = bus.send(GetHistory).await?;

        let rng_test = Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(42)));

        let crpoly = CommonRandomPoly::deserialize(&crp_bytes.clone(), &params)?;

        // Passing rng through function chain to ensure it matches usage in system above
        let (p1, sk1) = generate_pk_share(params.clone(), crpoly.clone(), rng_test.clone())?;
        let (p2, sk2) = generate_pk_share(params.clone(), crpoly.clone(), rng_test.clone())?;
        let (p3, sk3) = generate_pk_share(params.clone(), crpoly.clone(), rng_test.clone())?;

        let pubkey: PublicKey = [p1.clone(), p2.clone(), p3.clone()]
            .iter()
            .map(|k| PublicKeyShareSerializer::from_bytes(k).unwrap())
            .aggregate()?;

        assert_eq!(history.len(), 9);
        assert_eq!(
            history,
            vec![
                regevt_1,
                regevt_2,
                regevt_3,
                EnclaveEvent::from(CommitteeRequested {
                    e3_id: e3_id.clone(),
                    nodecount: 3,
                    threshold: 123,
                    sortition_seed: 123,
                    moduli,
                    degree,
                    plaintext_modulus,
                    crp: crp_bytes,
                }),
                EnclaveEvent::from(CiphernodeSelected {
                    e3_id: e3_id.clone(),
                    nodecount: 3,
                    threshold: 123,
                }),
                EnclaveEvent::from(KeyshareCreated {
                    pubkey: p1.clone(),
                    e3_id: e3_id.clone(),
                    node: eth_addrs[0]
                }),
                EnclaveEvent::from(KeyshareCreated {
                    pubkey: p2.clone(),
                    e3_id: e3_id.clone(),
                    node: eth_addrs[1]
                }),
                EnclaveEvent::from(KeyshareCreated {
                    pubkey: p3.clone(),
                    e3_id: e3_id.clone(),
                    node: eth_addrs[2]
                }),
                EnclaveEvent::from(PublicKeyAggregated {
                    pubkey: PublicKeySerializer::to_bytes(pubkey.clone(), params.clone())?,
                    e3_id: e3_id.clone()
                })
            ]
        );

        // Aggregate decryption
        bus.send(ResetHistory).await?;

        // TODO:
        // Making these values large (especially the yes value) requires changing
        // the params we use here - as we tune the FHE we need to take care
        let yes = 1234u64;
        let no = 873827u64;

        let raw_plaintext = vec![yes, no];
        let expected_raw_plaintext = bincode::serialize(&raw_plaintext)?;
        let pt = Plaintext::try_encode(&raw_plaintext, Encoding::poly(), &params)?;

        let ciphertext = pubkey.try_encrypt(&pt, &mut ChaCha20Rng::seed_from_u64(42))?;

        let event = EnclaveEvent::from(CiphertextOutputPublished {
            ciphertext_output: CiphertextSerializer::to_bytes(ciphertext.clone(), params.clone())?,
            e3_id: e3_id.clone(),
        });

        let arc_ct = Arc::new(ciphertext);

        let ds1 = DecryptionShareSerializer::to_bytes(
            DecryptionShare::new(&sk1, &arc_ct, &mut *rng_test.lock().unwrap()).unwrap(),
            params.clone(),
            arc_ct.clone(),
        )?;
        let ds2 = DecryptionShareSerializer::to_bytes(
            DecryptionShare::new(&sk2, &arc_ct, &mut *rng_test.lock().unwrap()).unwrap(),
            params.clone(),
            arc_ct.clone(),
        )?;
        let ds3 = DecryptionShareSerializer::to_bytes(
            DecryptionShare::new(&sk3, &arc_ct, &mut *rng_test.lock().unwrap()).unwrap(),
            params.clone(),
            arc_ct.clone(),
        )?;

        // let ds1 = sk1
        bus.send(event.clone()).await?;

        sleep(Duration::from_millis(1)).await; // need to push to next tick
        let history = bus.send(GetHistory).await?;

        assert_eq!(history.len(), 5);
        assert_eq!(
            history,
            vec![
                event.clone(),
                EnclaveEvent::from(DecryptionshareCreated {
                    decryption_share: ds1.clone(),
                    e3_id: e3_id.clone(),
                    node: eth_addrs[0]
                }),
                EnclaveEvent::from(DecryptionshareCreated {
                    decryption_share: ds2.clone(),
                    e3_id: e3_id.clone(),
                    node: eth_addrs[1]
                }),
                EnclaveEvent::from(DecryptionshareCreated {
                    decryption_share: ds3.clone(),
                    e3_id: e3_id.clone(),
                    node: eth_addrs[2]
                }),
                EnclaveEvent::from(PlaintextAggregated {
                    e3_id: e3_id.clone(),
                    decrypted_output: expected_raw_plaintext.clone()
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
                let _ = input.send(msg).await; // loopback to simulate a rebroadcast message
                                               // if this  manages to broadcast an event to the
                                               // event bus we will expect to see an extra event on
                                               // the bus
            }
        });

        let evt_1 = EnclaveEvent::from(CommitteeRequested {
            e3_id: E3id::new("1234"),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
            moduli: vec![0x3FFFFFFF000001],
            degree: 2048,
            plaintext_modulus: 1032193,
            crp: vec![1, 2, 3, 4],
        });

        let evt_2 = EnclaveEvent::from(CommitteeRequested {
            e3_id: E3id::new("1235"),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
            moduli: vec![0x3FFFFFFF000001],
            degree: 2048,
            plaintext_modulus: 1032193,
            crp: vec![1, 2, 3, 4],
        });

        let local_evt_3 = EnclaveEvent::from(CiphernodeSelected {
            e3_id: E3id::new("1235"),
            nodecount: 3,
            threshold: 123,
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
        // Setup elements in test
        let (tx, _) = channel(100); // Transmit byte events to the network
        let (input, rx) = channel(100); // Receive byte events from the network
        let bus = EventBus::new(true).start();
        P2p::spawn_and_listen(bus.clone(), tx.clone(), rx);

        // Capture messages from output on msgs vec
        let event = EnclaveEvent::from(CommitteeRequested {
            e3_id: E3id::new("1235"),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
            moduli: vec![0x3FFFFFFF000001],
            degree: 2048,
            plaintext_modulus: 1032193,
            crp: vec![1, 2, 3, 4],
        });

        // lets send an event from the network
        let _ = input.send(event.to_bytes()?).await;

        sleep(Duration::from_millis(1)).await; // need to push to next tick

        // check the history of the event bus
        let history = bus.send(GetHistory).await?;

        assert_eq!(history, vec![event]);

        Ok(())
    }
}
