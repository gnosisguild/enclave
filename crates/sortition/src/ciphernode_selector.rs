// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::sortition::{GetNodeIndex, Sortition};
use actix::prelude::*;
use anyhow::bail;
use anyhow::Result;
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::E3RequestComplete;
use e3_events::{
    prelude::*, trap, BusHandle, CiphernodeSelected, CommitteeFinalized, E3Requested, E3id, EType,
    EnclaveEvent, EnclaveEventData, EventType, Shutdown, TicketGenerated, TicketId,
};
use e3_request::E3Meta;
use e3_utils::NotifySync;
use std::collections::HashMap;
use tracing::info;

/// CiphernodeSelector is an actor that determines if a ciphernode is part of a committee and if so
/// emits a TicketGenerated event (score sortition) to the event bus
pub struct CiphernodeSelector {
    bus: BusHandle,
    sortition: Addr<Sortition>,
    address: String,
    e3_cache: Persistable<HashMap<E3id, E3Meta>>,
}

impl Actor for CiphernodeSelector {
    type Context = Context<Self>;
}

impl CiphernodeSelector {
    pub fn new(
        bus: &BusHandle,
        sortition: &Addr<Sortition>,
        e3_cache: Persistable<HashMap<E3id, E3Meta>>,
        address: &str,
    ) -> Self {
        Self {
            bus: bus.clone(),
            sortition: sortition.clone(),
            e3_cache,
            address: address.to_owned(),
        }
    }

    pub async fn attach(
        bus: &BusHandle,
        sortition: &Addr<Sortition>,
        selector_store: Repository<HashMap<E3id, E3Meta>>,
        address: &str,
    ) -> Result<Addr<Self>> {
        let e3_cache = selector_store.load_or_default(HashMap::new()).await?;
        let addr = CiphernodeSelector::new(bus, sortition, e3_cache, address).start();

        bus.subscribe(EventType::E3Requested, addr.clone().recipient());
        bus.subscribe(EventType::CommitteeFinalized, addr.clone().recipient());
        bus.subscribe(EventType::Shutdown, addr.clone().recipient());

        info!("CiphernodeSelector listening!");
        Ok(addr)
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::E3Requested(data) => ctx.notify(data),
            EnclaveEventData::E3RequestComplete(data) => self.notify_sync(ctx, data),
            EnclaveEventData::CommitteeFinalized(data) => self.notify_sync(ctx, data),
            EnclaveEventData::Shutdown(data) => self.notify_sync(ctx, data),
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

        trap(EType::Sortition, &bus.clone(), || {
            self.e3_cache.try_mutate(|mut cache| {
                info!(
                    "Mutating e3_cache: appending data: {:?}",
                    data.e3_id.clone()
                );
                cache.insert(
                    data.e3_id.clone(),
                    E3Meta {
                        seed: data.seed,
                        threshold_n: data.threshold_n,
                        threshold_m: data.threshold_m,
                        params: data.params,
                        esi_per_ct: data.esi_per_ct,
                        error_size: data.error_size,
                    },
                );
                Ok(cache)
            })
        });

        Box::pin(async move {
            let seed = data.seed;
            let size = data.threshold_n;
            info!(
                "Calling GetNodeIndex address={} seed={} size={}",
                address.clone(),
                seed,
                size
            );
            // TODO: instead of this it would be better to pass the event theough sortition and
            // then decorate it with this information WithIndex<E3Requested>
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
                    trap(EType::Sortition, &bus.clone(), || {
                        bus.publish(TicketGenerated {
                            e3_id: data.e3_id.clone(),
                            ticket_id: TicketId::Score(tid),
                            node: address.clone(),
                        })?;
                        Ok(())
                    })
                }
            } else {
                info!("This node is not selected");
            }
        })
    }
}

impl Handler<E3RequestComplete> for CiphernodeSelector {
    type Result = ();
    fn handle(&mut self, msg: E3RequestComplete, _: &mut Self::Context) -> Self::Result {
        trap(EType::Sortition, &self.bus.clone(), move || {
            self.e3_cache.try_mutate(|mut cache| {
                cache.remove(&msg.e3_id);
                Ok(cache)
            })
        })
    }
}

impl Handler<CommitteeFinalized> for CiphernodeSelector {
    type Result = ();

    fn handle(&mut self, msg: CommitteeFinalized, _ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Sortition, &self.bus.clone(), move || {
            info!("CiphernodeSelector received CommitteeFinalized.");
            let bus = self.bus.clone();
            info!("Getting e3_cache...");
            let Some(e3_cache) = self.e3_cache.get() else {
                bail!("Could not get cache");
            };

            info!("Getting e3_meta...");
            let Some(e3_meta) = e3_cache.get(&msg.e3_id) else {
                bail!(
                    "Could not find E3Meta on CiphernodeSelector for {}",
                    msg.e3_id
                );
            };

            // Check if this node is in the finalized committee
            if !msg.committee.contains(&self.address) {
                info!(node = self.address, "Node not in finalized committee");
                return Ok(());
            }

            // Retrieve E3 metadata from repository
            let Some(party_id) = msg.committee.iter().position(|addr| addr == &self.address) else {
                info!(
                    node = self.address,
                    "Node address not found in committee list (should not happen)"
                );
                return Ok(());
            };

            info!(
                node = self.address,
                party_id = party_id,
                "Node is in finalized committee, emitting CiphernodeSelected"
            );

            bus.publish(CiphernodeSelected {
                party_id: party_id as u64,
                e3_id: msg.e3_id,
                threshold_m: e3_meta.threshold_m,
                threshold_n: e3_meta.threshold_n,
                esi_per_ct: e3_meta.esi_per_ct,
                error_size: e3_meta.error_size.clone(),
                params: e3_meta.params.clone(),
                seed: e3_meta.seed,
            })?;

            Ok(())
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
