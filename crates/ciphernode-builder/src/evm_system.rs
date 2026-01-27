// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use alloy::{primitives::Address, providers::Provider};
use e3_events::{BusHandle, EventSubscriber, SyncStart};
use e3_evm::{
    EthProvider, EvmChainGateway, EvmEventProcessor, EvmReadInterface, EvmRouter, Filters,
    FixHistoricalOrder, OneShotRunner, SyncStartExtractor,
};

pub trait RouteFn: FnOnce(EvmEventProcessor) -> (Address, EvmEventProcessor) + Send {}
impl<F> RouteFn for F where F: FnOnce(EvmEventProcessor) -> (Address, EvmEventProcessor) + Send {}

type RouteFactory = Box<dyn RouteFn>;

// Build the event system for a single chain
pub struct EvmSystemChainBuilder<P> {
    provider: EthProvider<P>,
    bus: BusHandle,
    chain_id: u64,
    route_factories: Vec<RouteFactory>,
}

impl<P: Provider + Clone + 'static> EvmSystemChainBuilder<P> {
    pub fn new(bus: &BusHandle, provider: &EthProvider<P>) -> Self {
        let chain_id = provider.chain_id();
        Self {
            bus: bus.clone(),
            provider: provider.clone(),
            chain_id,
            route_factories: Vec::new(),
        }
    }

    pub fn with_contract<F: RouteFn + 'static>(mut self, route_fn: F) -> Self {
        self.route_factories.push(Box::new(route_fn));
        self
    }

    pub fn build(self) {
        let gateway = FixHistoricalOrder::setup(EvmChainGateway::setup(&self.bus));
        let runner = SyncStartExtractor::setup(OneShotRunner::setup({
            let bus = self.bus.clone();
            let provider = self.provider.clone();
            let gateway = gateway.clone();
            let chain_id = self.chain_id;
            let route_factories = self.route_factories;
            move |msg: SyncStart| {
                let config = msg.get_evm_config(chain_id)?;
                let gateway = gateway.recipient();
                let mut router = EvmRouter::new();

                for route_fn in route_factories {
                    let (address, processor) = route_fn(gateway.clone());
                    router = router.add_route(address, &processor);
                }

                router = router.add_fallback(&gateway);
                let filters =
                    Filters::from_routing_table(router.get_routing_table(), config.deploy_block());
                let router = router.start();
                EvmReadInterface::setup(&provider, &router.recipient(), &bus, filters);
                Ok(())
            }
        }));
        self.bus.subscribe("SyncStart", runner.recipient());
    }
}
