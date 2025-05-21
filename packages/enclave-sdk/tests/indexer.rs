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
    let address = contract.address().to_string();
    let endpoint = anvil.ws_endpoint();

    let mut indexer = EnclaveIndexer::new(&endpoint, &address, InMemoryStore::new()).await?;

    indexer.initialize().await?;

    indexer.start()?;

    // E3Activated
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

    sleep(Duration::from_millis(1)).await;

    // InputPublished
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

    sleep(Duration::from_millis(1)).await;

    let e3 = indexer.get_e3(e3_id).await?;

    assert_eq!(e3.ciphertext_inputs.len(), 1);

    Ok(())
}
