// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{StoreEventRequested, StoreEventResponse},
    EnclaveEvent, EventContextAccessors, EventLog, EventStoreFilter, EventStoreQueryBy,
    EventStoreQueryResponse, Seq, SequenceIndex, Sequenced, Ts, Unsequenced,
};
use actix::{Actor, Handler};
use anyhow::{bail, Result};
use tracing::{error, warn};

const MAX_STORAGE_ERRORS: u64 = 10;

pub struct EventStore<I: SequenceIndex, L: EventLog<EnclaveEvent<Unsequenced>>> {
    index: I,
    log: L,
    storage_errors: u64,
}

impl<I: SequenceIndex, L: EventLog<EnclaveEvent<Unsequenced>>> EventStore<I, L> {
    pub fn handle_store_event_requested(&mut self, msg: StoreEventRequested) -> Result<()> {
        let event = msg.event;
        let sender = msg.sender;
        let ts = event.ts();
        if let Some(_) = self.index.get(ts)? {
            warn!("Event already stored at timestamp {ts}! This might happen when recovering from a snapshot. Skipping storage");
            self.storage_errors += 1;
            if self.storage_errors > MAX_STORAGE_ERRORS {
                bail!(
                    "The eventstore had too many storage errors! {}",
                    self.storage_errors
                );
            }
            return Ok(());
        }
        let seq = self.log.append(&event)?;
        self.index.insert(ts, seq)?;
        sender.do_send(StoreEventResponse(event.into_sequenced(seq)));
        Ok(())
    }

    fn query_events(
        &self,
        iter: Box<dyn Iterator<Item = (u64, EnclaveEvent<Unsequenced>)>>,
        filter: Option<EventStoreFilter>,
        limit: Option<u64>,
    ) -> Vec<EnclaveEvent<Sequenced>> {
        let iter = iter.map(|(s, e)| e.into_sequenced(s));

        match filter {
            Some(EventStoreFilter::Source(source)) => {
                let iter = iter.filter(move |e| e.get_ctx().source() == source);
                match limit {
                    Some(lim) => iter.take(lim as usize).collect(),
                    None => iter.collect(),
                }
            }
            None => match limit {
                Some(lim) => iter.take(lim as usize).collect(),
                None => iter.collect(),
            },
        }
    }

    pub fn handle_event_store_query_ts(&mut self, msg: EventStoreQueryBy<Ts>) -> Result<()> {
        let id = msg.id();
        let query = msg.query();
        let filter = msg.filter().cloned();
        let limit = msg.limit();
        let sender = msg.sender();

        let Some(seq) = self.index.seek(query)? else {
            sender.try_send(EventStoreQueryResponse::new(id, vec![]))?;
            return Ok(());
        };

        let evts = self.query_events(self.log.read_from(seq), filter, limit);

        sender.try_send(EventStoreQueryResponse::new(id, evts))?;
        Ok(())
    }

    pub fn handle_event_store_query_seq(&mut self, msg: EventStoreQueryBy<Seq>) -> Result<()> {
        let id = msg.id();
        let query = msg.query();
        let filter = msg.filter().cloned();
        let limit = msg.limit();
        let sender = msg.sender();

        let evts = self.query_events(self.log.read_from(query), filter, limit);

        sender.try_send(EventStoreQueryResponse::new(id, evts))?;
        Ok(())
    }
}

impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
    pub fn new(index: I, log: L) -> Self {
        Self {
            index,
            log,
            storage_errors: 0,
        }
    }
}

impl<I: SequenceIndex, L: EventLog<EnclaveEvent<Unsequenced>>> Actor for EventStore<I, L> {
    type Context = actix::Context<Self>;
}

impl<I: SequenceIndex, L: EventLog<EnclaveEvent<Unsequenced>>> Handler<StoreEventRequested>
    for EventStore<I, L>
{
    type Result = ();
    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_store_event_requested(msg) {
            // Log append or index insert failed — storage is broken.
            error!("Event storage failed: {e}");
            panic!("Unrecoverable event storage failure: {e}");
        }
    }
}

impl<I: SequenceIndex, L: EventLog<EnclaveEvent<Unsequenced>>> Handler<EventStoreQueryBy<Ts>>
    for EventStore<I, L>
{
    type Result = ();
    fn handle(&mut self, msg: EventStoreQueryBy<Ts>, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_event_store_query_ts(msg) {
            error!("{e}");
        }
    }
}

impl<I: SequenceIndex, L: EventLog<EnclaveEvent<Unsequenced>>> Handler<EventStoreQueryBy<Seq>>
    for EventStore<I, L>
{
    type Result = ();
    fn handle(&mut self, msg: EventStoreQueryBy<Seq>, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_event_store_query_seq(msg) {
            error!("{e}");
        }
    }
}
