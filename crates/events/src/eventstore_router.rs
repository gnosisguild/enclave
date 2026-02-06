// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::eventstore::EventStore;
use crate::{
    events::{GetEventsAfterRequest, StoreEventRequested},
    AggregateId, EventContextAccessors, EventLog, SequenceIndex,
};
use crate::{CorrelationId, GetEventsAfterResponse};
use actix::{Actor, Addr, Handler, Message, Recipient};
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

    pub fn handle_get_events_after(&mut self, msg: GetEventsAfterTs) -> Result<()> {
        for (aggregate_id, ts) in msg.ts() {
            if let Some(store_addr) = self.stores.get(&aggregate_id) {
                let get_events_msg =
                    GetEventsAfterRequest::new(msg.id(), ts.to_owned(), msg.sender.clone());
                store_addr.do_send(get_events_msg);
            }
        }
        Ok(())
    }

    // pub fn handle_get_events_after_seq(&mut self, msg: GetEventsAfterSeq) -> Result<()> {
    //     for (aggregate_id, seq) in msg.seq() {
    //         if let Some(store_addr) = self.stores.get(&aggregate_id) {
    //             let get_events_msg =
    //                 GetEventsAfterRequest::new(msg.id(), seq.to_owned(), msg.sender.clone());
    //             store_addr.do_send(get_events_msg);
    //         }
    //     }
    //     Ok(())
    // }
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

impl<I: SequenceIndex, L: EventLog> Handler<GetEventsAfterTs> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: GetEventsAfterTs, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_get_events_after(msg) {
            error!("Failed to route get events after request: {}", e);
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<GetEventsAfterSeq> for EventStoreRouter<I, L> {
    type Result = ();

    fn handle(&mut self, msg: GetEventsAfterSeq, _: &mut Self::Context) -> Self::Result {
        // if let Err(e) = self.handle_get_events_after_seq(msg) {
        //     error!("Failed to route get events after request: {}", e);
        // }
    }
}

#[derive(Message, Debug)]
#[rtype("()")]
pub struct GetEventsAfterTs {
    correlation_id: CorrelationId,
    ts: HashMap<AggregateId, u128>,
    sender: Recipient<GetEventsAfterResponse>,
}

impl GetEventsAfterTs {
    pub fn new(
        correlation_id: CorrelationId,
        ts: HashMap<AggregateId, u128>,
        sender: Recipient<GetEventsAfterResponse>,
    ) -> Self {
        Self {
            correlation_id,
            ts,
            sender,
        }
    }

    pub fn id(&self) -> CorrelationId {
        self.correlation_id
    }

    pub fn ts(&self) -> &HashMap<AggregateId, u128> {
        &self.ts
    }

    pub fn sender(self) -> Recipient<GetEventsAfterResponse> {
        self.sender
    }
}

/// A request to get all events from all aggregates after a specific set of sequences
#[derive(Message, Debug)]
#[rtype("()")]
pub struct GetEventsAfterSeq {
    correlation_id: CorrelationId,
    seq: HashMap<AggregateId, u64>,
    sender: Recipient<GetEventsAfterResponse>,
}

impl GetEventsAfterSeq {
    pub fn new(
        correlation_id: CorrelationId,
        seq: HashMap<AggregateId, u64>,
        sender: impl Into<Recipient<GetEventsAfterResponse>>,
    ) -> Self {
        Self {
            correlation_id,
            seq,
            sender: sender.into(),
        }
    }

    pub fn id(&self) -> CorrelationId {
        self.correlation_id
    }

    pub fn seq(&self) -> &HashMap<AggregateId, u64> {
        &self.seq
    }

    pub fn sender(self) -> Recipient<GetEventsAfterResponse> {
        self.sender
    }
}
