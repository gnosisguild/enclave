use actix::{Actor, Addr, Context};
use alloy::primitives::Address;
use anyhow::Result;
use enclave_core::EventBus;
use evm::{CiphernodeRegistrySol, EnclaveSol, RegistryFilterSol};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use router::{E3RequestRouter, LazyFhe, LazyPlaintextAggregator, LazyPublicKeyAggregator};
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
        rpc_url: &str,
        enclave_contract: Address,
        registry_contract: Address,
        registry_filter_contract: Address,
        pubkey_write_path: Option<&str>,
        plaintext_write_path: Option<&str>,
    ) -> Result<(Addr<Self>, JoinHandle<()>)> {
        let bus = EventBus::new(true).start();
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));

        let sortition = Sortition::attach(bus.clone());
        EnclaveSol::attach(bus.clone(), rpc_url, enclave_contract).await?;
        RegistryFilterSol::attach(bus.clone(), rpc_url, registry_filter_contract).await?;
        CiphernodeRegistrySol::attach(bus.clone(), rpc_url, registry_contract).await?;

        let e3_manager = E3RequestRouter::builder(bus.clone())
            .add_hook(LazyFhe::create(rng))
            .add_hook(LazyPublicKeyAggregator::create(
                bus.clone(),
                sortition.clone(),
            ))
            .add_hook(LazyPlaintextAggregator::create(
                bus.clone(),
                sortition.clone(),
            ))
            .build();

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
