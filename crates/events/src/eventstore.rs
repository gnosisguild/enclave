// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{EventStored, StoreEventRequested},
    EventLog, GetEventsAfter, ReceiveEvents, SequenceIndex,
};
use actix::{Actor, Handler};
use anyhow::{bail, Result};
use tracing::error;

pub struct EventStore<I: SequenceIndex, L: EventLog> {
    index: I,
    log: L,
}

impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
    pub fn handle_store_event_requested(&mut self, msg: StoreEventRequested) -> Result<()> {
        let event = msg.event;
        let sender = msg.sender;
        let ts = event.get_ts();
        if let Some(_) = self.index.get(ts)? {
            bail!("Event already stored at timestamp {ts}!");
        }
        let seq = self.log.append(&event)?;
        self.index.insert(ts, seq)?;
        sender.try_send(EventStored(event.into_sequenced(seq)))?;
        Ok(())
    }

    pub fn handle_get_events_after(&mut self, msg: GetEventsAfter) -> Result<()> {
        let seq = self.index.seek(msg.ts)?.unwrap_or(1);
        let evts = self
            .log
            .read_from(seq)
            .map(|(s, e)| e.into_sequenced(s))
            .collect::<Vec<_>>();
        msg.sender.try_send(ReceiveEvents::new(evts))?;
        Ok(())
    }
}
impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
    pub fn new(index: I, log: L) -> Self {
        Self { index, log }
    }
}

impl<I: SequenceIndex, L: EventLog> Actor for EventStore<I, L> {
    type Context = actix::Context<Self>;
}

impl<I: SequenceIndex, L: EventLog> Handler<StoreEventRequested> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        match self.handle_store_event_requested(msg) {
            Ok(_) => (),
            Err(e) => error!("{e}"),
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<GetEventsAfter> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: GetEventsAfter, _: &mut Self::Context) -> Self::Result {
        match self.handle_get_events_after(msg) {
            Ok(_) => (),
            Err(e) => error!("{e}"),
        }
    }
}
