use std::sync::{Arc, Mutex};

use crate::{CiphernodeRegistry, CiphernodeSelector, Data, EventBus, P2p, Registry, Sortition};
use actix::{Actor, Addr, Context};
use alloy_primitives::Address;
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
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
    registry: Addr<Registry>,
    p2p: Addr<P2p>,
}

impl MainCiphernode {
    pub fn new(
        addr: Address,
        bus: Addr<EventBus>,
        data: Addr<Data>,
        sortition: Addr<Sortition>,
        selector: Addr<CiphernodeSelector>,
        registry: Addr<Registry>,
        p2p: Addr<P2p>,
    ) -> Self {
        Self {
            addr,
            bus,
            data,
            sortition,
            selector,
            registry,
            p2p,
        }
    }

    pub async fn attach(address: Address) -> (Addr<Self>, JoinHandle<()>) {
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));
        let bus = EventBus::new(true).start();
        let data = Data::new(true).start(); // TODO: Use a sled backed Data Actor
        let sortition = Sortition::attach(bus.clone());
        let selector = CiphernodeSelector::attach(bus.clone(), sortition.clone(), address);
        let registry =  Registry::attach(
            bus.clone(),
            rng,
            None,
            None,
            Some(CiphernodeRegistry::attach(bus.clone(), data.clone(), address)),
        )
        .await;
        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");
        let main_addr =
            MainCiphernode::new(address, bus, data, sortition, selector, registry, p2p_addr)
                .start();
        (main_addr, join_handle)
    }
}

impl Actor for MainCiphernode {
    type Context = Context<Self>;
}
