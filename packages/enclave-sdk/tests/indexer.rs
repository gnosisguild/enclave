use std::time::Duration;

use alloy::{
    node_bindings::Anvil,
    primitives::{Bytes, Uint},
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use enclave_sdk::indexer::{EnclaveIndexer, InMemoryStore};
use eyre::Result;
use tokio::time::sleep;

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

    let indexer = EnclaveIndexer::new(&endpoint, &address, InMemoryStore::new()).await?;

    // Start tracking state
    indexer.start()?;

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
            Bytes::from(data.clone()),
            Uint::from(1111),
            Uint::from(1),
        )
        .send()
        .await?
        .watch()
        .await?;

    contract
        .emitInputPublished(
            Uint::from(e3_id),
            Bytes::from(data.clone()),
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
            Bytes::from(data.clone()),
            Uint::from(3333),
            Uint::from(3),
        )
        .send()
        .await?
        .watch()
        .await?;

    sleep(Duration::from_millis(10)).await;

    assert_eq!(indexer.get_e3(e3_id).await?.ciphertext_inputs.len(), 3);
    assert_eq!(
        indexer.get_e3(e3_id).await?.ciphertext_inputs,
        vec![
            (Bytes::from(data.clone()).to_vec(), 1),
            (Bytes::from(data.clone()).to_vec(), 2),
            (Bytes::from(data.clone()).to_vec(), 3),
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

    Ok(())
}
