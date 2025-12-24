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
    pub fn new(seq: u64) -> Self {
        Self(seq)
    }

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
    pub fn new(events: Vec<EnclaveEvent>) -> Self {
        Self(events)
    }
    pub fn events(&self) -> &Vec<EnclaveEvent> {
        &self.0
    }
}

/// Direct event received by the Sequencer once an event has been stored
#[derive(Message, Debug)]
#[rtype("()")]
pub struct EventStored(pub EnclaveEvent<Sequenced>);

impl EventStored {
    pub fn into_event(self) -> EnclaveEvent<Sequenced> {
        self.0
    }
}
