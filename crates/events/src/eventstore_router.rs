// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::eventstore::EventStore;
use crate::{
    events::StoreEventRequested, AggregateId, EventContextAccessors, EventLog, SequenceIndex,
};
use crate::{EventStoreQueryBy, Seq, SeqAgg, Ts, TsAgg};
use actix::{Actor, Addr, Handler};
use anyhow::Result;
use e3_utils::{major_issue, MAILBOX_LIMIT};
use std::collections::HashMap;
use tracing::{error, info};

pub struct EventStoreRouter<I: SequenceIndex, L: EventLog> {
    stores: HashMap<AggregateId, Addr<EventStore<I, L>>>,
}

impl<I: SequenceIndex, L: EventLog> EventStoreRouter<I, L> {
    pub fn new(stores: HashMap<usize, Addr<EventStore<I, L>>>) -> Self {
        info!("Making eventstore router...");
        let stores = stores
            .into_iter()
            .map(|(index, addr)| (AggregateId::new(index), addr))
            .collect();

        Self { stores }
    }

    pub fn handle_store_event_requested(&mut self, msg: StoreEventRequested) -> Result<()> {
        info!("Handling store event requested....");
        let aggregate_id = msg.event.aggregate_id();

        let store_addr = self.stores.get(&aggregate_id).unwrap_or_else(|| {
            self.stores
                .get(&AggregateId::new(0))
                .expect("Default EventStore for AggregateId(0) not found")
        });

        let event = msg.event;
        let sender = msg.sender;

        let forwarded_msg = StoreEventRequested::new(event, sender);
        store_addr.try_send(forwarded_msg)?;
        Ok(())
    }

    pub fn handle_event_store_query_ts(&mut self, msg: EventStoreQueryBy<TsAgg>) -> Result<()> {
        let id = msg.id();
        let query = msg.query().clone();
        let sender = msg.sender();
        for (aggregate_id, ts) in query {
            if let Some(store_addr) = self.stores.get(&aggregate_id) {
                let get_events_msg =
                    EventStoreQueryBy::<Ts>::new(id, ts.to_owned(), sender.clone());
                store_addr.do_send(get_events_msg);
            }
        }
        Ok(())
    }

    pub fn handle_event_store_query_seq(&mut self, msg: EventStoreQueryBy<SeqAgg>) -> Result<()> {
        let id = msg.id();
        let query = msg.query().clone();
        let sender = msg.sender();
        for (aggregate_id, ts) in query {
            if let Some(store_addr) = self.stores.get(&aggregate_id) {
                let get_events_msg =
                    EventStoreQueryBy::<Seq>::new(id, ts.to_owned(), sender.clone());
                store_addr.do_send(get_events_msg);
            }
        }
        Ok(())
    }
}

impl<I: SequenceIndex, L: EventLog> Actor for EventStoreRouter<I, L> {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<StoreEventRequested> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_store_event_requested(msg) {
            panic!("{}", major_issue("Could not store event in eventstore.", e))
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<EventStoreQueryBy<TsAgg>> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: EventStoreQueryBy<TsAgg>, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_event_store_query_ts(msg) {
            error!("Failed to route get events after request: {}", e);
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<EventStoreQueryBy<SeqAgg>> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: EventStoreQueryBy<SeqAgg>, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_event_store_query_seq(msg) {
            error!("Failed to route get events after request: {}", e);
        }
    }
}
