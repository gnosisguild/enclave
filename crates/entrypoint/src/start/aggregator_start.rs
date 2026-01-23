// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::Address;
use anyhow::Result;
use e3_ciphernode_builder::CiphernodeBuilder;
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_data::RepositoriesFactory;
use e3_events::BusHandle;
use e3_net::{NetEventTranslator, NetRepositoryFactory};
use e3_test_helpers::{PlaintextWriter, PublicKeyWriter};
use rand::SeedableRng;
use rand_chacha::{rand_core::OsRng, ChaCha20Rng};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

pub async fn execute(
    config: &AppConfig,
    address: Address,
    pubkey_write_path: Option<PathBuf>,
    plaintext_write_path: Option<PathBuf>,
) -> Result<(BusHandle, JoinHandle<Result<()>>, String)> {
    let rng = Arc::new(Mutex::new(ChaCha20Rng::from_rng(OsRng)?));
    let cipher = Arc::new(Cipher::from_file(config.key_file()).await?);
    let builder = CiphernodeBuilder::new(&config.name(), rng.clone(), cipher.clone())
        .with_address(&address.to_string())
        .with_persistence(&config.log_file(), &config.db_file())
        .with_chains(&config.chains())
        .with_sortition_score()
        .with_contract_enclave_full()
        .with_contract_bonding_registry()
        .with_contract_ciphernode_registry()
        .with_max_threads()
        .with_pubkey_aggregation()
        .with_threshold_plaintext_aggregation();

    // TODO: put net package provisioning in the ciphernode-builder:
    let node = builder.build().await?;
    let store = node.store();
    let repositories = store.repositories();
    let bus = node.bus.clone();
    let (_, _, join_handle, peer_id) = NetEventTranslator::setup_with_interface(
        bus.clone(),
        config.peers(),
        &cipher,
        config.quic_port(),
        repositories.libp2p_keypair(),
    )
    .await?;

    // These are here purely for our integration test so leaving out of the builder
    if let Some(path) = pubkey_write_path {
        PublicKeyWriter::attach(&path, bus.clone());
    }

    if let Some(path) = plaintext_write_path {
        PlaintextWriter::attach(&path, bus.clone());
    }

    Ok((bus, join_handle, peer_id))
}
