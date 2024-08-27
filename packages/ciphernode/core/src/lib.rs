#![crate_name = "enclave_core"]
#![crate_type = "lib"]
// #![warn(missing_docs, unused_imports)]

mod ciphernode;
mod committee;
mod committee_key;
mod data;
mod enclave_contract;
mod eventbus;
mod events;
mod fhe;
mod ordered_set;
mod p2p;

// pub struct Core {
//     pub name: String,
// }
//
// impl Core {
//     fn new(name: String) -> Self {
//         Self { name }
//     }
//
//     fn run() {
//         actix::run(async move {
//             sleep(Duration::from_millis(100)).await;
//             actix::System::current().stop();
//         });
//     }
// }

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::{
        ciphernode::Ciphernode,
        committee::CommitteeManager,
        data::{Data, GetLog},
        eventbus::{EventBus, GetHistory, Subscribe},
        events::{ComputationRequested, E3id, EnclaveEvent, KeyshareCreated, PublicKeyAggregated},
        fhe::{Fhe, WrappedPublicKey, WrappedPublicKeyShare},
        p2p::P2p,
    };
    use actix::prelude::*;
    use anyhow::*;
    use fhe::{
        bfv::{BfvParameters, BfvParametersBuilder, PublicKey, SecretKey},
        mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use tokio::sync::Mutex;
    use tokio::{sync::mpsc::channel, time::sleep};

    // Simulating a local node
    fn setup_local_ciphernode(
        bus: Addr<EventBus>,
        fhe: Addr<Fhe>,
        logging: bool,
    ) -> (Addr<Ciphernode>, Addr<Data>) {
        // create data actor for saving data
        let data = Data::new(logging).start(); // TODO: Use a sled backed Data Actor

        // create ciphernode actor for managing ciphernode flow
        let node = Ciphernode::new(bus.clone(), fhe.clone(), data.clone()).start();

        // subscribe for computation requested events from the event bus
        bus.do_send(Subscribe::new("ComputationRequested", node.clone().into()));

        // setup the committee manager to generate the comittee public keys
        setup_committee_manager(bus.clone(), fhe);
        (node, data)
    }

    fn setup_bfv_params(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
        mut rng: ChaCha20Rng,
    ) -> Result<(Arc<BfvParameters>, CommonRandomPoly)> {
        let params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(&moduli)
            .build_arc()?;
        let crp = CommonRandomPoly::new(&params, &mut rng)?;
        Ok((params, crp))
    }

    fn generate_pk_share(
        params: Arc<BfvParameters>,
        crp: CommonRandomPoly,
        mut rng: ChaCha20Rng,
    ) -> Result<(WrappedPublicKeyShare, ChaCha20Rng)> {
        let sk = SecretKey::random(&params, &mut rng);
        let pk = WrappedPublicKeyShare::from_fhe_rs(
            PublicKeyShare::new(&sk, crp.clone(), &mut rng)?,
            params.clone(),
            crp,
        );
        Ok((pk, rng))
    }

    fn setup_committee_manager(bus: Addr<EventBus>, fhe: Addr<Fhe>) -> Addr<CommitteeManager> {
        let committee = CommitteeManager::new(bus.clone(), fhe.clone()).start();

        bus.do_send(Subscribe::new(
            "ComputationRequested",
            committee.clone().into(),
        ));
        bus.do_send(Subscribe::new("KeyshareCreated", committee.clone().into()));

        committee
    }

    fn setup_global_fhe_actor(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
        rng1: ChaCha20Rng,
        rng2: ChaCha20Rng,
    ) -> Result<Addr<Fhe>> {
        let (params, crp) = setup_bfv_params(&moduli, degree, plaintext_modulus, rng1)?;
        Ok(Fhe::new(params, crp, rng2)?.start())
    }

    #[actix::test]
    async fn test_public_key_aggregation() -> Result<()> {
        // Setup EventBus
        let bus = EventBus::new(true).start();

        // Setup global FHE actor
        let fhe = setup_global_fhe_actor(
            &vec![0x3FFFFFFF000001],
            2048,
            1032193,
            ChaCha20Rng::seed_from_u64(42),
            ChaCha20Rng::seed_from_u64(42),
        )?;
        setup_local_ciphernode(bus.clone(), fhe.clone(), true);
        setup_local_ciphernode(bus.clone(), fhe.clone(), true);
        setup_local_ciphernode(bus.clone(), fhe.clone(), true);

        let e3_id = E3id::new("1234");

        let event = EnclaveEvent::from(ComputationRequested {
            e3_id: e3_id.clone(),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
        });

        // Send the computation requested event
        bus.send(event.clone()).await?;

        // Test that we cannot send the same event twice
        bus.send(event).await?;

        let history = bus.send(GetHistory).await?;

        let (params, crp) = setup_bfv_params(
            &vec![0x3FFFFFFF000001],
            2048,
            1032193,
            ChaCha20Rng::seed_from_u64(42),
        )?;

        // Passing rng through function chain to ensure it matches usage in system above
        let rng = ChaCha20Rng::seed_from_u64(42);
        let (p1, rng) = generate_pk_share(params.clone(), crp.clone(), rng)?;
        let (p2, rng) = generate_pk_share(params.clone(), crp.clone(), rng)?;
        let (p3, _) = generate_pk_share(params.clone(), crp.clone(), rng)?;

        let aggregated: PublicKey = vec![p1.clone(), p2.clone(), p3.clone()]
            .iter()
            .map(|k| k.clone_inner())
            .aggregate()?;

        assert_eq!(history.len(), 5);
        assert_eq!(
            history,
            vec![
                EnclaveEvent::from(ComputationRequested {
                    e3_id: e3_id.clone(),
                    nodecount: 3,
                    threshold: 123,
                    sortition_seed: 123,
                }),
                EnclaveEvent::from(KeyshareCreated {
                    pubkey: p1.clone(),
                    e3_id: e3_id.clone(),
                }),
                EnclaveEvent::from(KeyshareCreated {
                    pubkey: p2.clone(),
                    e3_id: e3_id.clone(),
                }),
                EnclaveEvent::from(KeyshareCreated {
                    pubkey: p3.clone(),
                    e3_id: e3_id.clone()
                }),
                EnclaveEvent::from(PublicKeyAggregated {
                    pubkey: WrappedPublicKey::from_fhe_rs(aggregated, params),
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
        let (_, rx) = channel(100); // Receive byte events from the network
        let bus = EventBus::new(true).start();
        P2p::spawn_and_listen(bus.clone(), tx.clone(), rx);

        // Capture messages from output on msgs vec
        let msgs: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        let msgs_loop = msgs.clone();

        tokio::spawn(async move {
            while let Some(msg) = output.recv().await {
                msgs_loop.lock().await.push(msg);
            }
        });

        let evt_1 = EnclaveEvent::from(ComputationRequested {
            e3_id: E3id::new("1234"),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
        });

        let evt_2 = EnclaveEvent::from(ComputationRequested {
            e3_id: E3id::new("1235"),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
        });

        bus.do_send(evt_1.clone());
        bus.do_send(evt_2.clone());

        sleep(Duration::from_millis(1)).await; // need to push to next tick

        assert_eq!(
            *msgs.lock().await,
            vec![evt_1.to_bytes()?, evt_2.to_bytes()?],
            "P2p did not transmit events to the network"
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
        let event = EnclaveEvent::from(ComputationRequested {
            e3_id: E3id::new("1235"),
            nodecount: 3,
            threshold: 123,
            sortition_seed: 123,
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
