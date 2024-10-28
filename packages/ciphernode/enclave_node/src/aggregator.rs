use actix::{Actor, Addr};
use anyhow::{bail, Result};
use cipher::Cipher;
use config::AppConfig;
use enclave_core::EventBus;
use evm::{
    helpers::get_signer_from_repository, CiphernodeRegistrySol, EnclaveSol, RegistryFilterSol,
};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::{rand_core::OsRng, ChaCha20Rng};
use router::{
    E3RequestRouter, FheFeature, PlaintextAggregatorFeature, PublicKeyAggregatorFeature,
    RepositoriesFactory,
};
use sortition::Sortition;
use std::sync::{Arc, Mutex};
use test_helpers::{PlaintextWriter, PublicKeyWriter};
use tokio::task::JoinHandle;

use crate::setup_datastore;

pub async fn setup_aggregator(
    config: AppConfig,
    pubkey_write_path: Option<&str>,
    plaintext_write_path: Option<&str>,
) -> Result<(Addr<EventBus>, JoinHandle<()>)> {
    let bus = EventBus::new(true).start();
    let rng = Arc::new(Mutex::new(
        ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
    ));
    let store = setup_datastore(&config, &bus)?;
    let repositories = store.repositories();
    let sortition = Sortition::attach(&bus, repositories.sortition());
    let cipher = Arc::new(Cipher::from_config(&config).await?);
    let signer = get_signer_from_repository(repositories.eth_private_key(),&cipher).await?;
    for chain in config
        .chains()
        .iter()
        .filter(|chain| chain.enabled.unwrap_or(true))
    {
        let rpc_url = &chain.rpc_url;
        EnclaveSol::attach(&bus, rpc_url, &chain.contracts.enclave, &signer).await?;
        RegistryFilterSol::attach(&bus, rpc_url, &chain.contracts.filter_registry, &signer).await?;
        CiphernodeRegistrySol::attach(&bus, rpc_url, &chain.contracts.ciphernode_registry).await?;
    }

    E3RequestRouter::builder(&bus, store)
        .add_feature(FheFeature::create(&bus, &rng))
        .add_feature(PublicKeyAggregatorFeature::create(&bus, &sortition))
        .add_feature(PlaintextAggregatorFeature::create(&bus, &sortition))
        .build()
        .await?;

    let (_, join_handle) = P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

    if let Some(path) = pubkey_write_path {
        PublicKeyWriter::attach(path, bus.clone());
    }

    if let Some(path) = plaintext_write_path {
        PlaintextWriter::attach(path, bus.clone());
    }

    SimpleLogger::attach("AGG", bus.clone());

    Ok((bus, join_handle))
}
