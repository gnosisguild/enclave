use actix::{Actor, Addr, Context};
use alloy::primitives::Address;
use anyhow::Result;
use config::AppConfig;
use data::{DataStore, InMemStore, SledStore};
use enclave_core::EventBus;
use evm::{CiphernodeRegistrySol, EnclaveSolReader};
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

/// Main Ciphernode Actor
/// Suprvises all children
// TODO: add supervision logic
pub struct MainCiphernode {
    addr: Address,
    bus: Addr<EventBus>,
    data: DataStore,
    sortition: Addr<Sortition>,
    selector: Addr<CiphernodeSelector>,
    e3_manager: Addr<E3RequestRouter>,
    p2p: Addr<P2p>,
}

impl MainCiphernode {
    pub fn new(
        addr: Address,
        bus: Addr<EventBus>,
        data: DataStore,
        sortition: Addr<Sortition>,
        selector: Addr<CiphernodeSelector>,
        p2p: Addr<P2p>,
        e3_manager: Addr<E3RequestRouter>,
    ) -> Self {
        Self {
            addr,
            bus,
            data,
            sortition,
            selector,
            e3_manager,
            p2p,
        }
    }

    pub async fn attach(
        config: AppConfig,
        address: Address,
    ) -> Result<(Addr<EventBus>, JoinHandle<()>)> {
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));
        let bus = EventBus::new(true).start();

        let store: DataStore = if !config.use_in_mem_store() {
            (&SledStore::new(&bus, &config.db_file().to_string_lossy())?.start()).into()
        } else {
            (&InMemStore::new(true).start()).into()
        };

        let repositories = store.repositories();

        let sortition = Sortition::attach(&bus, repositories.sortition());
        let selector = CiphernodeSelector::attach(&bus, &sortition, &address.to_string());

        for chain in config
            .chains()
            .iter()
            .filter(|chain| chain.enabled.unwrap_or(true))
        {
            let rpc_url = &chain.rpc_url;

            EnclaveSolReader::attach(&bus, rpc_url, &chain.contracts.enclave).await?;
            CiphernodeRegistrySol::attach(&bus, rpc_url, &chain.contracts.ciphernode_registry)
                .await?;
        }

        let e3_manager = E3RequestRouter::builder(&bus, store.clone())
            .add_feature(FheFeature::create(&bus, &rng))
            .add_feature(KeyshareFeature::create(&bus, &address.to_string()))
            .build()
            .await?;

        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

        let nm = format!("CIPHER({})", &address.to_string()[0..5]);
        SimpleLogger::attach(&nm, bus.clone());
        MainCiphernode::new(
            address,
            bus.clone(),
            store,
            sortition,
            selector,
            p2p_addr,
            e3_manager,
        )
        .start();
        Ok((bus, join_handle))
    }
}

impl Actor for MainCiphernode {
    type Context = Context<Self>;
}
