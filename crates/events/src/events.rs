// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};

use crate::{CorrelationId, EnclaveEvent, Sequenced, Unsequenced};

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
    pub id: CorrelationId,
    pub sender: Recipient<ReceiveEvents>,
    pub ts: u128,
}

impl GetEventsAfter {
    pub fn new(id: CorrelationId, sender: impl Into<Recipient<ReceiveEvents>>, ts: u128) -> Self {
        Self {
            id,
            sender: sender.into(),
            ts,
        }
    }
}

#[derive(Message, Debug)]
#[rtype("()")]
pub struct ReceiveEvents {
    id: CorrelationId,
    events: Vec<EnclaveEvent<Sequenced>>,
}

impl ReceiveEvents {
    pub fn new(id: CorrelationId, events: Vec<EnclaveEvent>) -> Self {
        Self { id, events }
    }

    pub fn events(&self) -> &Vec<EnclaveEvent> {
        &self.events
    }

    pub fn id(&self) -> CorrelationId {
        self.id
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
