use alloy::{
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, ProviderBuilder, RootProvider,
    },
    sol,
    transports::BoxTransport,
};
use alloy_network::{Ethereum, EthereumWallet};
use anvil::{spawn, NodeConfig};
use tokio;

sol!(
    #[sol(rpc)]
    EmitLogs,
    "tests/fixtures/emit_logs.json"
);

type StaticProvider = FillProvider<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<BoxTransport>,
        BoxTransport,
        Ethereum,
    >;

pub async fn http_provider_with_signer(
    http_endpoint: &str,
    signer: EthereumWallet,
) -> eyre::Result<
    StaticProvider
> {
    Ok(ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(signer)
        .on_builtin(http_endpoint)
        .await?)
}

#[tokio::test]
async fn test_contract_events() -> eyre::Result<()> {
    // Start local Anvil node
    let (_api, handle) = spawn(NodeConfig::default()).await;

    // Get the test wallet and convert to EthereumWallet
    let wallet: EthereumWallet = handle.dev_wallets().next().unwrap().into();
    // let signer: EthereumWallet = wallet.into();

    // Create provider with signer
    let provider: StaticProvider  = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(&format!("{}", &handle.http_endpoint()))
        .await?;

    // Deploy the contract
    let _contract = EmitLogs::deploy(&provider).await?;

    // Create a move closure that captures the contract by value
    // contract.setValue("helo".to_string()).await;

    Ok(())
}
