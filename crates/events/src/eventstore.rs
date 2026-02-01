// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{EventStored, StoreEventRequested},
    EventContextAccessors, EventLog, GetEventsAfter, ReceiveEvents, SequenceIndex,
};
use actix::{Actor, Handler};
use anyhow::{bail, Result};
use e3_utils::major_issue;
use tracing::{error, warn};

const MAX_STORAGE_ERRORS: u64 = 10;

pub struct EventStore<I: SequenceIndex, L: EventLog> {
    index: I,
    log: L,
    storage_errors: u64,
}

impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
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
        sender.try_send(EventStored(event.into_sequenced(seq)))?;
        Ok(())
    }

    pub fn handle_get_events_after(&mut self, msg: GetEventsAfter) -> Result<()> {
        // if there are no events after the timestamp return an empty vector
        let Some(seq) = self.index.seek(msg.ts())? else {
            msg.sender()
                .try_send(ReceiveEvents::new(msg.id(), vec![]))?;
            return Ok(());
        };
        // read and return the events
        let evts = self
            .log
            .read_from(seq)
            .map(|(s, e)| e.into_sequenced(s))
            .collect::<Vec<_>>();

        msg.sender().try_send(ReceiveEvents::new(msg.id(), evts))?;
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

impl<I: SequenceIndex, L: EventLog> Actor for EventStore<I, L> {
    type Context = actix::Context<Self>;
}

impl<I: SequenceIndex, L: EventLog> Handler<StoreEventRequested> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_store_event_requested(msg) {
            panic!("{}", major_issue("Could not store event in eventstore.", e))
            // panic here because when event storage fails we really need
            // to just give up
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<GetEventsAfter> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: GetEventsAfter, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_get_events_after(msg) {
            error!("{e}");
        }
    }
}
