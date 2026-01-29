// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::mem::replace;

use actix::Actor;
use alloy::{primitives::Address, providers::Provider};
use e3_events::{BusHandle, EventSubscriber, EventType, SyncStart};
use e3_evm::{
    EthProvider, EvmChainGateway, EvmEventProcessor, EvmReadInterface, EvmRouter, Filters,
    FixHistoricalOrder, OneShotRunner, SyncStartExtractor,
};

pub trait RouteFn: FnOnce(EvmEventProcessor) -> EvmEventProcessor + Send {}
impl<F> RouteFn for F where F: FnOnce(EvmEventProcessor) -> EvmEventProcessor + Send {}

type RouteFactory = Box<dyn RouteFn>;

// Build the event system for a single chain
pub struct EvmSystemChainBuilder<P> {
    provider: EthProvider<P>,
    bus: BusHandle,
    chain_id: u64,
    route_factories: Vec<(Address, RouteFactory)>,
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

    pub fn with_contract<F: RouteFn + 'static>(
        &mut self,
        address: Address,
        route_fn: F,
    ) -> &mut Self {
        self.route_factories.push((address, Box::new(route_fn)));
        self
    }

    pub fn build(&mut self) {
        let gateway = FixHistoricalOrder::setup(EvmChainGateway::setup(&self.bus));
        let runner = SyncStartExtractor::setup(OneShotRunner::setup({
            let bus = self.bus.clone();
            let provider = self.provider.clone();
            let gateway = gateway.clone();
            let chain_id = self.chain_id;
            // Only gets consumed once so fine to do this
            let route_factories = replace(&mut self.route_factories, Vec::new());
            move |msg: SyncStart| {
                let config = msg.get_evm_config(chain_id)?;
                let gateway = gateway.recipient();
                let mut router = EvmRouter::new();

                for (address, route_fn) in route_factories {
                    let processor = route_fn(gateway.clone());
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
        self.bus.subscribe(EventType::SyncStart, runner.recipient());
    }
}
