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
async fn test_indexer() -> Result<()> {
    let (contract, address, contract2, address2, endpoint, _anvil) = setup_two_contracts().await?;
    let address = address.to_string();
    let endpoint = endpoint.to_string();

    let indexer = EnclaveIndexer::<InMemoryStore, ReadOnly>::from_endpoint_address_in_mem(
        &endpoint,
        &[&address, &address2],
    )
    .await?;

    let published: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

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

    let published_clone = published.clone();
    indexer
        .add_event_handler(move |msg: PublishMessage, _ctx| {
            let published_clone = published_clone.clone();
            async move {
                let mut guard = published_clone.lock().unwrap();
                guard.push(msg.value);
                Ok(())
            }
        })
        .await;

    // Start tracking state
    let _ = indexer.start();

    // E3Activated
    let e3_id = 10;

    let pubkey = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    contract
        .emitE3Activated(
            Uint::from(e3_id),
            Uint::from(10),
            Bytes::from(pubkey.clone()),
        )
        .send()
        .await?
        .watch()
        .await?;

    // InputPublished
    let data = "Random data that wont actually be a string".to_string();
    contract
        .emitInputPublished(
            Uint::from(e3_id),
            Bytes::from(data.clone().into_bytes()),
            Uint::from(1111),
            Uint::from(1),
        )
        .send()
        .await?
        .watch()
        .await?;

    // Here we check that we can emit events on a second contract and have the indexer still listen
    // for it
    contract2
        .emitPublishMessage("Hello from contract2!".into())
        .send()
        .await?
        .watch()
        .await?;

    contract
        .emitInputPublished(
            Uint::from(e3_id),
            Bytes::from(data.clone().into_bytes()),
            Uint::from(2222),
            Uint::from(2),
        )
        .send()
        .await?
        .watch()
        .await?;

    contract
        .emitInputPublished(
            Uint::from(e3_id),
            Bytes::from(data.clone().into_bytes()),
            Uint::from(3333),
            Uint::from(3),
        )
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(10)).await;
    let published_clone = published.clone();
    let published_guard = published_clone.lock().unwrap();
    assert_eq!(
        published_guard.iter().cloned().collect::<Vec<_>>(),
        vec!["Hello from contract2!".to_string()]
    );
    assert_eq!(indexer.get_e3(e3_id).await?.ciphertext_inputs.len(), 3);
    assert_eq!(
        indexer.get_e3(e3_id).await?.ciphertext_inputs,
        vec![
            (Bytes::from(data.clone().into_bytes()).to_vec(), 1),
            (Bytes::from(data.clone().into_bytes()).to_vec(), 2),
            (Bytes::from(data.clone().into_bytes()).to_vec(), 3),
        ]
    );

    let ciphertext_output = vec![9, 8, 7, 6, 5, 4, 3, 2, 1];
    contract
        .emitCiphertextOutputPublished(Uint::from(e3_id), Bytes::from(ciphertext_output.clone()))
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(10)).await;

    let e3 = indexer.get_e3(e3_id).await?;

    assert_eq!(e3.ciphertext_output, ciphertext_output);

    let store = indexer.get_store();
    let val = store.get::<u64>("input_count").await?.unwrap();
    assert_eq!(val, 3);
    Ok(())
}
