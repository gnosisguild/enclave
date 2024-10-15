use actix::{Actor, Addr, Context};
use alloy::primitives::Address;
use anyhow::Result;
use data::{DataStore, InMemDataStore};
use enclave_core::EventBus;
use evm::{CiphernodeRegistrySol, EnclaveSolReader};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use router::{CiphernodeSelector, E3RequestRouter, FheFeature, KeyshareFeature};
use sortition::Sortition;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::app_config::AppConfig;

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
    ) -> Result<(Addr<Self>, JoinHandle<()>)> {
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));
        let bus = EventBus::new(true).start();
        // TODO: switch to Sled actor
        let data = DataStore::from_in_mem(InMemDataStore::new(true).start());
        let sortition = Sortition::attach(bus.clone());
        let selector =
            CiphernodeSelector::attach(bus.clone(), sortition.clone(), &address.to_string());

        for chain in config
            .chains
            .iter()
            .filter(|chain| chain.enabled.unwrap_or(true))
        {
            let rpc_url = &chain.rpc_url;

            EnclaveSolReader::attach(bus.clone(), rpc_url, &chain.contracts.enclave).await?;
            CiphernodeRegistrySol::attach(
                bus.clone(),
                rpc_url,
                &chain.contracts.ciphernode_registry,
            )
            .await?;
        }

        let e3_manager = E3RequestRouter::builder(bus.clone(), data.clone())
            .add_feature(FheFeature::create(rng))
            .add_feature(KeyshareFeature::create(bus.clone(), &address.to_string()))
            .build();

        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

        let nm = format!("CIPHER({})", &address.to_string()[0..5]);
        SimpleLogger::attach(&nm, bus.clone());
        let main_addr = MainCiphernode::new(
            address, bus, data, sortition, selector, p2p_addr, e3_manager,
        )
        .start();
        Ok((main_addr, join_handle))
    }
}

impl Actor for MainCiphernode {
    type Context = Context<Self>;
}
