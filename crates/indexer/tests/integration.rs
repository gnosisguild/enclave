// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod helpers;
use alloy::{
    primitives::{Bytes, Uint},
    sol,
};
use e3_evm_helpers::contracts::ReadOnly;
use e3_indexer::{DataStore, EnclaveIndexer, InMemoryStore};
use eyre::Result;
use helpers::setup_two_contracts;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::sleep;
use EmitLogs::PublishMessage;
use Enclave::InputPublished;

sol!(
    #[sol(rpc)]
    Enclave,
    "tests/fixtures/fake_enclave.json"
);

sol!(
    #[sol(rpc)]
    EmitLogs,
    "tests/fixtures/emit_logs.json"
);

#[tokio::test]
// #[ignore]
async fn test_indexer() -> Result<()> {
    const E3_ID: u64 = 10;
    const THRESHOLD: u64 = 10;
    const INDEXER_DELAY_MS: u64 = 10;

    let (
        enclave_contract,
        enclave_address,
        emit_logs_contract,
        emit_logs_address,
        endpoint,
        _anvil,
    ) = setup_two_contracts().await?;

    let indexer = EnclaveIndexer::<InMemoryStore, ReadOnly>::from_endpoint_address_in_mem(
        &endpoint.to_string(),
        &[&enclave_address.to_string(), &emit_logs_address.to_string()],
    )
    .await?;

    // Track InputPublished event count in store
    indexer
        .add_event_handler(move |_: InputPublished, ctx| async move {
            let mut store = ctx.store();
            store
                .modify("input_count", |counter: Option<u64>| {
                    Some(counter.map_or(1, |c| c + 1))
                })
                .await?;
            Ok(())
        })
        .await;

    // Collect PublishMessage events
    let captured_messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let captured_messages_for_handler = captured_messages.clone();

    indexer
        .add_event_handler(move |msg: PublishMessage, _ctx| {
            // Collect message
            let messages = captured_messages_for_handler.clone();
            async move {
                messages.lock().unwrap().push(msg.value);
                Ok(())
            }
        })
        .await;

    let _ = indexer.start();

    let public_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let input_data = "Random data that wont actually be a string".to_string();
    let input_data_bytes = Bytes::from(input_data.clone().into_bytes());
    let ciphertext_output_data = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];

    enclave_contract
        .emitE3Activated(
            Uint::from(E3_ID),
            Uint::from(THRESHOLD),
            Bytes::from(public_key.clone()),
        )
        .send()
        .await?
        .watch()
        .await?;

    enclave_contract
        .emitInputPublished(
            Uint::from(E3_ID),
            input_data_bytes.clone(),
            Uint::from(1111),
            Uint::from(1),
        )
        .send()
        .await?
        .watch()
        .await?;

    // Sending message from logs contract which indexer is listening to
    emit_logs_contract
        .emitPublishMessage("Hello from contract2!".into())
        .send()
        .await?
        .watch()
        .await?;

    enclave_contract
        .emitInputPublished(
            Uint::from(E3_ID),
            input_data_bytes.clone(),
            Uint::from(2222),
            Uint::from(2),
        )
        .send()
        .await?
        .watch()
        .await?;

    enclave_contract
        .emitInputPublished(
            Uint::from(E3_ID),
            input_data_bytes.clone(),
            Uint::from(3333),
            Uint::from(3),
        )
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(INDEXER_DELAY_MS)).await;

    let messages_from_second_contract = captured_messages.lock().unwrap();
    assert_eq!(
        messages_from_second_contract
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["Hello from contract2!".to_string()]
    );
    drop(messages_from_second_contract);

    let e3_state = indexer.get_e3(E3_ID).await?;
    let expected_input_count = 3;

    assert_eq!(
        e3_state.ciphertext_inputs.len(),
        expected_input_count as usize
    );

    let expected_inputs = vec![
        (input_data_bytes.to_vec(), 1),
        (input_data_bytes.to_vec(), 2),
        (input_data_bytes.to_vec(), 3),
    ];
    assert_eq!(e3_state.ciphertext_inputs, expected_inputs);

    enclave_contract
        .emitCiphertextOutputPublished(
            Uint::from(E3_ID),
            Bytes::from(ciphertext_output_data.clone()),
        )
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(INDEXER_DELAY_MS)).await;

    let e3_state_after_output = indexer.get_e3(E3_ID).await?;
    assert_eq!(
        e3_state_after_output.ciphertext_output,
        ciphertext_output_data
    );

    let store = indexer.get_store();
    let total_inputs_processed = store.get::<u64>("input_count").await?.unwrap();
    assert_eq!(total_inputs_processed, expected_input_count);

    Ok(())
}

mod memory_leak {

    use e3_evm_helpers::{contracts::EnclaveContractFactory, listener::EventListener};

    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Track how many instances exist
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);
    static CREATE_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Clone)]
    struct LeakDetector(Arc<DropCounter>);

    #[derive(Debug)]
    struct DropCounter;

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl LeakDetector {
        fn new() -> Self {
            CREATE_COUNT.fetch_add(1, Ordering::SeqCst);
            Self(Arc::new(DropCounter))
        }
    }

    async fn create_indexer() -> Result<EnclaveIndexer<InMemoryStore, ReadOnly>> {
        let (_, enclave_address, _, _, endpoint, _anvil) = setup_two_contracts().await?;

        // Create indexer
        let listener =
            EventListener::create_contract_listener(&endpoint, &[&enclave_address]).await?;
        let contract = EnclaveContractFactory::create_read(&endpoint, &enclave_address).await?;

        EnclaveIndexer::<InMemoryStore, ReadOnly>::new_with_in_mem_store(listener, contract).await
    }

    #[tokio::test]
    #[ignore]
    async fn test_memory_leak() -> Result<()> {
        sol! {
            #[derive(Debug)]
            event TestEvent();

        }

        DROP_COUNT.store(0, Ordering::SeqCst);
        CREATE_COUNT.store(0, Ordering::SeqCst);

        {
            // Add an event handler that captures context
            let indexer = create_indexer().await?;
            let detector = LeakDetector::new();

            indexer
                .add_event_handler(move |event: TestEvent, _ctx| {
                    let _captured = detector.clone();
                    println!("{:?}", _captured.0);
                    async move {
                        // This closure captures ctx, which contains a listener clone
                        println!("Event received: {:?}", event);
                        Ok(())
                    }
                })
                .await;
        }

        // Delay to ensure everything is dropped.
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let created = CREATE_COUNT.load(Ordering::SeqCst);
        let dropped = DROP_COUNT.load(Ordering::SeqCst);

        println!("Created: {}, Dropped: {}", created, dropped);

        // This assertion will FAIL if there's a leak
        assert_eq!(
            created, dropped,
            "Memory leak detected! Created {} objects but only dropped {}",
            created, dropped
        );

        Ok(())
    }
}
