use std::sync::{Arc, Mutex};

use crate::{
    CiphernodeFactory, CiphernodeSelector, CommitteeMetaFactory, Data, E3RequestManager, EventBus,
    FheFactory, P2p, PlaintextAggregatorFactory, PublicKeyAggregatorFactory, SimpleLogger,
    Sortition,
};
use actix::{Actor, Addr, Context};
//use alloy_primitives::Address;
use alloy::{primitives::{Address, address}};
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
    e3_manager: Addr<E3RequestManager>,
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
        e3_manager: Addr<E3RequestManager>,
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
        // rpc_url: String,
        // contract_address: Address,
    ) -> (Addr<Self>, JoinHandle<()>) {
        let rng = Arc::new(Mutex::new(
            rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
        ));
        let bus = EventBus::new(true).start();
        let data = Data::new(true).start(); // TODO: Use a sled backed Data Actor
        let sortition = Sortition::attach(bus.clone());
        let selector = CiphernodeSelector::attach(bus.clone(), sortition.clone(), address);

        let e3_manager = E3RequestManager::builder(bus.clone())
            .add_hook(CommitteeMetaFactory::create())
            .add_hook(FheFactory::create(rng.clone()))
            .add_hook(CiphernodeFactory::create(
                bus.clone(),
                data.clone(),
                address,
            ))
            .build();

        let (p2p_addr, join_handle) =
            P2p::spawn_libp2p(bus.clone()).expect("Failed to setup libp2p");

        SimpleLogger::attach("CIPHERNODE", bus.clone());
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
