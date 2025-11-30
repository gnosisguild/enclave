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
use e3_events::{get_enclave_bus_handle, EnclaveEvent};
use e3_events::{prelude::*, BusHandle};
use e3_net::{NetEventTranslator, NetRepositoryFactory};
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::helpers::datastore::setup_datastore;

#[instrument(name = "app", skip_all)]
pub async fn execute(
    config: &AppConfig,
    address: Address,
    experimental_trbfv: bool,
) -> Result<(BusHandle<EnclaveEvent>, JoinHandle<Result<()>>, String)> {
    let rng = Arc::new(Mutex::new(rand_chacha::ChaCha20Rng::from_rng(OsRng)?));

    let bus = get_enclave_bus_handle();
    let cipher = Arc::new(Cipher::from_file(&config.key_file()).await?);
    let store = setup_datastore(&config, &bus)?;
    let repositories = store.repositories();

    let mut builder = CiphernodeBuilder::new(rng.clone(), cipher.clone())
        .with_address(&address.to_string())
        .with_source_bus(&bus.bus())
        .with_datastore(store)
        .with_sortition_score()
        .with_chains(&config.chains())
        .with_contract_enclave_reader()
        .with_contract_bonding_registry()
        .with_max_threads()
        .with_contract_ciphernode_registry();

    if experimental_trbfv {
        builder = builder.with_trbfv();
    } else {
        builder = builder.with_keyshare();
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

    Ok((bus, join_handle, peer_id))
}
