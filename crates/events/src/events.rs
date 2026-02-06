// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};

use crate::{CorrelationId, EnclaveEvent, Sequenced, Unsequenced};

/// Direct event received by the EventStore to store an event
#[derive(Message, Debug)]
#[rtype("()")]
pub struct StoreEventRequested {
    pub event: EnclaveEvent<Unsequenced>,
    pub sender: Recipient<StoreEventResponse>,
}

impl StoreEventRequested {
    pub fn new(
        event: EnclaveEvent<Unsequenced>,
        sender: impl Into<Recipient<StoreEventResponse>>,
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
pub struct GetEventsAfterRequest {
    correlation_id: CorrelationId,
    ts: u128,
    sender: Recipient<GetEventsAfterResponse>,
}

impl GetEventsAfterRequest {
    pub fn new(
        correlation_id: CorrelationId,
        ts: u128,
        sender: impl Into<Recipient<GetEventsAfterResponse>>,
    ) -> Self {
        Self {
            correlation_id,
            ts,
            sender: sender.into(),
        }
    }

    pub fn ts(&self) -> u128 {
        self.ts
    }

    pub fn id(&self) -> CorrelationId {
        self.correlation_id
    }

    pub fn sender(&self) -> &Recipient<GetEventsAfterResponse> {
        &self.sender
    }
}

/// The response of a request to get all EventStore events by either sequence or timestamp
#[derive(Message, Debug)]
#[rtype("()")]
pub struct GetEventsAfterResponse {
    id: CorrelationId,
    events: Vec<EnclaveEvent<Sequenced>>,
}

impl GetEventsAfterResponse {
    pub fn new(id: CorrelationId, events: Vec<EnclaveEvent>) -> Self {
        Self { id, events }
    }

    pub fn into_events(self) -> Vec<EnclaveEvent> {
        self.events
    }

    pub fn id(&self) -> CorrelationId {
        self.id
    }
}

/// Direct event received by the Sequencer once an event has been stored
#[derive(Message, Debug)]
#[rtype("()")]
pub struct StoreEventResponse(pub EnclaveEvent<Sequenced>);

impl StoreEventResponse {
    pub fn into_event(self) -> EnclaveEvent<Sequenced> {
        self.0
    }
}
