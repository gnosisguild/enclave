use actix::{Actor, Addr, Context};
use anyhow::Result;
use config::AppConfig;
use data::{DataStore, InMemStore, SledStore};
use enclave_core::EventBus;
use evm::{
    helpers::pull_eth_signer_from_env, CiphernodeRegistrySol, EnclaveSol, RegistryFilterSol,
};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use router::{
    E3RequestRouter, FheFeature, PlaintextAggregatorFeature, PublicKeyAggregatorFeature,
    RepositoriesFactory,
};
use sortition::Sortition;
use std::sync::{Arc, Mutex};
use test_helpers::{PlaintextWriter, PublicKeyWriter};
use tokio::task::JoinHandle;


/// Main Ciphernode Actor
/// Suprvises all children
// TODO: add supervision logic
pub struct MainAggregator {
    e3_manager: Addr<E3RequestRouter>,
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    p2p: Addr<P2p>,
}

impl MainAggregator {
    pub fn new(
        bus: &Addr<EventBus>,
        sortition: Addr<Sortition>,
        p2p: Addr<P2p>,
        e3_manager: Addr<E3RequestRouter>,
    ) -> Self {
        Self {
            e3_manager,
            bus: bus.clone(),
            sortition,
            p2p,
        }
    }

    pub async fn attach(
        config: AppConfig,
        pubkey_write_path: Option<&str>,
        plaintext_write_path: Option<&str>,
    ) -> Result<(Addr<EventBus>, JoinHandle<()>)> {
        let bus = EventBus::new(true).start();
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));

        let store: DataStore = if !config.use_in_mem_store() {
            (&SledStore::new(&bus, &config.db_file().to_string_lossy())?.start()).into()
        } else {
            (&InMemStore::new(true).start()).into()
        };

        let repositories = store.repositories();
        let sortition = Sortition::attach(&bus, repositories.sortition());
        let signer = pull_eth_signer_from_env("PRIVATE_KEY").await?;
        for chain in config
            .chains()
            .iter()
            .filter(|chain| chain.enabled.unwrap_or(true))
        {
            let rpc_url = &chain.rpc_url;
            EnclaveSol::attach(&bus, rpc_url, &chain.contracts.enclave, &signer).await?;
            RegistryFilterSol::attach(&bus, rpc_url, &chain.contracts.filter_registry, &signer)
                .await?;
            CiphernodeRegistrySol::attach(&bus, rpc_url, &chain.contracts.ciphernode_registry)
                .await?;
        }

        let e3_manager = E3RequestRouter::builder(&bus, store)
            .add_feature(FheFeature::create(&bus, &rng))
            .add_feature(PublicKeyAggregatorFeature::create(&bus, &sortition))
            .add_feature(PlaintextAggregatorFeature::create(&bus, &sortition))
            .build()
            .await?;

        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

        if let Some(path) = pubkey_write_path {
            PublicKeyWriter::attach(path, bus.clone());
        }

        if let Some(path) = plaintext_write_path {
            PlaintextWriter::attach(path, bus.clone());
        }

        SimpleLogger::attach("AGG", bus.clone());

        MainAggregator::new(&bus, sortition, p2p_addr, e3_manager).start();
        Ok((bus, join_handle))
    }
}

impl Actor for MainAggregator {
    type Context = Context<Self>;
}
