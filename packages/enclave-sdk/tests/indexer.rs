use std::{sync::Arc, time::Duration};

use alloy::{
    node_bindings::Anvil,
    primitives::{Bytes, Uint},
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use enclave_sdk::indexer::{models::E3, DataStore, EnclaveIndexer, InMemoryStore};
use eyre::Result;
use tokio::{sync::RwLock, time::sleep};

sol!(
    #[sol(rpc)]
    Enclave,
    "tests/fixtures/fake_enclave.json"
);

#[tokio::test]
async fn test_indexer() -> Result<()> {
    let anvil = Anvil::new().block_time(1).try_spawn()?;

    let provider = ProviderBuilder::new()
        .on_ws(WsConnect::new(anvil.ws_endpoint()))
        .await?;

    let contract = Enclave::deploy(provider).await?;

    println!("Contract addr: {}", contract.address());
    let address = contract.address().to_string();
    let endpoint = anvil.ws_endpoint();
    let store = Arc::new(RwLock::new(InMemoryStore::new()));
    // TODO: Builder might be appropriate here.
    let mut indexer = EnclaveIndexer::new(&endpoint, &address, store.clone()).await?;
    indexer.initialize().await?;

    tokio::spawn(async move {
        if let Err(er) = indexer.start().await {
            eprintln!("Error running indexer: {}", er);
        }
    });

    let e3_id = 10;
    let expiration = 10;
    let public_key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

    contract
        .emitE3Activated(
            Uint::from(e3_id),
            Uint::from(expiration),
            Bytes::from(public_key),
        )
        .send()
        .await?
        .watch()
        .await?;

    // sleep(Duration::from_millis(100)).await;

    // let Some(e3) = store
    //     .read()
    //     .await
    //     .get::<E3>(&format!("e3:{}", e3_id))
    //     .await?
    // else {
    //     panic!("Could not get e3");
    // };
    //
    // assert_eq!(e3.ciphertext_inputs.len(), 0);

    let data = "Random data that wont actually be a string".to_string();
    let data_hash = 1234;
    let index = 1;

    contract
        .emitInputPublished(
            Uint::from(e3_id),
            Bytes::from(data),
            Uint::from(data_hash),
            Uint::from(index),
        )
        .send()
        .await?
        .watch()
        .await?;

    // let Some(e3) = store
    //     .read()
    //     .await
    //     .get::<E3>(&format!("e3:{}", e3_id))
    //     .await?
    // else {
    //     panic!("Could not get e3");
    // };
    //
    // assert_eq!(e3.ciphertext_inputs.len(), 1);

    Ok(())
}
