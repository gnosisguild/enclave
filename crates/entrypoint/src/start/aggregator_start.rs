// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_ciphernode_builder::{CiphernodeBuilder, CiphernodeHandle};
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_test_helpers::{PlaintextWriter, PublicKeyWriter};
use rand::SeedableRng;
use rand_chacha::{rand_core::OsRng, ChaCha20Rng};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub async fn execute(
    config: &AppConfig,
    pubkey_write_path: Option<PathBuf>,
    plaintext_write_path: Option<PathBuf>,
) -> Result<CiphernodeHandle> {
    let rng = Arc::new(Mutex::new(ChaCha20Rng::from_rng(OsRng)?));
    let cipher = Arc::new(Cipher::from_file(config.key_file()).await?);
    let node = CiphernodeBuilder::new(rng.clone(), cipher.clone())
        .with_persistence(&config.log_file(), &config.db_file())
        .with_chains(&config.chains())
        .with_sortition_score()
        .with_contract_enclave_full()
        .with_contract_bonding_registry()
        .with_contract_ciphernode_registry()
        .with_max_threads()
        .with_pubkey_aggregation()
        .with_threshold_plaintext_aggregation()
        .with_net(config.peers(), config.quic_port())
        .build()
        .await?;

    // These are here purely for our integration test so leaving out of the builder
    if let Some(path) = pubkey_write_path {
        PublicKeyWriter::attach(&path, node.bus().clone());
    }

    if let Some(path) = plaintext_write_path {
        PlaintextWriter::attach(&path, node.bus().clone());
    }

    Ok(node)
}
