use actix::{Actor, Addr, Context};
use alloy::primitives::Address;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::{e3::{CommitteeMetaFactory, E3RequestManager}, enclave_core::EventBus, evm::{connect_evm_caller, connect_evm_ciphernode_registry, connect_evm_enclave}, fhe::FheFactory, logger::SimpleLogger, p2p::P2p, plaintext_aggregator::PlaintextAggregatorFactory, plaintext_writer::PlaintextWriter, public_key_writer::PublicKeyWriter, publickey_aggregator::PublicKeyAggregatorFactory, sortition::Sortition};

/// Main Ciphernode Actor
/// Suprvises all children
// TODO: add supervision logic
pub struct MainAggregator {
    e3_manager: Addr<E3RequestManager>,
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    p2p: Addr<P2p>,
}

impl MainAggregator {
    pub fn new(
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        p2p: Addr<P2p>,
        e3_manager: Addr<E3RequestManager>,
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
    ) -> (Addr<Self>, JoinHandle<()>) {
        let bus = EventBus::new(true).start();
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));

        let sortition = Sortition::attach(bus.clone());

        connect_evm_enclave(bus.clone(), rpc_url, enclave_contract).await;
        let _ = connect_evm_ciphernode_registry(bus.clone(), rpc_url, registry_contract).await;
        let _ = connect_evm_caller(bus.clone(), sortition.clone(), rpc_url, enclave_contract, registry_filter_contract).await;    

        let e3_manager = E3RequestManager::builder(bus.clone())
            .add_hook(CommitteeMetaFactory::create())
            .add_hook(FheFactory::create(rng))
            .add_hook(PublicKeyAggregatorFactory::create(
                bus.clone(),
                sortition.clone(),
            ))
            .add_hook(PlaintextAggregatorFactory::create(
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
        (main_addr, join_handle)
    }
}

impl Actor for MainAggregator {
    type Context = Context<Self>;
}
