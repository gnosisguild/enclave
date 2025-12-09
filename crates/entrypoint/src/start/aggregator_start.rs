// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_ciphernode_builder::{get_enclave_bus_handle, CiphernodeBuilder};
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

use crate::helpers::datastore::setup_datastore;

pub async fn execute(
    config: &AppConfig,
    pubkey_write_path: Option<PathBuf>,
    plaintext_write_path: Option<PathBuf>,
    experimental_trbfv: bool,
) -> Result<(BusHandle, JoinHandle<Result<()>>, String)> {
    let bus = get_enclave_bus_handle();
    let rng = Arc::new(Mutex::new(ChaCha20Rng::from_rng(OsRng)?));
    let store = setup_datastore(config, &bus)?;
    let repositories = store.repositories();
    let cipher = Arc::new(Cipher::from_file(config.key_file()).await?);

    let mut builder = CiphernodeBuilder::new(rng.clone(), cipher.clone())
        .with_source_bus(bus.consumer())
        .with_datastore(store)
        .with_chains(&config.chains())
        .with_sortition_score()
        .with_contract_enclave_full()
        .with_contract_bonding_registry()
        .with_contract_ciphernode_registry()
        .with_max_threads()
        .with_pubkey_aggregation();

    if experimental_trbfv {
        builder = builder.with_threshold_plaintext_aggregation();
    } else {
        builder = builder.with_plaintext_aggregation()
    }
    builder.build().await?;
    let (_, _, join_handle, peer_id) = NetEventTranslator::setup_with_interface(
        bus.clone(),
        config.peers(),
        &cipher,
        config.quic_port(),
        repositories.libp2p_keypair(),
        experimental_trbfv,
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
