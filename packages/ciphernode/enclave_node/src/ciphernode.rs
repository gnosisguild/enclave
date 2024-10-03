use actix::{Actor, Addr, Context};
use alloy::primitives::Address;
use data::Data;
use enclave_core::EventBus;
use evm::{connect_evm_ciphernode_registry, connect_evm_enclave};
use logger::SimpleLogger;
use p2p::P2p;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use router::{CiphernodeSelector, CommitteeMetaFactory, E3RequestRouter, LazyFhe, LazyKeyshare};
use sortition::Sortition;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

/// Main Ciphernode Actor
/// Suprvises all children
// TODO: add supervision logic
pub struct MainCiphernode {
    addr: Address,
    bus: Addr<EventBus>,
    data: Addr<Data>,
    sortition: Addr<Sortition>,
    selector: Addr<CiphernodeSelector>,
    e3_manager: Addr<E3RequestRouter>,
    p2p: Addr<P2p>,
}

impl MainCiphernode {
    pub fn new(
        addr: Address,
        bus: Addr<EventBus>,
        data: Addr<Data>,
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
        address: Address,
        rpc_url: &str,
        enclave_contract: Address,
        registry_contract: Address,
    ) -> (Addr<Self>, JoinHandle<()>) {
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));
        let bus = EventBus::new(true).start();
        let data = Data::new(true).start(); // TODO: Use a sled backed Data Actor
        let sortition = Sortition::attach(bus.clone());
        let selector =
            CiphernodeSelector::attach(bus.clone(), sortition.clone(), &address.to_string());

        connect_evm_enclave(bus.clone(), rpc_url, enclave_contract).await;
        let _ = connect_evm_ciphernode_registry(bus.clone(), rpc_url, registry_contract).await;

        let e3_manager = E3RequestRouter::builder(bus.clone())
            .add_hook(LazyFhe::create(rng))
            .add_hook(LazyKeyshare::create(
                bus.clone(),
                data.clone(),
                &address.to_string(),
            ))
            .build();

        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

        let nm = format!("CIPHER({})", &address.to_string()[0..5]);
        SimpleLogger::attach(&nm, bus.clone());
        let main_addr = MainCiphernode::new(
            address, bus, data, sortition, selector, p2p_addr, e3_manager,
        )
        .start();
        (main_addr, join_handle)
    }
}

impl Actor for MainCiphernode {
    type Context = Context<Self>;
}
