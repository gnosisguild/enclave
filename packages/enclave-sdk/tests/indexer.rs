use alloy::{
    node_bindings::Anvil,
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use enclave_sdk::indexer::{EnclaveIndexer, InMemoryStore};
use eyre::Result;

sol!(
    #[sol(rpc)]
    Enclave,
    "tests/fixtures/fake_enclave.json"
);

#[tokio::test]
async fn test_indexer() -> Result<()> {
    // let anvil = Anvil::new().block_time(0).try_spawn()?;
    //
    // let provider = ProviderBuilder::new()
    //     .on_ws(WsConnect::new(anvil.ws_endpoint()))
    //     .await?;
    //
    // let contract = Enclave::deploy(provider).await?;
    // let address = contract.address().to_string();
    // let store = InMemoryStore::new();
    // let endpoint = anvil.ws_endpoint();
    // let indexer = EnclaveIndexer::new(&endpoint, &address, store);

    Ok(())
}
