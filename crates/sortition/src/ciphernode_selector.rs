// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{GetNodeIndex, Sortition};
/// CiphernodeSelector is an actor that determines if a ciphernode is part of a committee and if so
/// forwards a CiphernodeSelected event to the event bus
use actix::prelude::*;
use e3_events::{CiphernodeSelected, E3Requested, EnclaveEvent, EventBus, Shutdown, Subscribe};
use tracing::info;

pub struct CiphernodeSelector {
    bus: Addr<EventBus<EnclaveEvent>>,
    sortition: Addr<Sortition>,
    address: String,
}

impl Actor for CiphernodeSelector {
    type Context = Context<Self>;
}

impl CiphernodeSelector {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
        sortition: &Addr<Sortition>,
        address: &str,
    ) -> Self {
        Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
            address: address.to_owned(),
        }
    }

    pub fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        sortition: &Addr<Sortition>,
        address: &str,
    ) -> Addr<Self> {
        let addr = CiphernodeSelector::new(bus, sortition, address).start();

        bus.do_send(Subscribe::new("E3Requested", addr.clone().recipient()));
        bus.do_send(Subscribe::new("Shutdown", addr.clone().recipient()));

        addr
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::E3Requested { data, .. } => ctx.notify(data),
            EnclaveEvent::Shutdown { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<E3Requested> for CiphernodeSelector {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, data: E3Requested, _ctx: &mut Self::Context) -> Self::Result {
        info!("CiphernodeSelector is handling E3Requested!!!");
        let address = self.address.clone();
        let sortition = self.sortition.clone();
        let bus = self.bus.clone();
        let chain_id = data.e3_id.chain_id();

        Box::pin(async move {
            let seed = data.seed;
            let size = data.threshold_n;
            info!(
                "Calling GetNodeIndex address={} seed={} size={}",
                address.clone(),
                seed,
                size
            );
            if let Ok(found_index) = sortition
                .send(GetNodeIndex {
                    chain_id,
                    seed,
                    address: address.clone(),
                    size,
                })
                .await
            {
                let Some(party_id) = found_index else {
                    info!(node = address, "Ciphernode was not selected");
                    return;
                };
                info!("CIPHERNODE SELECTED: node={} address={}", party_id, address);
                bus.do_send(EnclaveEvent::from(CiphernodeSelected {
                    party_id,
                    e3_id: data.e3_id,
                    threshold_m: data.threshold_m,
                    threshold_n: data.threshold_n,
                    esi_per_ct: data.esi_per_ct,
                    error_size: data.error_size,
                    params: data.params.clone(),
                    seed: data.seed.clone(),
                }));
            } else {
                info!("This node is not selected");
            }
        })
    }
}

impl Handler<Shutdown> for CiphernodeSelector {
    type Result = ();
    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        info!("Killing CiphernodeSelector");
        ctx.stop();
    }
}
