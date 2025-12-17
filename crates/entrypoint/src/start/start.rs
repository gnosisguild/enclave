// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::Address;
use anyhow::Result;
use e3_ciphernode_builder::{get_enclave_bus_handle, CiphernodeBuilder};
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_data::RepositoriesFactory;
use e3_events::BusHandle;
use e3_net::{NetEventTranslator, NetRepositoryFactory};
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::instrument;

/// Start and configure a ciphernode, initialize networking, and prepare the background event task.
///
/// This function builds a ciphernode using values from `config` and `address`, initializes the enclave
/// bus and cipher, configures node features (including TRBFV or keyshare depending on
/// `experimental_trbfv`), and sets up the network event translator. It returns the enclave bus handle,
/// the JoinHandle for the running background task, and the local libp2p peer id.
///
/// # Parameters
///
/// * `config` - Application configuration used to configure the node and networking.
/// * `address` - Network address to bind the node to.
/// * `experimental_trbfv` - When `true`, enable TRBFV support; otherwise enable keyshare support.
///
/// # Returns
///
/// A tuple containing:
/// 1. the enclave `BusHandle`,
/// 2. a `JoinHandle<Result<()>>` for the node's background task,
/// 3. the local libp2p peer id as a `String`.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // let config = AppConfig::load("config.toml")?; // hypothetical loader
/// // let address = Address::from_str("127.0.0.1:9000")?;
/// // let (bus, join_handle, peer_id) = crate::start::execute(&config, address, false).await?;
/// # Ok(())
/// # }
/// ```
#[instrument(name = "app", skip_all)]
pub async fn execute(
    config: &AppConfig,
    address: Address,
    experimental_trbfv: bool,
) -> Result<(BusHandle, JoinHandle<Result<()>>, String)> {
    let rng = Arc::new(Mutex::new(rand_chacha::ChaCha20Rng::from_rng(OsRng)?));

    let bus = get_enclave_bus_handle(config)?;
    let cipher = Arc::new(Cipher::from_file(&config.key_file()).await?);

    let mut builder = CiphernodeBuilder::new(&config.name(), rng.clone(), cipher.clone())
        .with_address(&address.to_string())
        .with_source_bus(bus.consumer())
        .with_sortition_score()
        .with_chains(&config.chains())
        .with_persistence(&config.log_file(), &config.db_file())
        .with_contract_enclave_reader()
        .with_contract_bonding_registry()
        .with_max_threads()
        .with_contract_ciphernode_registry();

    if experimental_trbfv {
        builder = builder.with_trbfv();
    } else {
        builder = builder.with_keyshare();
    }

    let node = builder.build().await?;
    let repositories = node.store().repositories();
    let (_, _, join_handle, peer_id) = NetEventTranslator::setup_with_interface(
        bus.clone(),
        config.peers(),
        &cipher,
        config.quic_port(),
        repositories.libp2p_keypair(),
        experimental_trbfv,
    )
    .await?;

    Ok((bus, join_handle, peer_id))
}