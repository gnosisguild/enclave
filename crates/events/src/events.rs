// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;

use actix::{Message, Recipient};

use crate::{AggregateId, CorrelationId, EnclaveEvent, Sequenced, Unsequenced};

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

/// The response of a request to get all EventStore events by either sequence or timestamp
#[derive(Message, Debug)]
#[rtype("()")]
pub struct EventStoreQueryResponse {
    id: CorrelationId,
    events: Vec<EnclaveEvent<Sequenced>>,
}

impl EventStoreQueryResponse {
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

/// Trait for various EventStore query types
pub trait QueryKind {
    type Shape;
}

/// Query by aggregated sequence
pub struct SeqAgg;
impl QueryKind for SeqAgg {
    type Shape = HashMap<AggregateId, u64>;
}

/// Query by aggregated timestamp
pub struct TsAgg;
impl QueryKind for TsAgg {
    type Shape = HashMap<AggregateId, u128>;
}

/// Query by timestamp
pub struct Ts;
impl QueryKind for Ts {
    type Shape = u128;
}

/// Query by seq
pub struct Seq;
impl QueryKind for Seq {
    type Shape = u64;
}

#[derive(Message, Debug)]
#[rtype("()")]
pub struct EventStoreQueryBy<Q: QueryKind> {
    correlation_id: CorrelationId,
    query: Q::Shape,
    sender: Recipient<EventStoreQueryResponse>,
}

impl EventStoreQueryBy<SeqAgg> {
    pub fn new(
        correlation_id: CorrelationId,
        query: HashMap<AggregateId, u64>,
        sender: impl Into<Recipient<EventStoreQueryResponse>>,
    ) -> Self {
        Self {
            correlation_id,
            query,
            sender: sender.into(),
        }
    }

    pub fn query(&self) -> &HashMap<AggregateId, u64> {
        &self.query
    }
}

impl EventStoreQueryBy<TsAgg> {
    pub fn new(
        correlation_id: CorrelationId,
        query: HashMap<AggregateId, u128>,
        sender: impl Into<Recipient<EventStoreQueryResponse>>,
    ) -> Self {
        Self {
            correlation_id,
            query,
            sender: sender.into(),
        }
    }

    pub fn query(&self) -> &HashMap<AggregateId, u128> {
        &self.query
    }
}

impl EventStoreQueryBy<Ts> {
    pub fn new(
        correlation_id: CorrelationId,
        query: u128,
        sender: impl Into<Recipient<EventStoreQueryResponse>>,
    ) -> Self {
        Self {
            correlation_id,
            query,
            sender: sender.into(),
        }
    }

    pub fn query(&self) -> u128 {
        self.query
    }
}

impl EventStoreQueryBy<Seq> {
    pub fn new(
        correlation_id: CorrelationId,
        query: u64,
        sender: impl Into<Recipient<EventStoreQueryResponse>>,
    ) -> Self {
        Self {
            correlation_id,
            query,
            sender: sender.into(),
        }
    }

    pub fn query(&self) -> u64 {
        self.query
    }
}

impl<Q: QueryKind> EventStoreQueryBy<Q> {
    pub fn id(&self) -> CorrelationId {
        self.correlation_id
    }

    pub fn sender(self) -> Recipient<EventStoreQueryResponse> {
        self.sender
    }
}
