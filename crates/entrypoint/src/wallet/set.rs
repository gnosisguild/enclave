// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::{hex::FromHex, primitives::FixedBytes, signers::local::PrivateKeySigner};
use alloy_primitives::Address;
use anyhow::{anyhow, Result};
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_evm::EthPrivateKeyRepositoryFactory;
use e3_net::NetRepositoryFactory;
use libp2p::{identity::Keypair, PeerId};
use zeroize::{Zeroize, Zeroizing};

use crate::helpers::{datastore::get_repositories, rand::generate_random_bytes};

pub fn validate_private_key(input: &String) -> Result<()> {
    let bytes =
        FixedBytes::<32>::from_hex(input).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    let _ =
        PrivateKeySigner::from_bytes(&bytes).map_err(|e| anyhow!("Invalid private key: {}", e))?;
    Ok(())
}

pub async fn execute(config: &AppConfig, input: Zeroizing<String>) -> Result<(Address, PeerId)> {
    let cipher = Cipher::from_file(config.key_file()).await?;

    let (encrypted_private_key, encrypted_keypair, address, peer_id) = process_key(&cipher, input)?;

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
    Ok((address, peer_id))
}

fn process_key(
    cipher: &Cipher,
    private_key: Zeroizing<String>,
) -> Result<(Vec<u8>, Vec<u8>, Address, PeerId)> {
    let private_key_bytes = FixedBytes::<32>::from_hex(private_key)?;
    let keypair = Keypair::ed25519_from_bytes(&mut private_key_bytes.clone())?;
    let peer_id = PeerId::from(&keypair.public());
    let mut keypair = keypair.try_into_ed25519()?.to_bytes().to_vec();
    let address = PrivateKeySigner::from_bytes(&private_key_bytes)?.address();
    let encrypted_private_key = cipher.encrypt_data(&mut private_key_bytes.to_vec())?;
    let encrypted_keypair = cipher.encrypt_data(&mut keypair)?;

    Ok((encrypted_private_key, encrypted_keypair, address, peer_id))
}

pub async fn autowallet(config: &AppConfig) -> Result<()> {
    let mut bytes = generate_random_bytes(32);
    let input = Zeroizing::new(hex::encode(&bytes));
    bytes.zeroize();
    execute(config, input).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_key() -> Result<()> {
        let cipher = Cipher::from_password("test_password").await?;
        // Hardhat default private key
        let input = Zeroizing::new(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string(),
        );

        let (encrypted_private_key, encrypted_keypair, address, peer_id) =
            process_key(&cipher, input)?;

        assert!(!encrypted_private_key.is_empty());
        assert!(!encrypted_keypair.is_empty());
        assert_eq!(
            address,
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".parse::<Address>()?
        );
        assert_eq!(
            &peer_id.to_string(),
            "12D3KooWEZiPVmEZkwCFEWYxPL6xts6LnPHRFqsSEDGmt1vQ17By"
        );

        Ok(())
    }
}
