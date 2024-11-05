use actix::{Actor, Addr};
use alloy::primitives::Address;
use anyhow::Result;
use cipher::Cipher;
use config::AppConfig;
use enclave_core::EventBus;
use evm::{
    helpers::{create_readonly_provider, ensure_ws_rpc},
    CiphernodeRegistrySol, EnclaveSolReader,
};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use router::{
    CiphernodeSelector, E3RequestRouter, FheFeature, KeyshareFeature, RepositoriesFactory,
};
use sortition::Sortition;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::setup_datastore;

pub async fn setup_ciphernode(
    config: AppConfig,
    address: Address,
) -> Result<(Addr<EventBus>, JoinHandle<()>)> {
    let rng = Arc::new(Mutex::new(
        rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
    ));
    let bus = EventBus::new(true).start();
    let cipher = Arc::new(Cipher::from_config(&config).await?);
    let store = setup_datastore(&config, &bus)?;

    let repositories = store.repositories();

    let sortition = Sortition::attach(&bus, repositories.sortition());
    CiphernodeSelector::attach(&bus, &sortition, &address.to_string());

    for chain in config
        .chains()
        .iter()
        .filter(|chain| chain.enabled.unwrap_or(true))
    {
        let rpc_url = &chain.rpc_url;

        let read_provider = create_readonly_provider(&ensure_ws_rpc(rpc_url)).await?;
        EnclaveSolReader::attach(
            &bus,
            &read_provider,
            &chain.contracts.enclave.address(),
            &repositories.enclave_sol_reader(read_provider.get_chain_id()),
            chain.contracts.enclave.deploy_block(),
        )
        .await?;
        CiphernodeRegistrySol::attach(
            &bus,
            &read_provider,
            &chain.contracts.ciphernode_registry.address(),
            &repositories.ciphernode_registry_reader(read_provider.get_chain_id()),
            chain.contracts.ciphernode_registry.deploy_block(),
        )
        .await?;
    }

    E3RequestRouter::builder(&bus, store.clone())
        .add_feature(FheFeature::create(&bus, &rng))
        .add_feature(KeyshareFeature::create(&bus, &address.to_string(), &cipher))
        .build()
        .await?;

    let (_, join_handle) = P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

    let nm = format!("CIPHER({})", &address.to_string()[0..5]);
    SimpleLogger::attach(&nm, bus.clone());

    Ok((bus, join_handle))
}
