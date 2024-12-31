use actix::{Actor, Addr};
use aggregator::{PlaintextAggregatorFeature, PublicKeyAggregatorFeature};
use anyhow::Result;
use config::AppConfig;
use crypto::Cipher;
use data::RepositoriesFactory;
use e3_request::E3Router;
use events::EventBus;
use evm::{
    helpers::{get_signer_from_repository, ProviderConfig},
    CiphernodeRegistryReaderRepositoryFactory, CiphernodeRegistrySol, EnclaveSol,
    EnclaveSolReaderRepositoryFactory, EthPrivateKeyRepositoryFactory, RegistryFilterSol,
};
use fhe::FheFeature;
use logger::SimpleLogger;
use net::{NetRepositoryFactory, NetworkManager};
use rand::SeedableRng;
use rand_chacha::{rand_core::OsRng, ChaCha20Rng};
use sortition::Sortition;
use sortition::SortitionRepositoryFactory;
use std::sync::{Arc, Mutex};
use test_helpers::{PlaintextWriter, PublicKeyWriter};

use crate::helpers::datastore::setup_datastore;

pub async fn execute(
    config: AppConfig,
    pubkey_write_path: Option<&str>,
    plaintext_write_path: Option<&str>,
) -> Result<(Addr<EventBus>, String)> {
    let bus = EventBus::new(true).start();
    let rng = Arc::new(Mutex::new(ChaCha20Rng::from_rng(OsRng)?));
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
        let rpc_url = chain.rpc_url()?;
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

    E3Router::builder(&bus, store)
        .add_feature(FheFeature::create(&bus, &rng))
        .add_feature(PublicKeyAggregatorFeature::create(&bus, &sortition))
        .add_feature(PlaintextAggregatorFeature::create(&bus, &sortition))
        .build()
        .await?;

    let (_, peer_id) = NetworkManager::setup_with_peer(
        bus.clone(),
        config.peers(),
        &cipher,
        config.quic_port(),
        config.enable_mdns(),
        repositories.libp2p_keypair(),
    )
    .await?;

    if let Some(path) = pubkey_write_path {
        PublicKeyWriter::attach(path, bus.clone());
    }

    if let Some(path) = plaintext_write_path {
        PlaintextWriter::attach(path, bus.clone());
    }

    SimpleLogger::attach("AGG", bus.clone());

    Ok((bus, peer_id))
}
