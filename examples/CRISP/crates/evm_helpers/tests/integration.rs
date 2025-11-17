// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::signers::local::PrivateKeySigner;
use evm_helpers::CRISPContractFactory;
use eyre::Result;

async fn setup_provider() -> Result<(impl Provider, String, AnvilInstance)> {
    let anvil = Anvil::new().block_time_f64(0.01).try_spawn()?;
    let provider = ProviderBuilder::new()
        .wallet(PrivateKeySigner::from_slice(&anvil.keys()[0].to_bytes())?)
        .connect_ws(WsConnect::new(anvil.ws_endpoint()))
        .await?;
    let endpoint = anvil.ws_endpoint().to_string();
    Ok((provider, endpoint, anvil))
}

#[tokio::test]
async fn test_factory_creates_contract() -> Result<()> {
    let (_, endpoint, _anvil) = setup_provider().await?;
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"; // Anvil default
    let contract_address = "0x5FbDB2315678afecb367f032d93F642f64180aa3"; // Dummy address

    let contract =
        CRISPContractFactory::create_write(&endpoint, contract_address, private_key).await?;

    // Verify the contract was created successfully
    assert_eq!(
        contract.address().to_string().to_lowercase(),
        contract_address.to_lowercase()
    );

    Ok(())
}

#[tokio::test]
async fn test_factory_invalid_address() {
    let (_, endpoint, _anvil) = setup_provider().await.unwrap();
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let invalid_address = "not-an-address";

    let result = CRISPContractFactory::create_write(&endpoint, invalid_address, private_key).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_factory_invalid_private_key() {
    let (_, endpoint, _anvil) = setup_provider().await.unwrap();
    let invalid_key = "not-a-key";
    let contract_address = "0x5FbDB2315678afecb367f032d93F642f64180aa3";

    let result = CRISPContractFactory::create_write(&endpoint, contract_address, invalid_key).await;
    assert!(result.is_err());
}
