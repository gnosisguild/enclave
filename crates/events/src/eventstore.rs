// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{EventStored, StoreEventRequested},
    EventLog, SequenceIndex,
};
use actix::{Actor, Handler};
use anyhow::Result;
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
        let seq = self.log.append(&event)?;
        self.index.insert(ts, seq)?;
        sender.try_send(EventStored(event.into_sequenced(seq)))?;
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
