use actix::{Actor, Addr, Context};
use anyhow::Result;
use data::{DataStore, InMemDataStore};
use enclave_core::EventBus;
use evm::{
    helpers::pull_eth_signer_from_env, CiphernodeRegistrySol, EnclaveSol, RegistryFilterSol,
};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use router::{E3RequestRouter, FheFeature, PlaintextAggregatorFeature, PublicKeyAggregatorFeature};
use sortition::Sortition;
use std::sync::{Arc, Mutex};
use test_helpers::{PlaintextWriter, PublicKeyWriter};
use tokio::task::JoinHandle;

use crate::app_config::AppConfig;

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
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        p2p: Addr<P2p>,
        e3_manager: Addr<E3RequestRouter>,
    ) -> Self {
        Self {
            e3_manager,
            bus,
            sortition,
            p2p,
        }
    }

    pub async fn attach(
        config: AppConfig,
        pubkey_write_path: Option<&str>,
        plaintext_write_path: Option<&str>,
    ) -> Result<(Addr<Self>, JoinHandle<()>)> {
        let bus = EventBus::new(true).start();
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));

        let store = DataStore::from_in_mem(InMemDataStore::new(true).start());
        let sortition = Sortition::attach(bus.clone(), store.clone());
        let signer = pull_eth_signer_from_env("PRIVATE_KEY").await?;
        for chain in config
            .chains
            .iter()
            .filter(|chain| chain.enabled.unwrap_or(true))
        {
            let rpc_url = &chain.rpc_url;
            EnclaveSol::attach(
                bus.clone(),
                rpc_url,
                &chain.contracts.enclave,
                signer.clone(),
            )
            .await?;
            RegistryFilterSol::attach(
                bus.clone(),
                rpc_url,
                &chain.contracts.filter_registry,
                signer.clone(),
            )
            .await?;
            CiphernodeRegistrySol::attach(
                bus.clone(),
                rpc_url,
                &chain.contracts.ciphernode_registry,
            )
            .await?;
        }

        let e3_manager = E3RequestRouter::builder(bus.clone(), store)
            .add_feature(FheFeature::create(rng))
            .add_feature(PublicKeyAggregatorFeature::create(
                bus.clone(),
                sortition.clone(),
            ))
            .add_feature(PlaintextAggregatorFeature::create(
                bus.clone(),
                sortition.clone(),
            ))
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

        let main_addr = MainAggregator::new(bus, sortition, p2p_addr, e3_manager).start();
        Ok((main_addr, join_handle))
    }
}

impl Actor for MainAggregator {
    type Context = Context<Self>;
}
