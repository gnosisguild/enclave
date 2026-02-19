// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::{hex::FromHex, primitives::FixedBytes, signers::local::PrivateKeySigner};
use anyhow::{anyhow, Result};
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_evm::EthPrivateKeyRepositoryFactory;
use e3_net::NetRepositoryFactory;
use libp2p::identity::Keypair;
use zeroize::{Zeroize, Zeroizing};

use crate::helpers::{datastore::get_repositories, rand::generate_random_bytes};

pub fn validate_private_key(input: &String) -> Result<()> {
    let bytes =
        FixedBytes::<32>::from_hex(input).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    let _ =
        PrivateKeySigner::from_bytes(&bytes).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    Ok(())
}

pub async fn execute(config: &AppConfig, input: Zeroizing<String>) -> Result<()> {
    let cipher = Cipher::from_file(config.key_file()).await?;
    let mut bytes = input.as_bytes().to_vec();

    let mut keypair = Keypair::ed25519_from_bytes(&mut bytes.clone())? // this zeroizes bytes so cloning ok
        .try_into_ed25519()?
        .to_bytes()
        .to_vec();

    let encrypted_private_key = cipher.encrypt_data(&mut bytes)?; // This zeroizes input
    let encrypted_keypair = cipher.encrypt_data(&mut keypair)?; // This zeroizes input

    // Save the encrypted keys
    let repositories = get_repositories(config)?;
    repositories
        .eth_private_key()
        .write_sync(&encrypted_private_key)
        .await?;

    repositories
        .libp2p_keypair()
        .write_sync(&encrypted_keypair)
        .await?;

    Ok(())
}

pub async fn autowallet(config: &AppConfig) -> Result<()> {
    let mut bytes = generate_random_bytes(32);
    let input = Zeroizing::new(hex::encode(&bytes));
    bytes.zeroize();
    execute(config, input).await?;
    Ok(())
}
