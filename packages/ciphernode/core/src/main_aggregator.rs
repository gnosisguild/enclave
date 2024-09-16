use std::sync::{Arc, Mutex};

use crate::{EventBus, P2p, PlaintextRegistry, PublicKeyRegistry, Registry, Sortition};
use actix::{Actor, Addr, Context};
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use tokio::task::JoinHandle;

/// Main Ciphernode Actor
/// Suprvises all children
// TODO: add supervision logic
pub struct MainAggregator {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    registry: Addr<Registry>,
    p2p: Addr<P2p>,
}

impl MainAggregator {
    pub fn new(
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        registry: Addr<Registry>,
        p2p: Addr<P2p>,
    ) -> Self {
        Self {
            bus,
            sortition,
            registry,
            p2p,
        }
    }

    pub async fn attach() -> (Addr<Self>, JoinHandle<()>) {
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));
        let bus = EventBus::new(true).start();
        let sortition = Sortition::attach(bus.clone());
        let registry = Registry::attach(
            bus.clone(),
            rng,
            Some(PublicKeyRegistry::attach(bus.clone(), sortition.clone())),
            Some(PlaintextRegistry::attach(bus.clone(), sortition.clone())),
            None,
        )
        .await;
        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");
        let main_addr = MainAggregator::new(bus, sortition, registry, p2p_addr).start();
        (main_addr, join_handle)
    }
}

impl Actor for MainAggregator {
    type Context = Context<Self>;
}
