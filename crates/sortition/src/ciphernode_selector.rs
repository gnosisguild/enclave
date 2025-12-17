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
    EnclaveEvent, EnclaveEventData, Shutdown, TicketGenerated, TicketId,
};
use e3_request::E3Meta;
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
    /// Constructs a new CiphernodeSelector configured with the provided bus handle,
    /// sortition actor address, persistent E3 metadata cache, and this node's address.
    ///
    /// `e3_cache` is moved into the selector and used to persist E3 metadata across requests.
    /// `address` is the ciphernode's own address used for selection and committee membership checks.
    ///
    /// # Returns
    ///
    /// A configured `CiphernodeSelector` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// // assume `bus`, `sortition_addr`, `cache`, and `addr` are available in scope
    /// let selector = CiphernodeSelector::new(&bus, &sortition_addr, cache, "0xabc");
    /// ```
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

    /// Creates, initializes, and starts a CiphernodeSelector actor, subscribing it to relevant bus events.
    ///
    /// This loads or initializes the persistent E3 metadata cache from `selector_store`, constructs and starts
    /// the actor, subscribes the actor to "E3Requested", "CommitteeFinalized", and "Shutdown" events on the bus,
    /// and returns the actor address.
    ///
    /// # Returns
    ///
    /// The started actor's address wrapped in `Result::Ok` on success, or an error from loading the cache.
    ///
    /// # Examples
    ///
    /// ```
    /// # use actix::Addr;
    /// # use std::collections::HashMap;
    /// # use my_crate::{CiphernodeSelector, BusHandle, Sortition, E3Meta, E3id, Repository};
    /// # async fn example(bus: &BusHandle, sortition: &Addr<Sortition>, store: Repository<HashMap<E3id, E3Meta>>, addr: &str) -> anyhow::Result<Addr<CiphernodeSelector>> {
    /// let selector_addr = CiphernodeSelector::attach(bus, sortition, store, addr).await?;
    /// Ok(selector_addr)
    /// # }
    /// ```
    pub async fn attach(
        bus: &BusHandle,
        sortition: &Addr<Sortition>,
        selector_store: Repository<HashMap<E3id, E3Meta>>,
        address: &str,
    ) -> Result<Addr<Self>> {
        let e3_cache = selector_store.load_or_default(HashMap::new()).await?;
        let addr = CiphernodeSelector::new(bus, sortition, e3_cache, address).start();

        bus.subscribe("E3Requested", addr.clone().recipient());
        bus.subscribe("CommitteeFinalized", addr.clone().recipient());
        bus.subscribe("Shutdown", addr.clone().recipient());

        info!("CiphernodeSelector listening!");
        Ok(addr)
    }
}

impl Handler<EnclaveEvent> for CiphernodeSelector {
    type Result = ();
    /// Forwards an incoming `EnclaveEvent`'s inner data to the actor context as a notification.
    ///
    /// This method extracts the `EnclaveEventData` from the provided `EnclaveEvent` and notifies the
    /// actor context with the contained message for these variants: `E3Requested`, `E3RequestComplete`,
    /// `CommitteeFinalized`, and `Shutdown`. Other event variants are ignored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Pseudocode illustrating intended use:
    /// // let event: EnclaveEvent = ...;
    /// // let mut selector: CiphernodeSelector = ...;
    /// // let mut ctx: Context<CiphernodeSelector> = ...;
    /// // selector.handle(event, &mut ctx);
    /// // The context will receive a notification for E3Requested, E3RequestComplete,
    /// // CommitteeFinalized, or Shutdown variants.
    /// ```
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::E3Requested(data) => ctx.notify(data),
            EnclaveEventData::E3RequestComplete(data) => ctx.notify(data),
            EnclaveEventData::CommitteeFinalized(data) => ctx.notify(data),
            EnclaveEventData::Shutdown(data) => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<E3Requested> for CiphernodeSelector {
    type Result = ResponseFuture<()>;

    /// Handles an incoming E3Requested event: caches E3 metadata, queries the Sortition
    /// actor for a node index for the given seed and size, and publishes a `TicketGenerated`
    /// event if this node is selected.
    ///
    /// The handler first inserts an `E3Meta` entry for the provided `e3_id` into the
    /// persistent `e3_cache`. It then requests a node index from the `Sortition` actor
    /// using the event seed and threshold, and if a ticket is returned emits a
    /// `TicketGenerated` event on the bus with the `e3_id`, ticket id, and node address.
    ///
    /// # Examples
    ///
    /// ```
    /// # use futures::executor::block_on;
    /// # // `selector` is a mutable CiphernodeSelector, `ctx` is its context, and
    /// # // `req` is an E3Requested value prepared for the test.
    /// # let mut selector = /* ... */ panic!();
    /// # let mut ctx = /* ... */ panic!();
    /// # let req = /* ... */ panic!();
    /// // Call the handler and wait for it to complete.
    /// let fut = selector.handle(req, &mut ctx);
    /// block_on(fut);
    /// ```
    ///
    /// # Returns
    ///
    /// `()` on completion.
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
    /// Remove cached metadata for the completed E3 request and report any sortition errors to the bus.
    ///
    /// This notifies the sortition error trap while attempting to remove `msg.e3_id` from the persistent
    /// E3 metadata cache; failures during the mutation are reported via the bus.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given a mutable CiphernodeSelector `sel`, remove the completed request's cache entry:
    /// // sel.handle(E3RequestComplete { e3_id: my_e3_id }, &mut sel_context);
    /// ```
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

    /// Handles a finalized committee event by checking local membership and, if a member, publishing a `CiphernodeSelected` event with cached E3 metadata.
    ///
    /// Retrieves E3 metadata for the finalized E3 id from the persistent cache, verifies whether this node's address is present in the finalized committee, and if so publishes a `CiphernodeSelected` event containing the party id and E3 metadata (thresholds, parameters, seed, etc.). If the cache is unavailable or the E3 metadata is missing, the handler reports the condition through the sortition trap and exits without publishing.
    ///
    /// # Parameters
    ///
    /// - `msg`: the `CommitteeFinalized` message containing the finalized committee list and the `e3_id`.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success; any reported error is propagated through the sortition trap wrapper.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Sketch: when a CommitteeFinalized message arrives, the actor will check membership
    /// // and publish CiphernodeSelected if applicable.
    /// let msg = CommitteeFinalized { e3_id: some_e3_id, committee: vec![my_addr.clone(), /* ... */] };
    /// // The actual invocation occurs inside Actix actor runtime as part of Handler<CommitteeFinalized>.
    /// ```
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