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
use anyhow::Result;
use tracing::error;

pub struct EventStore<I: SequenceIndex, L: EventLog> {
    index: I,
    log: L,
}

impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
    /// Store an event in the persistent log, index it by its timestamp, and reply to the requester with the sequenced event.
    ///
    /// Attempts to append the provided event to the event log, insert the resulting sequence number into the sequence index keyed by the event timestamp, and send an `EventStored` message containing the sequenced event to the original sender. Propagates any error encountered during append, index insertion, or sending.
    ///
    /// # Parameters
    ///
    /// - `msg`: a `StoreEventRequested` containing the event to persist and the sender to reply to.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or the underlying error encountered while appending to the log, updating the index, or sending the response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // pseudo-code illustrating typical usage:
    /// // let mut store = EventStore::new(index, log);
    /// // let request = StoreEventRequested { event, sender };
    /// // store.handle_store_event_requested(request).unwrap();
    /// ```
    pub fn handle_store_event_requested(&mut self, msg: StoreEventRequested) -> Result<()> {
        let event = msg.event;
        let sender = msg.sender;
        let ts = event.get_ts();
        let seq = self.log.append(&event)?;
        self.index.insert(ts, seq)?;
        sender.try_send(EventStored(event.into_sequenced(seq)))?;
        Ok(())
    }

    /// Sends all events with sequence numbers at or after the sequence corresponding to `msg.ts` to the requesting actor.
    ///
    /// Looks up the starting sequence for `msg.ts` in the index (defaults to 1 if not found), reads events from the log
    /// starting at that sequence, converts them into sequenced events, and delivers them to `msg.sender`.
    ///
    /// # Returns
    ///
    /// `Ok(())` if events were read and the response was delivered; `Err` if the index lookup, log read, or send operation failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # // Example is illustrative and ignored in doctest
    /// # #[allow(unused_imports)]
    /// # use crate::events::{EventStore, GetEventsAfter, ReceiveEvents};
    /// # fn example() {
    /// // let mut store = EventStore::new(index, log);
    /// // let msg = GetEventsAfter::new(ts, requester);
    /// // store.handle_get_events_after(msg).unwrap();
    /// # }
    /// ```
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
    /// Creates a new EventStore using the provided sequence index and event log.
    ///
    /// # Examples
    ///
    /// ```
    /// // `index` implements `SequenceIndex` and `log` implements `EventLog`.
    /// let index = Default::default();
    /// let log = Default::default();
    /// let store = EventStore::new(index, log);
    /// ```
    pub fn new(index: I, log: L) -> Self {
        Self { index, log }
    }
}

impl<I: SequenceIndex, L: EventLog> Actor for EventStore<I, L> {
    type Context = actix::Context<Self>;
}

impl<I: SequenceIndex, L: EventLog> Handler<StoreEventRequested> for EventStore<I, L> {
    type Result = ();
    /// Handles an incoming `StoreEventRequested` message for this actor.
    ///
    /// Delegates processing to the internal store handler and logs any error produced during handling.
    /// The method does not propagate errors to the caller; failures are recorded via tracing.
    ///
    /// # Parameters
    ///
    /// - `msg`: the `StoreEventRequested` message containing the event to persist and the reply channel.
    ///
    /// # Returns
    ///
    /// This handler does not return a value.
    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        match self.handle_store_event_requested(msg) {
            Ok(_) => (),
            Err(e) => error!("{e}"),
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<GetEventsAfter> for EventStore<I, L> {
    type Result = ();
    /// Delegates a `GetEventsAfter` message to `handle_get_events_after` and logs any error.
    ///
    /// This handler forwards the incoming message to the internal processing method and suppresses
    /// errors by logging them instead of propagating them to the Actix runtime.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crates::events::{EventStore, GetEventsAfter};
    /// // Given an EventStore `store` and a `ctx` obtained from Actix, calling the handler:
    /// // store.handle(GetEventsAfter { /* ... */ }, &mut ctx);
    /// ```
    fn handle(&mut self, msg: GetEventsAfter, _: &mut Self::Context) -> Self::Result {
        match self.handle_get_events_after(msg) {
            Ok(_) => (),
            Err(e) => error!("{e}"),
        }
    }
}