use actix::{Message, Recipient};

use crate::{EnclaveEvent, Sequenced, Unsequenced};

/// Direct event received by the snapshot buffer in order to save snapshot to disk
#[derive(Message)]
#[rtype("()")]
pub struct CommitSnapshot(pub u64);

/// Direct event received by the EventStore to store an event
#[derive(Message)]
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

/// Direct event received by the Sequencer once an event has been stored
#[derive(Message)]
#[rtype("()")]
pub struct EventStored(pub EnclaveEvent<Sequenced>);

impl EventStored {
    pub fn into_event(self) -> EnclaveEvent<Sequenced> {
        self.0
    }
}
