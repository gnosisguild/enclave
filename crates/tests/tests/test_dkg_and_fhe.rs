// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, SyncArbiter};
use anyhow::Result;
use e3_crypto::Cipher;
use e3_data::RepositoriesFactory;
use e3_data::{DataStore, InMemStore};
use e3_events::{E3Requested, E3id, EnclaveEvent, EventBus, EventBusConfig};
use e3_fhe::create_crp;
use e3_keyshare::ext::ThresholdKeyshareExtension;
use e3_multithread::Multithread;
use e3_request::E3Router;
use e3_sdk::bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_sortition::SortitionRepositoryFactory;
use e3_sortition::{CiphernodeSelector, Sortition};
use e3_test_helpers::ciphernode_builder::CiphernodeBuilder;
use e3_test_helpers::ciphernode_system::{CiphernodeSimulated, CiphernodeSystemBuilder};
use e3_test_helpers::{
    create_seed_from_u64, create_shared_rng_from_u64, rand_eth_addr, AddToCommittee,
};
use e3_trbfv::SharedRng;
use fhe::{
    bfv,
    trbfv::{SmudgingBoundCalculator, SmudgingBoundCalculatorConfig},
};
use num_bigint::BigUint;
use std::thread;
use std::time::Duration;
use std::{fs, sync::Arc};

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

pub fn calculate_error_size(
    params: Arc<bfv::BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
) -> Result<BigUint> {
    let config = SmudgingBoundCalculatorConfig::new(params, n, num_ciphertexts);
    let calculator = SmudgingBoundCalculator::new(config);
    Ok(calculator.calculate_sm_bound()?)
}

/// Test trbfv
#[actix::test]
async fn test_trbfv() -> Result<()> {
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
    let total_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let threads_to_use = std::cmp::max(1, total_threads.saturating_sub(1));
    println!("Total threads available: {}", total_threads);
    println!("Using {} threads for rayon pool", threads_to_use);
    let multithread = Multithread::attach(rng.clone(), cipher.clone(), threads_to_use);

    let nodes = CiphernodeSystemBuilder::new()
        // Adding 7 total nodes of which we are only choosing 5 for the committee
        .add_group(7, || async {
            CiphernodeBuilder::new(rng.clone(), cipher.clone())
                .with_address(&rand_eth_addr(&rng))
                .with_injected_multithread(multithread.clone())
                .with_history()
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

    ///////////////////////////////////////////////////////////////////////////////////
    // 2. Trigger E3Requested
    //
    //   - m=2.
    //   - n=5
    //   - error_size -> calculate using calculate_error_size
    //   - esi_per_ciphertext = 3
    ///////////////////////////////////////////////////////////////////////////////////

    // Prepare round

    let seed = create_seed_from_u64(123);

    // let crp = create_crp(params_raw.clone(), create_rng_from_seed(seed));

    // Calculate Error Size for E3Program (this will be done by the E3Program implementor)
    let error_size = Arc::new(BigUint::to_bytes_be(&calculate_error_size(
        params_raw, 5, 3,
    )?));

    let e3_requested = E3Requested {
        e3_id: E3id::new("0", 1),
        threshold_m: 1,
        threshold_n: 3,
        // threshold_m: 2,
        // threshold_n: 5, // Committee size is 5 from 7 total nodes
        seed: seed.clone(),
        error_size,
        esi_per_ct: 3,
        params,
    };

    let event = EnclaveEvent::from(e3_requested);

    bus.do_send(event);
    let expected = vec![
        "E3Requested",
        "CiphernodeSelected",
        "ThresholdShareCreated",
        "ThresholdShareCreated",
        "ThresholdShareCreated",
        // "ThresholdShareCreated",
        // "ThresholdShareCreated",
        "KeyshareCreated",
        "KeyshareCreated",
        "KeyshareCreated",
        // "KeyshareCreated",
        // "KeyshareCreated",
    ];

    // node #1 is selected so lets grab all events
    let h = nodes
        .take_history_with_timeout(1, expected.len(), Duration::from_secs(300))
        .await?;

    println!("{:?}", h);

    assert_eq!(h.event_types(), expected);

    Ok(())
}
