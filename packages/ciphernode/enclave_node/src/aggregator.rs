use actix::{Actor, Addr};
use anyhow::Result;
use cipher::Cipher;
use config::AppConfig;
use data::RepositoriesFactory;
use enclave_core::EventBus;
use evm::{
    helpers::{get_signer_from_repository, ProviderConfig, RPC},
    CiphernodeRegistryReaderRepositoryFactory, CiphernodeRegistrySol, EnclaveSol,
    EnclaveSolReaderRepositoryFactory, EthPrivateKeyRepositoryFactory, RegistryFilterSol,
};
use logger::SimpleLogger;
use net::NetworkRelay;
use rand::SeedableRng;
use rand_chacha::{rand_core::OsRng, ChaCha20Rng};
use router::{E3RequestRouter, FheFeature, PlaintextAggregatorFeature, PublicKeyAggregatorFeature};
use sortition::{Sortition, SortitionRepositoryFactory};
use std::sync::{Arc, Mutex};
use test_helpers::{PlaintextWriter, PublicKeyWriter};
use tokio::task::JoinHandle;

use crate::setup_datastore;

pub async fn setup_aggregator(
    config: AppConfig,
    pubkey_write_path: Option<&str>,
    plaintext_write_path: Option<&str>,
) -> Result<(Addr<EventBus>, JoinHandle<Result<()>>, String)> {
    let bus = EventBus::new(true).start();
    let rng = Arc::new(Mutex::new(
        ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
    ));
    let store = setup_datastore(&config, &bus)?;
    let repositories = store.repositories();
    let sortition = Sortition::attach(&bus, repositories.sortition()).await?;
    let cipher = Arc::new(Cipher::from_config(&config).await?);
    let signer = get_signer_from_repository(repositories.eth_private_key(), &cipher).await?;

    for chain in config
        .chains()
        .iter()
        .filter(|chain| chain.enabled.unwrap_or(true))
    {
        let rpc_url = RPC::from_url(&chain.rpc_url).map_err(|e| {
            anyhow::anyhow!("Failed to parse RPC URL for chain {}: {}", chain.name, e)
        })?;
        let provider_config = ProviderConfig::new(rpc_url, chain.rpc_auth.clone());
        let read_provider = provider_config.create_readonly_provider().await?;
        let write_provider = provider_config.create_ws_signer_provider(&signer).await?;

        EnclaveSol::attach(
            &bus,
            &read_provider,
            &write_provider,
            &chain.contracts.enclave.address(),
            &repositories.enclave_sol_reader(read_provider.get_chain_id()),
            chain.contracts.enclave.deploy_block(),
        )
        .await?;
        RegistryFilterSol::attach(
            &bus,
            &write_provider,
            &chain.contracts.filter_registry.address(),
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

    E3RequestRouter::builder(&bus, store)
        .add_feature(FheFeature::create(&bus, &rng))
        .add_feature(PublicKeyAggregatorFeature::create(&bus, &sortition))
        .add_feature(PlaintextAggregatorFeature::create(&bus, &sortition))
        .build()
        .await?;

    let (_, join_handle, peer_id) = NetworkRelay::setup_with_peer(bus.clone(), config.peers())?;

    if let Some(path) = pubkey_write_path {
        PublicKeyWriter::attach(path, bus.clone());
    }

    if let Some(path) = plaintext_write_path {
        PlaintextWriter::attach(path, bus.clone());
    }

    SimpleLogger::attach("AGG", bus.clone());

    Ok((bus, join_handle, peer_id))
}
