// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::eventstore::EventStore;
use crate::{
    events::StoreEventRequested, AggregateId, EventContextAccessors, EventLog, SequenceIndex,
};
use actix::{Actor, Handler};
use std::collections::HashMap;

pub struct EventStoreRouter<I: SequenceIndex, L: EventLog> {
    stores: HashMap<AggregateId, actix::Addr<EventStore<I, L>>>,
}

impl<I: SequenceIndex, L: EventLog> EventStoreRouter<I, L> {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    pub fn register_store(
        &mut self,
        aggregate_id: AggregateId,
        store: actix::Addr<EventStore<I, L>>,
    ) {
        self.stores.insert(aggregate_id, store);
    }

    pub fn handle_store_event_requested(
        &mut self,
        msg: StoreEventRequested,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let aggregate_id = msg.event.aggregate_id();

        let store_addr = self.stores.get(&aggregate_id).unwrap_or_else(|| {
            self.stores
                .get(&AggregateId::new(0))
                .expect("Default EventStore for AggregateId(0) not found")
        });

        let event = msg.event;
        let sender = msg.sender;

        let forwarded_msg = StoreEventRequested::new(event, sender);
        store_addr.do_send(forwarded_msg);
        Ok(())
    }
}

impl<I: SequenceIndex, L: EventLog> Actor for EventStoreRouter<I, L> {
    type Context = actix::Context<Self>;
}

impl<I: SequenceIndex, L: EventLog> Handler<StoreEventRequested> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_store_event_requested(msg) {
            tracing::error!("Failed to route store event request: {}", e);
        }
    }
}
