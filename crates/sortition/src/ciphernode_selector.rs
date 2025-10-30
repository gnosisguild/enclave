// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::sortition::{GetNodeIndex, Sortition};
/// CiphernodeSelector is an actor that determines if a ciphernode is part of a committee and if so
/// emits a TicketGenerated event (score sortition) to the event bus
use actix::prelude::*;
use e3_config::StoreKeys;
use e3_data::{DataStore, RepositoriesFactory};
use e3_events::{
    CiphernodeSelected, CommitteeFinalized, E3Requested, EnclaveEvent, EventBus, Shutdown,
    Subscribe, TicketGenerated, TicketId,
};
use e3_request::MetaRepositoryFactory;
use tracing::info;

pub struct CiphernodeSelector {
    bus: Addr<EventBus<EnclaveEvent>>,
    sortition: Addr<Sortition>,
    address: String,
    data_store: DataStore,
}

impl Actor for CiphernodeSelector {
    type Context = Context<Self>;
}

impl CiphernodeSelector {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
        sortition: &Addr<Sortition>,
        address: &str,
        data_store: &DataStore,
    ) -> Self {
        Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
            address: address.to_owned(),
            data_store: data_store.clone(),
        }
    }

    pub fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        sortition: &Addr<Sortition>,
        address: &str,
        data_store: &DataStore,
    ) -> Addr<Self> {
        let addr = CiphernodeSelector::new(bus, sortition, address, data_store).start();

        bus.do_send(Subscribe::new("E3Requested", addr.clone().recipient()));
        bus.do_send(Subscribe::new(
            "CommitteeFinalized",
            addr.clone().recipient(),
        ));
        bus.do_send(Subscribe::new("Shutdown", addr.clone().recipient()));

        addr
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::E3Requested { data, .. } => ctx.notify(data),
            EnclaveEvent::CommitteeFinalized { data, .. } => ctx.notify(data),
            EnclaveEvent::Shutdown { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<E3Requested> for CiphernodeSelector {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, data: E3Requested, _ctx: &mut Self::Context) -> Self::Result {
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
            if let Ok(found_result) = sortition
                .send(GetNodeIndex {
                    chain_id,
                    seed,
                    address: address.clone(),
                    size,
                })
                .await
            {
                let Some((_party_id, ticket_id)) = found_result else {
                    info!(node = address, "Ciphernode was not selected");
                    return;
                };

                if let Some(tid) = ticket_id {
                    info!(
                        node = address,
                        ticket_id = tid,
                        "Ticket generated for score sortition"
                    );
                    bus.do_send(EnclaveEvent::from(TicketGenerated {
                        e3_id: data.e3_id.clone(),
                        ticket_id: TicketId::Score(tid),
                        node: address.clone(),
                    }));
                }
            } else {
                info!("This node is not selected");
            }
        })
    }
}

impl Handler<CommitteeFinalized> for CiphernodeSelector {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: CommitteeFinalized, _ctx: &mut Self::Context) -> Self::Result {
        let address = self.address.clone();
        let bus = self.bus.clone();
        let e3_id = msg.e3_id.clone();
        let repositories = self
            .data_store
            .scope(StoreKeys::router())
            .scope(StoreKeys::context(&e3_id))
            .repositories();

        // Check if this node is in the finalized committee
        if !msg.committee.contains(&address) {
            info!(node = address, "Node not in finalized committee");
            return Box::pin(async {});
        }

        Box::pin(async move {
            // Retrieve E3 metadata from repository
            let meta_repo = repositories.meta(&e3_id);
            let Some(e3_meta) = meta_repo.read().await.ok().flatten() else {
                info!(
                    node = address,
                    "No stored E3 metadata for {:?}, skipping", e3_id
                );
                return;
            };

            let Some(party_id) = msg.committee.iter().position(|addr| addr == &address) else {
                info!(
                    node = address,
                    "Node address not found in committee list (should not happen)"
                );
                return;
            };

            info!(
                node = address,
                party_id = party_id,
                "Node is in finalized committee, emitting CiphernodeSelected"
            );
            bus.do_send(EnclaveEvent::from(CiphernodeSelected {
                party_id: party_id as u64,
                e3_id,
                threshold_m: e3_meta.threshold_m,
                threshold_n: e3_meta.threshold_n,
                esi_per_ct: e3_meta.esi_per_ct,
                error_size: e3_meta.error_size,
                params: e3_meta.params,
                seed: e3_meta.seed,
            }));
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
