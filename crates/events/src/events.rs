// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};

use crate::{EnclaveEvent, Sequenced, Unsequenced};

/// Direct event received by the snapshot buffer in order to save snapshot to disk
#[derive(Message, Debug)]
#[rtype("()")]
pub struct CommitSnapshot(u64);

impl CommitSnapshot {
    /// Creates a CommitSnapshot containing the specified sequence number.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = CommitSnapshot::new(42);
    /// assert_eq!(msg.seq(), 42);
    /// ```
    pub fn new(seq: u64) -> Self {
        Self(seq)
    }

    /// Retrieve the stored sequence number.
    ///
    /// # Returns
    ///
    /// The `u64` sequence number contained in the message.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = CommitSnapshot::new(42);
    /// assert_eq!(msg.seq(), 42);
    /// ```
    pub fn seq(&self) -> u64 {
        self.0
    }
}

/// Direct event received by the EventStore to store an event
#[derive(Message, Debug)]
#[rtype("()")]
pub struct StoreEventRequested {
    pub event: EnclaveEvent<Unsequenced>,
    pub sender: Recipient<EventStored>,
}

impl StoreEventRequested {
    /// Constructs a `StoreEventRequested` pairing an unsequenced event with a recipient that will receive an `EventStored` response.
    ///
    /// The provided `sender` is converted into a `Recipient<EventStored>` using `Into`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use actix::Recipient;
    /// // `event` should be an `EnclaveEvent<Unsequenced>`
    /// // `recipient` should be a `Recipient<EventStored>` obtained from an actor or converted from an compatible type.
    /// let event = /* build or obtain an EnclaveEvent<Unsequenced> */ unimplemented!();
    /// let recipient: Recipient<EventStored> = /* obtain recipient */ unimplemented!();
    /// let req = StoreEventRequested::new(event, recipient);
    /// ```
    pub fn new(
        event: EnclaveEvent<Unsequenced>,
        sender: impl Into<Recipient<EventStored>>,
    ) -> Self {
        Self {
            event,
            sender: sender.into(),
        }
    }
}

/// Get events after timestamp in EventStore
#[derive(Message, Debug)]
#[rtype("()")]
pub struct GetEventsAfter {
    pub ts: u128,
    pub sender: Recipient<ReceiveEvents>,
}

impl GetEventsAfter {
    /// Creates a `GetEventsAfter` message for requesting events that occurred after `ts`.
    ///
    /// The `sender` is converted into a `Recipient<ReceiveEvents>` and stored in the message.
    ///
    /// # Examples
    ///
    /// ```
    /// // assume `recipient` implements `Into<actix::Recipient<ReceiveEvents>>`
    /// let msg = GetEventsAfter::new(1_700_000_000_000_000_000u128, recipient);
    /// ```
    pub fn new(ts: u128, sender: impl Into<Recipient<ReceiveEvents>>) -> Self {
        Self {
            ts,
            sender: sender.into(),
        }
    }
}

#[derive(Message, Debug)]
#[rtype("()")]
pub struct ReceiveEvents(Vec<EnclaveEvent<Sequenced>>);

impl ReceiveEvents {
    /// Creates a new `ReceiveEvents` wrapping the provided sequenced events.
    ///
    /// # Examples
    ///
    /// ```
    /// let msg = ReceiveEvents::new(Vec::new());
    /// assert!(msg.events().is_empty());
    /// ```
    pub fn new(events: Vec<EnclaveEvent>) -> Self {
        Self(events)
    }
    /// Borrows the stored list of sequenced events.
    ///
    /// Returns a reference to the internal `Vec<EnclaveEvent<Sequenced>>`.
    ///
    /// # Examples
    ///
    /// ```
    /// let evts = ReceiveEvents::new(Vec::new());
    /// let slice: &Vec<EnclaveEvent<Sequenced>> = evts.events();
    /// assert!(slice.is_empty());
    /// ```
    pub fn events(&self) -> &Vec<EnclaveEvent> {
        &self.0
    }
}

/// Direct event received by the Sequencer once an event has been stored
#[derive(Message, Debug)]
#[rtype("()")]
pub struct EventStored(pub EnclaveEvent<Sequenced>);

impl EventStored {
    /// Extracts the contained sequenced `EnclaveEvent`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given a tuple struct `EventStored(pub EnclaveEvent<Sequenced>)`
    /// // let stored = EventStored(enclave_event);
    /// // let event = stored.into_event();
    /// ```
    pub fn into_event(self) -> EnclaveEvent<Sequenced> {
        self.0
    }
}