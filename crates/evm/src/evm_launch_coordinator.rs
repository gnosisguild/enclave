// use crate::evm_router::EvmRouter;
// use crate::helpers::EthProvider;
// use crate::EvmReadInterface;
// use crate::{events::EvmEventProcessor, evm_read_interface::Filters};
// use actix::{Actor, ActorContext, Addr, AsyncContext, Handler};
// use alloy::providers::Provider;
// use alloy_primitives::Address;
// use anyhow::Context;
// use anyhow::Result;
// use e3_events::{
//     trap, BusHandle, EType, EnclaveEvent, EnclaveEventData, Event, EventSubscriber, SyncStart,
// };
// use std::collections::HashMap;
//
// // Configured with Addr for
// pub struct EvmLaunchCoordinator<P> {
//     provider: Option<EthProvider<P>>,
//     routing_table: HashMap<Address, EvmEventProcessor>,
//     bus: BusHandle,
// }
//
// impl<P> EvmLaunchCoordinator<P>
// where
//     P: Provider + Clone + 'static,
// {
//     pub fn builder(bus: &BusHandle, provider: &EthProvider<P>) -> EvmLaunchCoordinatorBuilder<P> {
//         EvmLaunchCoordinatorBuilder {
//             routing_table: HashMap::new(),
//             bus: bus.clone(),
//             provider: provider.clone(),
//         }
//     }
//
//     fn filters(&self, start_block: Option<u64>) -> Filters {
//         let addresses = self.routing_table.keys().cloned().collect();
//         Filters::new(addresses, start_block)
//     }
//
//     fn bootstrap_reader(&mut self, _event: SyncStart) -> Result<()> {
//         // Setup upstream router
//         // The routing table holds addresses for upstream processors
//         let next = EvmRouter::setup(self.routing_table.clone());
//
//         // Setup read interface
//         EvmReadInterface::attach(
//             self.provider.take().context("Cannot call setup twice!")?,
//             &next.into(),
//             &self.bus,
//             self.filters(None),
//         );
//
//         Ok(())
//     }
// }
//
// impl<P> Actor for EvmLaunchCoordinator<P>
// where
//     P: Provider + Clone + 'static,
// {
//     type Context = actix::Context<Self>;
// }
//
// impl<P> Handler<EnclaveEvent> for EvmLaunchCoordinator<P>
// where
//     P: Provider + Clone + 'static,
// {
//     type Result = ();
//     fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
//         trap(EType::Evm, &self.bus.clone(), || {
//             if let EnclaveEventData::SyncStart(event) = msg.into_data() {
//                 // Run the setup process
//                 self.bootstrap_reader(event)?;
//
//                 // We now don't need the launcher and can kill it now
//                 self.bus.unsubscribe("SyncStart", ctx.address().into());
//                 ctx.stop();
//             }
//             Ok(())
//         })
//     }
// }
//
// pub struct EvmLaunchCoordinatorBuilder<P> {
//     provider: EthProvider<P>,
//     routing_table: HashMap<Address, EvmEventProcessor>,
//     bus: BusHandle,
// }
//
// impl<P> EvmLaunchCoordinatorBuilder<P>
// where
//     P: Provider + Clone + 'static,
// {
//     pub fn with_contract(
//         &mut self,
//         address: impl AsRef<str>,
//         dest: impl Into<EvmEventProcessor>,
//     ) -> Result<()> {
//         let address: Address = address.as_ref().parse().context("invalid address")?;
//         self.routing_table.insert(address, dest.into());
//         Ok(())
//     }
//
//     pub fn build(self) -> Addr<EvmLaunchCoordinator<P>> {
//         let routing_table = self.routing_table;
//         let addr = EvmLaunchCoordinator {
//             routing_table,
//             provider: Some(self.provider),
//             bus: self.bus.clone(),
//         }
//         .start();
//
//         self.bus.subscribe("SyncStart", addr.clone().recipient());
//
//         addr
//     }
// }
