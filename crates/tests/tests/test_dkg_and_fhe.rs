// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr};
use anyhow::{bail, Result};
use async_std::task::sleep;
use e3_aggregator::ext::{PlaintextAggregatorExtension, PublicKeyAggregatorExtension};
use e3_crypto::Cipher;
use e3_data::RepositoriesFactory;
use e3_data::{DataStore, InMemStore};
use e3_events::{E3Requested, E3id, EnclaveEvent, EventBus, EventBusConfig, TakeHistory};
use e3_fhe::{create_crp, ext::FheExtension};
use e3_keyshare::ext::{KeyshareExtension, ThresholdKeyshareExtension};
use e3_multithread::Multithread;
use e3_request::E3Router;
use e3_sdk::bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_sortition::SortitionRepositoryFactory;
use e3_sortition::{CiphernodeSelector, Sortition};
use e3_test_helpers::ciphernode_system::{CiphernodeSimulated, CiphernodeSystemBuilder};
use e3_test_helpers::{
    create_random_eth_addrs, create_rng_from_seed, create_seed_from_u64,
    create_shared_rng_from_u64, rand_eth_addr, simulate_libp2p_net, AddToCommittee,
};
use e3_trbfv::SharedRng;
use fhe::{
    bfv,
    trbfv::{SmudgingBoundCalculator, SmudgingBoundCalculatorConfig},
};
use num_bigint::BigUint;
use std::time::Duration;
use std::{fs, sync::Arc};
use tokio::sync::RwLock;
// use zeroize::Zeroizing;

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

// fn serialize_z_vec_of_bytes(data: &Vec<Zeroizing<Vec<u8>>>) -> Vec<u8> {
//     bincode::serialize(
//         &data
//             .iter()
//             .map(|z| -> &Vec<u8> { z.as_ref() })
//             .collect::<Vec<_>>(),
//     )
//     .unwrap()
// }

pub fn calculate_error_size(
    params: Arc<bfv::BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
) -> Result<BigUint> {
    let config = SmudgingBoundCalculatorConfig::new(params, n, num_ciphertexts);
    let calculator = SmudgingBoundCalculator::new(config);
    Ok(calculator.calculate_sm_bound()?)
}

// Act like a single party in multithread
// #[derive(Clone)]
// struct PartySharesResult {
//     pk_share_and_sk_sss_event: EnclaveEvent,
//     esi_sss_event: EnclaveEvent,
// }
// async fn generate_party_shares(
//     rng: Arc<Mutex<ChaCha20Rng>>,
//     params: Arc<Vec<u8>>,
//     cipher: Arc<Cipher>,
//     crp: Arc<Vec<u8>>,
//     error_size: Arc<Vec<u8>>,
//     num_parties: u64,
//     threshold: u64,
// ) -> Result<PartySharesResult> {
//
//     // 1. Setup test environment
//     let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
//     //
//     //
//     //
//     //
//     // 1. ThresholdKeyshare receives CiphernodeSelected event
//     // 1. EventBus emits EncryptionPubkeyCreated
//     // 1. Send other parties' EncryptionPubkeyCreated events to everyone else
//     // 1. Wait for GlobalEncryptionKeyAggregated
//     // 1. EventBus emits KeyshareCreated
//     // 1. EventBus emits THresholdShareCreated
//     //
//
//     // Setup multithread processor
//     // TODO: Currently only testing logic not setup on multithread yet
//     let _multi = Multithread::attach(&bus, rng, cipher.clone());
//
//     /////////////////////////////////////////////
//     // 1. Generate initial pk and sk sss
//     /////////////////////////////////////////////
//
//     let event: EnclaveEvent = e3_trbfv::gen_pk_share_and_sk_sss::Request {
//         trbfv_config: TrBFVConfig::new(params.clone(), num_parties, threshold),
//         crp,
//     }
//     .into();
//
//     let correlation_id = event.correlation_id();
//
//     let pk_share_and_sk_sss_event = EventWaiter::send_and_wait(
//         &bus,
//         event,
//         Box::new(move |e| e.correlation_id().is_some() && e.correlation_id() == correlation_id),
//     )
//     .await?;
//
//     // // Now lets setup a waiter to wait for the response
//     // let wait_for_response = wait_for_event(
//     //     &bus,
//     //     Box::new(move |e| match e {
//     //         EnclaveEvent::ComputeRequestSucceeded { data, .. } => {
//     //             data.correlation_id == correlation_id
//     //         }
//     //         _ => false,
//     //     }),
//     // );
//
//     // Send the event
//     // bus.do_send(gen_pk_share_and_sk_sss.clone());
//
//     // let pk_share_and_sk_sss_event = wait_for_response.await??;
//
//     /////////////////////////////////////////////
//     // 2. Generate smudging noise
//     /////////////////////////////////////////////
//
//     let gen_esi_sss: EnclaveEvent = e3_trbfv::gen_esi_sss::Request {
//         trbfv_config: TrBFVConfig::new(params.clone(), num_parties, threshold),
//         error_size,
//         esi_per_ct: 1,
//     }
//     .into();
//
//     let correlation_id = gen_esi_sss.correlation_id().unwrap();
//
//     // Now lets setup a waiter to wait for the response
//     let wait_for_response = wait_for_event(
//         &bus,
//         Box::new(move |e| match e {
//             EnclaveEvent::ComputeRequestSucceeded { data, .. } => {
//                 data.correlation_id == correlation_id
//             }
//             _ => false,
//         }),
//     );
//
//     bus.do_send(gen_esi_sss.clone());
//
//     let esi_sss_event = wait_for_response.await??;
//     Ok(PartySharesResult {
//         pk_share_and_sk_sss_event,
//         esi_sss_event,
//     })
// }

// async fn snapshot_test_events(party: PartySharesResult, cipher: &Cipher) -> Result<()> {
//     let Some(TrBFVResponse::GenPkShareAndSkSss(res)) =
//         party.pk_share_and_sk_sss_event.trbfv_response()
//     else {
//         bail!("bad response from GenPkShareAndSkSss");
//     };
//
//     // Ensure pk_share is correct
//     let pk_share = res.pk_share.clone();
//
//     // NOTE: uncomment the following to save new snapshot.
//     // save_snapshot("fixtures/01_pk_share.bin", &pk_share[..]);
//
//     // Check against snapshot
//     assert_eq!(
//         pk_share,
//         Arc::new(include_bytes!("fixtures/01_pk_share.bin").to_vec())
//     );
//
//     // Ensure sk_sss is correct
//     let sk_sss = SensitiveBytes::access_vec(res.sk_sss.clone(), &cipher)?;
//
//     let serialized_sk_sss = serialize_z_vec_of_bytes(&sk_sss);
//
//     // NOTE: uncomment the following to save new snapshot.
//     // save_snapshot("fixtures/02_sk_sss.bin", &serialized_sk_sss);
//
//     // Check against snapshot
//     assert_eq!(
//         serialized_sk_sss,
//         include_bytes!("fixtures/02_sk_sss.bin").to_vec()
//     );
//
//     let Some(TrBFVResponse::GenEsiSss(res)) = party.esi_sss_event.trbfv_response() else {
//         bail!("bad response from GenEsiSss");
//     };
//
//     let esi_sss = SensitiveBytes::access_vec(res.esi_sss.clone(), &cipher)?;
//
//     let serialized_esi_sss = serialize_z_vec_of_bytes(&esi_sss);
//     // NOTE: uncomment the following to save new snapshot.
//     // save_snapshot("fixtures/03_esi_sss.bin", &serialized_esi_sss);
//
//     assert_eq!(
//         serialized_esi_sss,
//         include_bytes!("fixtures/03_esi_sss.bin").to_vec()
//     );
//
//     Ok(())
// }

/// Function to setup a specific ciphernode actor configuration
async fn setup_local_ciphernode(
    bus: Addr<EventBus<EnclaveEvent>>,
    rng: SharedRng,
    logging: bool,
    addr: String,
    data: Option<Addr<InMemStore>>,
    cipher: Arc<Cipher>,
) -> Result<CiphernodeSimulated> {
    // Local bus for ciphernode events
    let local_bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();

    // History collector for taking historical events for analysis
    let history = EventBus::<EnclaveEvent>::history(&local_bus);

    // Error collector for taking historical events for analysis
    let errors = EventBus::<EnclaveEvent>::error(&local_bus);

    // Pipe all source events to the local bus
    EventBus::pipe(&bus, &local_bus);

    // create data actor for saving data
    let data_actor = data.unwrap_or_else(|| InMemStore::new(logging).start());
    let store = DataStore::from(&data_actor);
    let repositories = store.repositories();

    // create ciphernode actor for managing ciphernode flow
    let sortition = Sortition::attach(&local_bus, repositories.sortition()).await?;

    // Multithread actor
    let multithread = Multithread::attach(&rng, &cipher);

    // Ciphernode Selector
    CiphernodeSelector::attach(&local_bus, &sortition, &addr);

    // E3 specific chain
    E3Router::builder(&local_bus, store)
        .with(ThresholdKeyshareExtension::create(
            &local_bus,
            &addr,
            &cipher,
            &multithread,
            &rng,
        ))
        .build()
        .await?;

    Ok(CiphernodeSimulated {
        store: data_actor.clone(),
        address: addr.to_owned(),
        bus: local_bus,
        history,
        errors,
    })
}

async fn create_ciphernods_system(
    bus: &Addr<EventBus<EnclaveEvent>>,
    rng: &SharedRng,
    count: u32,
    cipher: &Arc<Cipher>,
) -> Result<Vec<CiphernodeSimulated>> {
    let mut nodes = Vec::new();
    for addr in create_random_eth_addrs(count) {
        nodes.push(
            setup_local_ciphernode(bus.clone(), rng.clone(), false, addr, None, cipher.clone())
                .await?,
        );
    }
    simulate_libp2p_net(&nodes);
    Ok(nodes)
}

/// Test trbfv
#[actix::test]
async fn test_trbfv() -> Result<()> {
    // NOTE: Here we are trying to make it as clear as possible as to what is going on so attempting to
    // avoid over abstracting test helpers and favouring straight forward single descriptive
    // functions alongside explanations

    ////
    // 1. Setup ThresholdKeyshare system
    //
    //   - E3Router
    //   - ThresholdKeyshare
    //   - Multithread actor
    //   - 7 nodes (so as to check for some nodes not getting selected)
    //   - Loopback libp2p simulation
    ////

    // Create rng
    let rng = create_shared_rng_from_u64(42);

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

    // Common Random Polynomial for BFV
    let crp = create_crp(params_raw.clone(), rng.clone());

    // Encoded Params
    let params = Arc::new(encode_bfv_params(&params_raw));

    // Cipher
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let mut adder = AddToCommittee::new(&bus, 1);

    // Actor system setup
    let nodes = CiphernodeSystemBuilder::new()
        .add_group(7, || async {
            setup_local_ciphernode(
                bus.clone(),
                rng.clone(),
                true,
                rand_eth_addr(&rng),
                None,
                cipher.clone(),
            )
            .await
        })
        .simulate_libp2p()
        .build()
        .await?;

    for node in nodes.iter() {
        adder.add(&node.address).await?;
    }

    // Flush all events
    nodes.flush_all_history(100).await?;

    ////
    // 2. Trigger E3Requested
    //
    //   - m=2.
    //   - n=5
    //   - error_size -> calculate using calculate_error_size
    //   - esi_per_ciphertext = 3
    ////

    // Prepare round

    let seed = create_seed_from_u64(123);

    // let crp = create_crp(params_raw.clone(), create_rng_from_seed(seed));

    // Calculate Error Size for E3Program (this will be done by the E3Program implementor)
    let error_size = Arc::new(BigUint::to_bytes_be(&calculate_error_size(
        params_raw, 5, 3,
    )?));

    let e3_requested = E3Requested {
        e3_id: E3id::new("0", 1),
        threshold_m: 2,
        threshold_n: 5,
        seed: seed.clone(),
        error_size,
        esi_per_ct: 3,
        params,
    };

    let event = EnclaveEvent::from(e3_requested);

    bus.do_send(event);

    // node #1 is selected so lets grab all events
    let h = nodes.take_history(1, 4).await?;

    assert_eq!(h.event_types(), vec!["E3Requested", "CiphernodeSelected"]);

    Ok(())
}
