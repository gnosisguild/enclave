// helpers.rs
use alloy::{
    network::Ethereum,
    node_bindings::{Anvil, AnvilInstance},
    providers::{Provider, ProviderBuilder, WsConnect},
    signers::local::PrivateKeySigner,
    sol,
};
use eyre::Result;
use EmitLogs::EmitLogsInstance;
use Enclave::EnclaveInstance;

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

pub async fn setup_fake_enclave() -> Result<(
    EnclaveInstance<impl Provider>,
    String,
    String,
    AnvilInstance,
)> {
    let (provider, endpoint, anvil) = setup_provider().await?;
    let contract = Enclave::deploy(provider).await?;
    let address = contract.address().to_string();
    Ok((contract, address, endpoint, anvil))
}

pub async fn setup_provider() -> Result<(impl Provider, String, AnvilInstance)> {
    // Set anvil with fast blocktimes for testing
    let anvil = Anvil::new().block_time_f64(0.01).try_spawn()?;

    let provider = ProviderBuilder::new()
        .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
        .connect_ws(WsConnect::new(anvil.ws_endpoint()))
        .await?;

    let endpoint = anvil.ws_endpoint();
    Ok((provider, endpoint, anvil))
}
