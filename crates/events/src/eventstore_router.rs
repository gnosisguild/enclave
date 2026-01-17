// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::eventstore::EventStore;
use crate::ReceiveEvents;
use crate::{
    events::{GetEventsAfter, StoreEventRequested},
    AggregateId, EventContextAccessors, EventLog, SequenceIndex,
};
use actix::{Actor, Addr, Handler, Message, Recipient};
use anyhow::Result;
use std::collections::HashMap;
use tracing::error;

pub struct EventStoreRouter<I: SequenceIndex, L: EventLog> {
    stores: HashMap<AggregateId, actix::Addr<EventStore<I, L>>>,
}

impl<I: SequenceIndex, L: EventLog> EventStoreRouter<I, L> {
    pub fn new(stores: HashMap<usize, Addr<EventStore<I, L>>>) -> Self {
        let stores = stores
            .into_iter()
            .map(|(index, addr)| (AggregateId::new(index), addr))
            .collect();
        Self { stores }
    }

    pub fn handle_store_event_requested(&mut self, msg: StoreEventRequested) -> Result<()> {
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

    pub fn handle_get_events_after(&mut self, msg: GetAggregateEventsAfter) -> Result<()> {
        for (aggregate_id, ts) in msg.ts {
            if let Some(store_addr) = self.stores.get(&aggregate_id) {
                let get_events_msg = GetEventsAfter::new(ts, msg.sender.clone());
                store_addr.do_send(get_events_msg);
            }
        }
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
            error!("Failed to route store event request: {}", e);
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<GetAggregateEventsAfter> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: GetAggregateEventsAfter, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_get_events_after(msg) {
            error!("Failed to route get events after request: {}", e);
        }
    }
}

#[derive(Message, Debug)]
#[rtype("()")]
pub struct GetAggregateEventsAfter {
    pub ts: HashMap<AggregateId, u128>,
    pub sender: Recipient<ReceiveEvents>,
}
