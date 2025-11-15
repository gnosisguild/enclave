// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::{Address, U256};
use alloy::sol;
use eyre::Result;

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    contract ERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
    }
}

pub async fn approve_token(
    http_rpc_url: &str,
    private_key: &str,
    token_address: &str,
    spender_address: &str,
    amount: U256,
) -> Result<()> {
    use alloy::network::EthereumWallet;
    use alloy::providers::{Provider, ProviderBuilder};
    use alloy::signers::local::PrivateKeySigner;

    let token_address: Address = token_address.parse()?;
    let spender_address: Address = spender_address.parse()?;
    let signer: PrivateKeySigner = private_key.parse()?;
    let wallet = EthereumWallet::from(signer.clone());

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(http_rpc_url)
        .await?;

    let contract = ERC20::new(token_address, &provider);
    let owner = signer.clone().address();
    let current_allowance = contract.allowance(owner, spender_address).call().await?;

    log::info!("Current allowance: {}", current_allowance);

    if current_allowance < amount {
        log::info!(
            "Approving {} tokens for spender {}",
            amount,
            spender_address
        );
        let builder = contract.approve(spender_address, amount);
        let receipt = builder.send().await?.get_receipt().await?;
        log::info!(
            "Approval successful. TxHash: {:?}",
            receipt.transaction_hash
        );
    } else {
        log::info!("Sufficient allowance already exists");
    }

    Ok(())
}
