use actix::Addr;
use alloy::primitives::Address;
use anyhow::Result;
use e3_config::AppConfig;
use e3_crypto::Cipher;
use e3_data::RepositoriesFactory;
use e3_events::get_enclave_event_bus;
use e3_events::{EnclaveEvent, EventBus};
use e3_evm::{
    helpers::ProviderConfig, CiphernodeRegistryReaderRepositoryFactory, CiphernodeRegistrySol,
    EnclaveSolReader, EnclaveSolReaderRepositoryFactory,
};
use e3_fhe::ext::FheExtension;
use e3_keyshare::ext::KeyshareExtension;
use e3_net::{NetRepositoryFactory, NetworkManager};
use e3_request::E3Router;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use sortition::CiphernodeSelector;
use sortition::Sortition;
use sortition::SortitionRepositoryFactory;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::helpers::datastore::setup_datastore;

#[instrument(name = "app", skip_all)]
pub async fn execute(
    config: &AppConfig,
    address: Address,
) -> Result<(Addr<EventBus<EnclaveEvent>>, JoinHandle<Result<()>>, String)> {
    let rng = Arc::new(Mutex::new(rand_chacha::ChaCha20Rng::from_rng(OsRng)?));

    let bus = get_enclave_event_bus();
    let cipher = Arc::new(Cipher::from_config(&config).await?);
    let store = setup_datastore(&config, &bus)?;

    let repositories = store.repositories();

    let sortition = Sortition::attach(&bus, repositories.sortition()).await?;
    CiphernodeSelector::attach(&bus, &sortition, &address.to_string());

    // TODO: gather an async handle from the event readers that closes when they shutdown and
    // join it with the network manager joinhandle below
    for chain in config
        .chains()
        .iter()
        .filter(|chain| chain.enabled.unwrap_or(true))
    {
        let rpc_url = chain.rpc_url()?;
        let provider_config = ProviderConfig::new(rpc_url, chain.rpc_auth.clone());
        let read_provider = provider_config.create_readonly_provider().await?;
        EnclaveSolReader::attach(
            &bus,
            &read_provider,
            &chain.contracts.enclave.address(),
            &repositories.enclave_sol_reader(read_provider.get_chain_id()),
            chain.contracts.enclave.deploy_block(),
            chain.rpc_url.clone(),
        )
        .await?;
        CiphernodeRegistrySol::attach(
            &bus,
            &read_provider,
            &chain.contracts.ciphernode_registry.address(),
            &repositories.ciphernode_registry_reader(read_provider.get_chain_id()),
            chain.contracts.ciphernode_registry.deploy_block(),
            chain.rpc_url.clone(),
        )
        .await?;
    }

    E3Router::builder(&bus, store.clone())
        .with(FheExtension::create(&bus, &rng))
        .with(KeyshareExtension::create(
            &bus,
            &address.to_string(),
            &cipher,
        ))
        .build()
        .await?;

    let (_, join_handle, peer_id) = NetworkManager::setup_with_peer(
        bus.clone(),
        config.peers(),
        &cipher,
        config.quic_port(),
        config.enable_mdns(),
        repositories.libp2p_keypair(),
    )
    .await?;

    Ok((bus, join_handle, peer_id))
}
