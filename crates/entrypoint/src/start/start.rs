// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_ciphernode_builder::{CiphernodeBuilder, CiphernodeHandle};
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_zk_prover::ZkBackend;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use std::sync::{Arc, Mutex};
use tracing::instrument;

#[instrument(name = "app", skip_all)]
pub async fn execute(config: &AppConfig) -> Result<CiphernodeHandle> {
    let rng = Arc::new(Mutex::new(rand_chacha::ChaCha20Rng::from_rng(OsRng)?));
    let cipher = Arc::new(Cipher::from_file(&config.key_file()).await?);
    let zk_config = e3_zk_prover::ZkConfig::fetch_or_default().await;
    let backend = ZkBackend::new(
        config.bb_binary(),
        config.circuits_dir(),
        config.work_dir(),
        zk_config,
    );
    backend.ensure_installed().await?;

    let node = CiphernodeBuilder::new(rng.clone(), cipher.clone())
        .with_persistence(&config.log_file(), &config.db_file())
        .with_sortition_score()
        .with_chains(&config.chains())
        .with_contract_enclave_reader()
        .with_contract_bonding_registry()
        .with_max_threads()
        .with_contract_ciphernode_registry()
        .with_trbfv()
        .with_zkproof(backend)
        .with_net(config.peers(), config.quic_port())
        .build()
        .await?;

    Ok(node)
}
